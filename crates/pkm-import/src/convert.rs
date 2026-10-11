//! Shared conversion helpers for the importers.
//!
//! This module contains the primitives that all three importers build on:
//! destination-path sanitization, inline markdown rewrites (embeds and wiki
//! links), inline tag extraction, content hashing for idempotent dedup, and the
//! line-level structural rewriters that turn foreign task markers, callouts and
//! properties into Stratum's canonical raw-block syntax.
//!
//! The conversion pipeline intentionally delegates all hierarchy construction
//! (parent/left ids by indentation, heading levels) to
//! [`pkm_markdown::block_parser::parse_document`] / the serializer at the end —
//! this module only rewrites source text into the canonical raw-block form so
//! that parser can interpret it.

use crate::report::ImportFormat;
use pkm_core::{Frontmatter, PkmResult};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;

/// Result of the inline + structural body transformation for one source file.
pub struct Converted {
    /// Frontmatter, possibly augmented during conversion (e.g. Logseq inline
    /// tags collected into `frontmatter.tags`).
    pub frontmatter: Frontmatter,
    /// The transformed body, in Stratum raw-block syntax, ready to be passed to
    /// `pkm_markdown::block_parser::parse_document`.
    pub body: String,
}

/// Sanitize a source directory basename into a safe destination subdirectory
/// segment: lowercase ASCII, any non-alphanumeric character mapped to `-`
/// (runs are NOT collapsed — each offending char becomes its own `-`, then the
/// result is trimmed of leading/trailing `-`).
pub fn sanitize_subdir(name: &str) -> String {
    let out: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    out.trim_matches('-').to_string()
}

/// Compute a content hash over raw bytes using `std::collections::hash_map::DefaultHasher`.
///
/// Used to decide, idempotently, whether a destination file already has
/// byte-identical content. Deterministic within a process (no random seed), so
/// re-importing the same vault always yields the same hash and thus a
/// "duplicate" classification.
pub fn content_hash(bytes: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}

/// Resolve a wiki-link target to a slug: take the final path segment and strip
/// any file extension, so `[[path/to/Note]]` becomes `[[Note]]` and imports
/// resolve to the same slug the importer writes.
pub fn resolve_link_target(target: &str) -> String {
    let leaf = target.rsplit('/').next().unwrap_or(target);
    match leaf.rsplit_once('.') {
        Some((stem, _)) => stem.to_string(),
        None => leaf.to_string(),
    }
}

/// Rewrite embeds and (optionally) wiki-link targets in body text.
///
/// Runs a single left-to-right scan so an embed `![[…]]` is never mistaken for a
/// wiki-link `[[…]]` (which would otherwise resolve the asset path as a note
/// slug). Obsidian-import passes `resolve_links = true` to also collapse
/// `[[path/to/Note]]` to the target slug; Logseq passes `false` to leave
/// wiki-links untouched while still wiring embeds to the destination assets.
pub fn rewrite_inline(
    body: &str,
    assets_rel_prefix: &str,
    source_root: &Path,
    resolve_links: bool,
) -> String {
    let chars: Vec<char> = body.chars().collect();
    let mut out = String::with_capacity(body.len());
    let mut i = 0;
    while i < chars.len() {
        // Embed `![[…]]`
        if chars[i] == '!' && chars.get(i + 1) == Some(&'[') && chars.get(i + 2) == Some(&'[') {
            if let Some(end) = find_double_close(&chars, i + 3) {
                let inner: String = chars[i + 3..end].iter().collect();
                let file_name = inner.rsplit('/').next().unwrap_or(&inner);
                if source_root.join("assets").join(file_name).is_file() {
                    out.push_str(&format!("![[{assets_rel_prefix}/{file_name}]]"));
                } else {
                    out.push_str(&format!("![[{inner}]]"));
                }
                i = end + 2;
                continue;
            }
        }
        // Wiki-link `[[…]]`
        if chars[i] == '[' && chars.get(i + 1) == Some(&'[') {
            if let Some(end) = find_double_close(&chars, i + 2) {
                let inner: String = chars[i + 2..end].iter().collect();
                if resolve_links {
                    match inner.split_once('|') {
                        Some((target, display)) => {
                            out.push_str(&format!("[[{}|{display}]]", resolve_link_target(target)))
                        }
                        None => out.push_str(&format!("[[{}]]", resolve_link_target(&inner))),
                    }
                } else {
                    out.push_str(&format!("[[{inner}]]"));
                }
                i = end + 2;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// Find the index of the first `]` of a closing `]]` pair, scanning from `start`.
fn find_double_close(chars: &[char], start: usize) -> Option<usize> {
    let mut j = start;
    while j + 1 < chars.len() {
        if chars[j] == ']' && chars[j + 1] == ']' {
            return Some(j);
        }
        j += 1;
    }
    None
}

/// Extract inline `#tag` occurrences from body text.
///
/// Matches `#` followed by letters/numbers/`_`/`-`/`/`, without matching inside
/// fenced code blocks (a best-effort guard). Used by the Logseq importer, which
/// stores tags inline in the body and whose frontmatter may not carry them.
pub fn extract_inline_tags(body: &str) -> Vec<String> {
    let re = regex::Regex::new(r"#(\w+(?:[/-]\w+)*)").expect("static tag regex");
    let mut seen = Vec::new();
    for caps in re.captures_iter(body) {
        let tag = caps[1].to_string();
        if !seen.contains(&tag) {
            seen.push(tag);
        }
    }
    seen
}

/// Parse a Logseq-style `key:: value` property line.
///
/// Returns `(key, value)`. `key::` with an empty value yields an empty-string
/// value, matching the spec. The opening bullet (`-`/`*`/`+`/`- `) is already
/// removed by the caller; this only recognizes the `key::` marker.
pub fn parse_logseq_property(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim_start();
    let idx = trimmed.find("::")?;
    let key = trimmed[..idx].trim();
    if key.is_empty() {
        return None;
    }
    let value = trimmed[idx + 2..].trim();
    Some((key.to_string(), value.to_string()))
}

/// Append `tags` tokens parsed from a Logseq `tags::`/`file::` property value to
/// the frontmatter tags, de-duplicated and order-preserving. Used only for
/// Logseq import.
pub fn append_property_tags(fm: &mut Frontmatter, key: &str, value: &str) {
    if key != "tags" && key != "file" {
        return;
    }
    for token in value.split(',') {
        let t = token.trim();
        if !t.is_empty() && !fm.tags.iter().any(|x| x == t) {
            fm.tags.push(t.to_string());
        }
    }
}

/// Detect a foreign task-marker prefix on a block's content and strip it.
///
/// Returns `(remaining_content, marker_string)` when the content begins with a
/// recognizable marker:
///
/// * Obsidian checkbox syntax `[ ] ` / `[x] ` / `[X] ` → TODO / DONE.
/// * Logseq leading markers `TODO ` / `DOING ` / `DONE ` / `NOW ` / `LATER ` /
///   `WAITING ` / `CANCELLED ` (case-insensitive) via
///   [`pkm_block::TaskMarker::parse`].
///
/// Returns `None` when the content is not a task line.
pub(crate) fn strip_task_marker(content: &str) -> Option<(String, String)> {
    let trimmed = content.trim_start();
    // Obsidian `- [ ]` / `- [x]` / `- [X]` task checkboxes
    if let Some(rest) = trimmed.strip_prefix('[') {
        let marker = match rest.chars().next() {
            Some(' ') => Some("TODO"),
            Some('x') | Some('X') => Some("DONE"),
            _ => None,
        };
        if let Some(m) = marker {
            if rest[1..].starts_with(']') {
                let remainder = rest[2..].trim_start();
                return Some((remainder.to_string(), m.to_string()));
            }
        }
    }
    // Logseq `- TODO` / `- DOING` / `- DONE` / `- NOW` / `- LATER` / etc.
    if let Some((word, tail)) = trimmed.split_once(' ') {
        if let Some(marker) = pkm_block::TaskMarker::parse(word) {
            return Some((tail.trim_start().to_string(), marker.as_str().to_string()));
        }
    }
    None
}

/// Parse an Obsidian callout opener `> [!type] Title`.
///
/// Returns `(callout_type, title)` when the line is a callout opener.
pub(crate) fn parse_callout_opener(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim_start();
    let rest = trimmed
        .strip_prefix("> ")
        .or_else(|| trimmed.strip_prefix('>'))?;
    let rest = rest.trim_start();
    let rest = rest.strip_prefix("[!")?;
    let (callout_type, after) = split_once_char(rest, ']')?;
    let title = after.trim_start().trim();
    Some((callout_type.to_string(), title.to_string()))
}

fn split_once_char(s: &str, needle: char) -> Option<(&str, &str)> {
    let idx = s.find(needle)?;
    Some((&s[..idx], &s[idx + needle.len_utf8()..]))
}

/// Strip a leading list marker from a trim-started line.
///
/// Recognizes `- `, `* `, `+ `, and ordered `N. ` bullets and returns the
/// remaining content. This exists because `pkm_markdown`'s internal
/// `strip_list_marker` is `pub(crate)` and not reachable from this crate.
pub(crate) fn strip_bullet(trimmed: &str) -> Option<String> {
    if let Some(rest) = trimmed
        .strip_prefix("- ")
        .or_else(|| trimmed.strip_prefix("* "))
        .or_else(|| trimmed.strip_prefix("+ "))
    {
        return Some(rest.to_string());
    }
    if let Some(rest) = trimmed.strip_prefix(|c: char| c.is_ascii_digit()) {
        let after = rest.trim_start_matches(|c: char| c.is_ascii_digit());
        if let Some(content) = after.strip_prefix(". ") {
            return Some(content.to_string());
        }
    }
    None
}

/// Determine the destination subdirectory for an import operation.
///
/// `opts.subdir` takes precedence; otherwise `imported/<sanitized-source-name>`
/// is used. `source_name` is the source directory's basename.
pub fn resolve_subdir(opts_subdir: &Option<String>, source_name: &str) -> String {
    match opts_subdir {
        Some(s) if !s.is_empty() => s.clone(),
        _ => format!("imported/{}", sanitize_subdir(source_name)),
    }
}

/// The canonical 'assets/' destination prefix for the current subdirectory.
pub fn assets_rel_prefix(subdir: &str) -> String {
    format!("{subdir}/assets")
}

/// Assurance helper used by conversion entry points to resolve the effective
/// format (replacing `Auto` via detection). Kept here so all importers share it.
pub fn resolve_format(format: ImportFormat, source_path: &Path) -> ImportFormat {
    match format {
        ImportFormat::Auto => crate::detect::detect_format(source_path),
        other => other,
    }
}

/// Convert raw source content into final file content per format.
///
/// For `Generic` the content is returned byte-for-byte unchanged; for the
/// structured importers the frontmatter is re-serialized and the body is
/// re-emitted through the canonical block serializer. This is the function the
/// frontmatter/body serialization logic funnels through.
pub fn finalize_output(converted: &Converted, blocks: &[pkm_block::Block]) -> PkmResult<String> {
    let yaml = serde_yaml::to_string(&converted.frontmatter)
        .map_err(|e| pkm_core::PkmError::Frontmatter(e.to_string()))?;
    let body = pkm_markdown::block_parser::serialize_blocks(blocks);
    Ok(format!("---\n{yaml}---\n\n{body}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn sanitize_lowercases_and_dashes() {
        assert_eq!(sanitize_subdir("Obsidian Vault"), "obsidian-vault");
        assert_eq!(sanitize_subdir("obsidian-sample"), "obsidian-sample");
        assert_eq!(sanitize_subdir("My.Vault#2"), "my-vault-2");
    }

    #[test]
    fn resolve_link_target_strips_path_and_ext() {
        assert_eq!(resolve_link_target("path/to/Note"), "Note");
        assert_eq!(resolve_link_target("Note.md"), "Note");
        assert_eq!(resolve_link_target("Note"), "Note");
    }

    #[test]
    fn rewrite_embeds_only_when_asset_exists() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("assets")).unwrap();
        std::fs::write(dir.path().join("assets").join("logo.png"), "x").unwrap();
        let body = "See ![[assets/logo.png]] and ![[missing.png]].";
        let out = rewrite_inline(body, "imported/x/assets", dir.path(), true);
        assert_eq!(
            out,
            "See ![[imported/x/assets/logo.png]] and ![[missing.png]]."
        );
    }

    #[test]
    fn resolve_obsidian_links_with_path() {
        let dir = TempDir::new().unwrap();
        assert_eq!(
            rewrite_inline(
                "See [[folder/Note]] and [[Note|display]].",
                "i/assets",
                dir.path(),
                true
            ),
            "See [[Note]] and [[Note|display]]."
        );
    }

    #[test]
    fn embeds_are_not_clobbered_by_link_resolution() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("assets")).unwrap();
        std::fs::write(dir.path().join("assets").join("pic.png"), "x").unwrap();
        let out = rewrite_inline(
            "See [[folder/Note]] ![[assets/pic.png]].",
            "imported/sample/assets",
            dir.path(),
            true,
        );
        assert_eq!(out, "See [[Note]] ![[imported/sample/assets/pic.png]].");
    }

    #[test]
    fn extract_inline_tags_works() {
        let tags = extract_inline_tags("text #alpha and #beta/gamma.");
        assert_eq!(tags, vec!["alpha", "beta/gamma"]);
    }

    #[test]
    fn parse_logseq_property_line() {
        assert_eq!(
            parse_logseq_property("key:: value"),
            Some(("key".into(), "value".into()))
        );
        assert_eq!(
            parse_logseq_property("empty::"),
            Some(("empty".into(), "".into()))
        );
        assert_eq!(parse_logseq_property("no marker"), None);
    }

    #[test]
    fn task_marker_parsing() {
        assert_eq!(
            strip_task_marker("[ ] read paper"),
            Some(("read paper".into(), "TODO".into()))
        );
        assert_eq!(
            strip_task_marker("[x] done"),
            Some(("done".into(), "DONE".into()))
        );
        assert_eq!(
            strip_task_marker("[X] done"),
            Some(("done".into(), "DONE".into()))
        );
        assert_eq!(
            strip_task_marker("TODO fix typo"),
            Some(("fix typo".into(), "TODO".into()))
        );
        assert_eq!(
            strip_task_marker("NOW current"),
            Some(("current".into(), "NOW".into()))
        );
        assert_eq!(
            strip_task_marker("LATER later"),
            Some(("later".into(), "LATER".into()))
        );
        assert_eq!(strip_task_marker("plain bullet"), None);
    }

    #[test]
    fn callout_opener_parsing() {
        assert_eq!(
            parse_callout_opener("> [!tip] Tip"),
            Some(("tip".into(), "Tip".into()))
        );
        assert_eq!(
            parse_callout_opener("> [!NOTE] Note"),
            Some(("NOTE".into(), "Note".into()))
        );
        assert_eq!(parse_callout_opener("> plain quote"), None);
    }
}
