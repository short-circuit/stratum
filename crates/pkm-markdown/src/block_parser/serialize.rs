//! Serialization of block trees back to markdown body text and full documents.

use super::parse_document;
use pkm_block::Block;

pub fn serialize_blocks(blocks: &[Block]) -> String {
    use pkm_block::BlockTree;

    // Build a temporary tree to get ordering
    let mut tree = BlockTree::new();
    for block in blocks {
        tree.insert(block.clone());
    }

    let ordered = tree.into_sorted_vec();
    serialize_ordered_blocks(&ordered, 0)
}

/// Assemble a full `.md` document from a serialized body, preserving any existing
/// YAML frontmatter found in `existing`.
///
/// When `existing` begins with a `---` frontmatter block, all of its fields
/// (title/created/modified/tags/aliases/extra) are re-serialized and prepended to
/// `body`; if `title` is provided it overwrites the preserved title. Otherwise the
/// output follows the legacy behaviour: title-only frontmatter when a `title` is
/// given, or the bare `body` when it is not.
///
/// Block-editor saves are issued without a `title`, so rebuilding a file from
/// `title` alone would strip every other frontmatter field from disk — and the file
/// watcher would then sync that stripped file back into SQLite, erasing the metadata.
pub fn assemble_blocks_markdown(existing: &str, body: &str, title: Option<&str>) -> String {
    if existing.trim_start().starts_with("---") {
        let (mut fm, _, _) = parse_document(existing);
        if let Some(t) = title {
            fm.title = Some(t.to_string());
        }
        let yaml_str = serde_yaml::to_string(&fm).unwrap_or_default();
        format!("---\n{}---\n\n{}", yaml_str, body)
    } else if let Some(t) = title {
        format!("---\ntitle: {}\n---\n\n{}", t, body)
    } else {
        body.to_string()
    }
}

fn serialize_ordered_blocks(blocks: &[Block], depth: usize) -> String {
    let mut output = String::new();

    for block in blocks {
        let indent = "  ".repeat(depth);
        output.push_str(&format!(
            "{}- {}\n",
            indent,
            block.content.replace('\n', &format!("\n{}  ", indent))
        ));

        // Write properties
        if let Some(ref marker) = block.marker {
            output.push_str(&format!("{}  .marker: {}\n", indent, marker.as_str()));
        }
        if let Some(ref priority) = block.priority {
            output.push_str(&format!("{}  .priority: {}\n", indent, priority.as_str()));
        }
        if let Some(hl) = block.meta.heading_level {
            output.push_str(&format!("{}  .heading-level: {}\n", indent, hl));
        }
        for (key, value) in &block.properties {
            output.push_str(&format!("{}  .{}: {}\n", indent, key, value));
        }
        // Always write id last
        output.push_str(&format!("{}  .id: {}\n", indent, block.id));
    }

    output
}
