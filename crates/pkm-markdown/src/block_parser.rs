//! Block-aware markdown parser.
//!
//! Parses markdown body content into a tree of blocks using indentation to
//! determine hierarchy. Supports Logseq-style properties with `.` prefix,
//! task markers, and block references.

use pkm_block::{Block, Priority, TaskMarker};
use std::collections::BTreeMap;
use std::str::FromStr;
use uuid::Uuid;

/// A raw block parsed from lines before final struct construction.
#[derive(Debug, Clone)]
pub(crate) struct RawBlock {
    indent: usize,
    content_lines: Vec<String>,
    properties: BTreeMap<String, String>,
    marker: Option<String>,
    priority: Option<String>,
    block_id: Option<Uuid>,
}

/// Count leading spaces for a line.
fn count_indent(line: &str) -> usize {
    line.chars().take_while(|c| *c == ' ').count()
}

/// Check if a line starts a new block (begins with `- ` after indentation).
fn is_block_line(line: &str) -> bool {
    let trimmed_start = line.chars().skip_while(|c| *c == ' ').collect::<String>();
    trimmed_start.starts_with("- ")
}

/// Get the block content from a line (everything after `- `, after indentation).
fn block_content(line: &str) -> &str {
    let indent = count_indent(line);
    &line[indent + 2..] // skip indent + "- "
}

/// Parse a property line: returns `(key, value)` if the line looks like `.key: value`.
fn parse_property(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('.') {
        return None;
    }
    if let Some(colon_pos) = trimmed[1..].find(':') {
        let key = &trimmed[1..1 + colon_pos]; // after '.', before ':'
        let value = trimmed[1 + colon_pos + 1..].trim(); // after ':'
        Some((key.to_string(), value.to_string()))
    } else {
        None
    }
}

/// Parse raw body into a flat list of `RawBlock`s.
fn parse_raw_blocks(body: &str) -> Vec<RawBlock> {
    let lines: Vec<&str> = body.lines().collect();
    let mut blocks = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        if line.trim().is_empty() {
            i += 1;
            continue;
        }

        if is_block_line(line) {
            let indent = count_indent(line);
            let mut content_lines = vec![block_content(line).to_string()];
            let mut properties = BTreeMap::new();
            let mut marker = None;
            let mut priority = None;
            let mut block_id = None;

            i += 1;

            // Consume continuation lines and properties
            while i < lines.len() {
                let next_line = lines[i];
                let trimmed = next_line.trim();

                if trimmed.is_empty() {
                    i += 1;
                    continue;
                }

                let next_indent = count_indent(next_line);

                // If we hit another block line at same or lower indent, stop
                if is_block_line(next_line) && next_indent <= indent {
                    break;
                }

                // Try to parse as property
                if let Some((key, value)) = parse_property(next_line) {
                    match key.as_str() {
                        "id" => {
                            block_id = Uuid::from_str(&value).ok();
                        }
                        "marker" => {
                            marker = Some(value.to_uppercase());
                        }
                        "priority" => {
                            priority = Some(value.to_uppercase());
                        }
                        _ => {
                            properties.insert(key, value);
                        }
                    }
                    i += 1;
                    continue;
                }

                // Must be a continuation line or child block
                if is_block_line(next_line) {
                    // Child block at deeper indent - stop consuming for this block
                    break;
                }

                // Continuation line
                content_lines.push(trimmed.to_string());
                i += 1;
            }

            blocks.push(RawBlock {
                indent,
                content_lines,
                properties,
                marker,
                priority,
                block_id,
            });
        } else {
            i += 1;
        }
    }

    blocks
}

/// Build a tree of blocks from flat `RawBlock`s, assigning parent_id and left_id.
/// Returns blocks in document order with proper hierarchy.
pub(crate) fn build_block_tree(raw_blocks: &[RawBlock]) -> Vec<Block> {
    let mut result = Vec::new();
    // Stack of (indent, block_id) representing the parent chain.
    let mut parent_stack: Vec<(usize, Uuid)> = Vec::new();
    // Map from indent level to the last block ID seen at that level.
    let mut last_at_indent: BTreeMap<usize, Uuid> = BTreeMap::new();

    for raw in raw_blocks {
        let id = raw.block_id.unwrap_or_else(Uuid::new_v4);

        // Pop stack until the top has less indent (i.e., is our parent).
        while let Some(&(stack_indent, _)) = parent_stack.last() {
            if stack_indent >= raw.indent {
                parent_stack.pop();
            } else {
                break;
            }
        }

        let parent_id = parent_stack.last().map(|&(_, id)| id);
        let left_id = last_at_indent.get(&raw.indent).copied();

        let mut block = Block::new(id, raw.content_lines.join("\n"));

        if let Some(pid) = parent_id {
            block.parent_id = Some(pid);
        }
        if let Some(lid) = left_id {
            block.left_id = Some(lid);
        }

        // Set marker
        if let Some(ref m) = raw.marker {
            block.marker = TaskMarker::from_str(m);
        }

        // Set priority
        if let Some(ref p) = raw.priority {
            block.priority = Priority::from_str(p);
        }

        // Set properties
        for (key, value) in &raw.properties {
            block.properties.insert(key.clone(), value.clone());
        }

        result.push(block);

        // Track state for next block
        last_at_indent.insert(raw.indent, id);
        parent_stack.push((raw.indent, id));
    }

    result
}

/// Parse a full markdown document into its frontmatter, body, and blocks.
///
/// Returns (frontmatter, body_text, blocks).
pub fn parse_document(raw: &str) -> (pkm_core::Frontmatter, String, Vec<Block>) {
    let (frontmatter, body) = crate::parser::parse_frontmatter(raw);
    let raw_blocks = parse_raw_blocks(&body);
    let blocks = build_block_tree(&raw_blocks);
    (frontmatter, body, blocks)
}

/// Serialize blocks to markdown body text.
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

fn serialize_ordered_blocks(blocks: &[Block], depth: usize) -> String {
    let mut output = String::new();

    for block in blocks {
        let indent = "  ".repeat(depth);
        output.push_str(&format!("{}- {}\n", indent, block.content.replace('\n', &format!("\n{}  ", indent))));

        // Write properties
        if let Some(ref marker) = block.marker {
            output.push_str(&format!("{}  .marker: {}\n", indent, marker.as_str()));
        }
        if let Some(ref priority) = block.priority {
            output.push_str(&format!("{}  .priority: {}\n", indent, priority.as_str()));
        }
        for (key, value) in &block.properties {
            output.push_str(&format!("{}  .{}: {}\n", indent, key, value));
        }
        // Always write id last
        output.push_str(&format!("{}  .id: {}\n", indent, block.id));
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_indent() {
        assert_eq!(count_indent(""), 0);
        assert_eq!(count_indent("hello"), 0);
        assert_eq!(count_indent("  hello"), 2);
        assert_eq!(count_indent("    hello"), 4);
    }

    #[test]
    fn test_is_block_line() {
        assert!(is_block_line("- hello"));
        assert!(is_block_line("  - hello"));
        assert!(!is_block_line("hello"));
        assert!(!is_block_line("-- hello"));
    }

    #[test]
    fn test_block_content() {
        assert_eq!(block_content("- hello"), "hello");
        assert_eq!(block_content("  - hello world"), "hello world");
    }

    #[test]
    fn test_parse_property() {
        assert_eq!(
            parse_property(".id: abc-123"),
            Some(("id".into(), "abc-123".into()))
        );
        assert_eq!(
            parse_property("  .priority: A"),
            Some(("priority".into(), "A".into()))
        );
        assert_eq!(
            parse_property(".deadline: 2026-07-01"),
            Some(("deadline".into(), "2026-07-01".into()))
        );
        assert_eq!(parse_property("not a property"), None);
        assert_eq!(parse_property("- block line"), None);
        assert_eq!(parse_property(".nocolon"), None);
    }

    #[test]
    fn test_parse_single_block() {
        let body = "- Hello world\n";
        let raw = parse_raw_blocks(body);
        assert_eq!(raw.len(), 1);
        assert_eq!(raw[0].content_lines, vec!["Hello world"]);
    }

    #[test]
    fn test_parse_block_with_properties() {
        let body = "- A task\n  .marker: TODO\n  .priority: A\n  .deadline: tomorrow\n";
        let raw = parse_raw_blocks(body);
        assert_eq!(raw.len(), 1);
        assert_eq!(raw[0].marker, Some("TODO".into()));
        assert_eq!(raw[0].priority, Some("A".into()));
        assert_eq!(raw[0].properties.get("deadline").unwrap(), "tomorrow");
    }

    #[test]
    fn test_parse_block_with_id() {
        let id = "65f8a1e2-3a4b-4c5d-6e7f-8a9b0c1d2e3f";
        let body = format!("- Test block\n  .id: {}\n", id);
        let raw = parse_raw_blocks(&body);
        assert_eq!(raw.len(), 1);
        assert_eq!(raw[0].block_id, Uuid::from_str(id).ok());
    }

    #[test]
    fn test_parse_nested_blocks() {
        let body = "- Parent\n  - Child\n    - Grandchild\n  - Sibling\n";
        let raw = parse_raw_blocks(body);
        assert_eq!(raw.len(), 4);
        assert_eq!(raw[0].indent, 0);
        assert_eq!(raw[1].indent, 2);
        assert_eq!(raw[2].indent, 4);
        assert_eq!(raw[3].indent, 2);
    }

    #[test]
    fn test_parse_multiple_roots() {
        let body = "- First\n- Second\n- Third\n";
        let raw = parse_raw_blocks(body);
        assert_eq!(raw.len(), 3);
        assert_eq!(raw[0].indent, 0);
        assert_eq!(raw[1].indent, 0);
        assert_eq!(raw[2].indent, 0);
    }

    #[test]
    fn test_parse_block_with_continuation() {
        let body = "- First line\n  continuation\n- Next block\n";
        let raw = parse_raw_blocks(body);
        assert_eq!(raw.len(), 2);
        assert_eq!(raw[0].content_lines, vec!["First line", "continuation"]);
        assert_eq!(raw[1].content_lines, vec!["Next block"]);
    }

    #[test]
    fn test_parse_empty_body() {
        let raw = parse_raw_blocks("");
        assert!(raw.is_empty());
    }

    #[test]
    fn test_build_block_tree_simple() {
        let raw = parse_raw_blocks("- First\n- Second\n");
        let blocks = build_block_tree(&raw);
        assert_eq!(blocks.len(), 2);
        assert!(blocks[0].parent_id.is_none());
        assert!(blocks[1].parent_id.is_none());
        assert_eq!(blocks[1].left_id, Some(blocks[0].id));
    }

    #[test]
    fn test_build_block_tree_nested() {
        let raw = parse_raw_blocks("- Parent\n  - Child\n");
        let blocks = build_block_tree(&raw);
        assert_eq!(blocks.len(), 2);
        assert!(blocks[0].parent_id.is_none());
        assert_eq!(blocks[1].parent_id, Some(blocks[0].id));
    }

    #[test]
    fn test_parse_document() {
        let raw = "---\ntitle: Test\ntags: [demo]\n---\n- First block\n  .id: 65f8a1e2-3a4b-4c5d-6e7f-8a9b0c1d2e3f\n- Second block\n";
        let (fm, _body, blocks) = parse_document(raw);
        assert_eq!(fm.title, Some("Test".into()));
        assert_eq!(blocks.len(), 2);
    }

    #[test]
    fn test_serialize_round_trip() {
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        let mut block1 = Block::new(id1, "Hello".into());
        block1.left_id = None;
        block1.parent_id = None;

        let mut block2 = Block::new(id2, "World".into());
        block2.left_id = Some(id1);
        block2.parent_id = None;

        let blocks = vec![block1, block2];
        let serialized = serialize_blocks(&blocks);
        assert!(serialized.contains("- Hello"));
        assert!(serialized.contains("- World"));
        assert!(serialized.contains(&format!(".id: {}", id1)));
        assert!(serialized.contains(&format!(".id: {}", id2)));
    }
}
