//! Obsidian vault → Stratum markdown conversion.
//!
//! Reads an Obsidian note, applies the shared inline rewrites (embeds and
//! wiki-link slug resolution), and converts Obsidian-specific syntactic
//! conventions — task checkboxes and callouts — into Stratum's canonical
//! raw-block form. Obsidian stores tags in frontmatter, so those are preserved
//! verbatim and never duplicated from body content.

use crate::convert::{
    assets_rel_prefix, parse_callout_opener, rewrite_inline, strip_bullet, strip_task_marker,
    Converted,
};
use pkm_core::{Frontmatter, PkmResult};
use std::path::Path;

/// Convert a single Obsidian note's source content into Stratum form.
///
/// `source_root` is the source vault directory (used to check whether a
/// referenced asset exists before rewriting an embed) and `subdir` is the
/// destination subdirectory used to build the asset prefix.
pub fn convert(source_content: &str, source_root: &Path, subdir: &str) -> PkmResult<Converted> {
    let (frontmatter, body) = pkm_markdown::parser::parse_frontmatter(source_content);
    let prefix = assets_rel_prefix(subdir);
    let body = rewrite_inline(&body, &prefix, source_root, true);
    let transformed = transform_obsidian(&body);
    Ok(Converted {
        frontmatter: with_default_title(frontmatter),
        body: transformed,
    })
}

/// Assert the frontmatter has a title, since Obsidian notes usually carry one in
/// YAML; if absent, the body's first heading is not used here (the page slug is
/// the safe fallback and is filled in by the importer).
fn with_default_title(fm: Frontmatter) -> Frontmatter {
    fm
}

/// Line-level Obsidian structural conversion into Stratum raw-block syntax:
/// callouts become blocks with a `.callout` property, and `- [ ]` / `- [x]`
/// task checkboxes become blocks with a `.marker` property. All other lines
/// (headings, plain bullets, paragraphs) are preserved verbatim so the upstream
/// block parser can assign hierarchy.
fn transform_obsidian(body: &str) -> String {
    let lines: Vec<&str> = body.lines().collect();
    let mut out: Vec<String> = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];

        // Callout opener: `> [!type] Title` plus any following `> ` continuation
        // lines become a single block with content and a `.callout` property.
        if let Some((callout_type, title)) = parse_callout_opener(line) {
            let mut continuation: Vec<String> = Vec::new();
            i += 1;
            while i < lines.len() {
                let lt = lines[i].trim_start();
                if let Some(rest) = lt.strip_prefix("> ") {
                    continuation.push(rest.to_string());
                    i += 1;
                } else if let Some(rest) = lt.strip_prefix('>') {
                    continuation.push(rest.to_string());
                    i += 1;
                } else {
                    break;
                }
            }
            out.push(format!("- {title}"));
            for c in &continuation {
                out.push(format!("  {c}"));
            }
            out.push(format!("  .callout: {callout_type}"));
            continue;
        }

        // Task checkbox or ordinary bullet.
        if let Some(bullet_content) = strip_bullet(line.trim_start()) {
            if let Some((remaining, marker)) = strip_task_marker(&bullet_content) {
                out.push(format!("- {remaining}"));
                out.push(format!("  .marker: {marker}"));
            } else {
                out.push(format!("- {bullet_content}"));
            }
            i += 1;
            continue;
        }

        // Heading, blank line, paragraph, or inline content — leave untouched.
        out.push(line.to_string());
        i += 1;
    }
    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn converts_task_markers_and_callouts() {
        let body = "# Heading\n\n- [ ] todo item\n- [x] done item\n\n> [!tip] Tip\n> body";
        let out = transform_obsidian(body);
        assert!(out.contains("- todo item\n  .marker: TODO"));
        assert!(out.contains("- done item\n  .marker: DONE"));
        assert!(out.contains("- Tip\n  body\n  .callout: tip"));
        assert!(out.contains("# Heading"));
    }

    #[test]
    fn convert_rewrites_embed_and_link() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("assets")).unwrap();
        std::fs::write(dir.path().join("assets").join("pic.png"), "x").unwrap();
        let content =
            "---\ntitle: Welcome\ntags: [meta]\n---\nSee [[folder/Note]] ![[assets/pic.png]].";
        let c = convert(content, dir.path(), "imported/obsidian-sample").unwrap();
        assert!(c.body.contains("[[Note]]"));
        assert!(c
            .body
            .contains("![[imported/obsidian-sample/assets/pic.png]]"));
        assert_eq!(c.frontmatter.title.as_deref(), Some("Welcome"));
    }
}
