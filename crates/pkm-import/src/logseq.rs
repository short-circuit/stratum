//! Logseq vault → Stratum markdown conversion.
//!
//! Handles Logseq-specific conventions the generic/Obsidian paths do not: task
//! prefixes (`- TODO`/`- NOW`/…), `key:: value` properties indented under a
//! block, and inline `#tag`/`tags::` collection into frontmatter (Logseq stores
//! tags inline, not in YAML). Block references `((uuid))` are preserved as-is.

use crate::convert::{
    append_property_tags, assets_rel_prefix, extract_inline_tags, parse_logseq_property,
    rewrite_inline, strip_bullet, strip_task_marker, Converted,
};
use pkm_core::{Frontmatter, PkmResult};
use std::path::Path;

/// Convert a single Logseq note's source content into Stratum form.
///
/// Inline `#tag` occurrences and `tags::`/`file::` property values are folded
/// into `frontmatter.tags`; embeds are rewritten to the destination assets
/// prefix; task prefixes and properties become `.marker`/`.prop` lines.
pub fn convert(source_content: &str, source_root: &Path, subdir: &str) -> PkmResult<Converted> {
    let (mut frontmatter, body) = pkm_markdown::parser::parse_frontmatter(source_content);
    let prefix = assets_rel_prefix(subdir);
    let body = rewrite_inline(&body, &prefix, source_root, false);

    // Logseq stores tags inline in the body, not in YAML — collect them into
    // frontmatter so the Stratum tagger indexes them like any other frontmatter.
    for tag in extract_inline_tags(&body) {
        if !frontmatter.tags.iter().any(|t| t == &tag) {
            frontmatter.tags.push(tag);
        }
    }

    let transformed = transform_logseq(&body, &mut frontmatter);
    Ok(Converted {
        frontmatter,
        body: transformed,
    })
}

/// Line-level Logseq structural conversion into Stratum raw-block syntax.
///
/// Bullets carrying a Logseq task marker become `.marker:` blocks; `key:: value`
/// property lines become `.key: value` property lines (folding tags/file values
/// into frontmatter); everything else is passed through verbatim.
fn transform_logseq(body: &str, fm: &mut Frontmatter) -> String {
    let lines: Vec<&str> = body.lines().collect();
    let mut out: Vec<String> = Vec::new();

    for line in lines {
        let trimmed = line.trim_start();

        // Bullet first (a `- key:: value` line is a bullet, not a property).
        if let Some(bullet_content) = strip_bullet(trimmed) {
            if let Some((remaining, marker)) = strip_task_marker(&bullet_content) {
                out.push(format!("- {remaining}"));
                out.push(format!("  .marker: {marker}"));
            } else {
                out.push(format!("- {bullet_content}"));
            }
            continue;
        }

        // `key:: value` property line, indented under a block.
        if let Some((key, value)) = parse_logseq_property(trimmed) {
            if key == "tags" || key == "file" {
                append_property_tags(fm, &key, &value);
            }
            out.push(format!("  .{key}: {value}"));
            continue;
        }

        out.push(line.to_string());
    }
    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn converts_markers_properties_and_block_refs() {
        let content = "- [[Projects]]\n- TODO fix typo\n  key:: value\n  other-key:: b\n- ((00000000-0000-0000-0000-000000000001))\n- DONE closed\n";
        let dir = TempDir::new().unwrap();
        let c = convert(content, dir.path(), "imported/logseq-sample").unwrap();
        assert!(c.body.contains("- fix typo\n  .marker: TODO"));
        assert!(c.body.contains("  .key: value"));
        assert!(c.body.contains("  .other-key: b"));
        assert!(c
            .body
            .contains("- ((00000000-0000-0000-0000-000000000001))"));
        assert!(c.body.contains("- closed\n  .marker: DONE"));
    }

    #[test]
    fn collects_inline_and_property_tags_into_frontmatter() {
        let dir = TempDir::new().unwrap();
        let content =
            "---\ntitle: Journal\n---\n- entry one\n- entry two\n  tags:: alpha\n#inline\n";
        let c = convert(content, dir.path(), "imported/logseq-sample").unwrap();
        assert!(c.frontmatter.tags.contains(&"alpha".to_string()));
        assert!(c.frontmatter.tags.contains(&"inline".to_string()));
        assert!(c.body.contains(".tags: alpha"));
    }
}
