//! Raw (pre-struct) block parsing: line classification, `RawBlock` construction,
//! and tree building on top of flat block lists.

use pkm_block::{Block, Priority, TaskMarker};
use std::collections::BTreeMap;
use std::str::FromStr;
use uuid::Uuid;

/// A raw block parsed from lines before final struct construction.
#[derive(Debug, Clone)]
pub(crate) struct RawBlock {
    pub(crate) indent: usize,
    pub(crate) content_lines: Vec<String>,
    pub(crate) properties: BTreeMap<String, String>,
    pub(crate) marker: Option<String>,
    pub(crate) priority: Option<String>,
    pub(crate) heading_level: Option<u8>,
    pub(crate) block_id: Option<Uuid>,
}

/// Count leading spaces for a line.
pub(crate) fn count_indent(line: &str) -> usize {
    line.chars().take_while(|c| *c == ' ').count()
}

/// Check if a line starts a new block (begins with `- ` after indentation).
pub(crate) fn is_block_line(line: &str) -> bool {
    let trimmed_start = line.chars().skip_while(|c| *c == ' ').collect::<String>();
    trimmed_start.starts_with("- ")
}

/// Get the block content from a line (everything after `- `, after indentation).
pub(crate) fn block_content(line: &str) -> &str {
    let indent = count_indent(line);
    &line[indent + 2..] // skip indent + "- "
}

/// Parse a property line: returns `(key, value)` if the line looks like `.key: value`.
pub(crate) fn parse_property(line: &str) -> Option<(String, String)> {
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
pub(crate) fn parse_raw_blocks(body: &str) -> Vec<RawBlock> {
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
            let mut heading_level = None;
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
                        "heading-level" => {
                            heading_level =
                                value.parse::<u8>().ok().filter(|l| (1..=6).contains(l));
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
                heading_level,
                block_id,
            });
        } else if let Some(hl) = parse_atx_heading(line) {
            // ATX heading — each heading line produces one RawBlock
            let content = strip_atx_marker(line).trim().to_string();
            blocks.push(RawBlock {
                indent: 0,
                content_lines: vec![content],
                properties: BTreeMap::new(),
                marker: None,
                priority: None,
                heading_level: Some(hl),
                block_id: None,
            });
            i += 1;
        } else {
            // Group consecutive non-block lines into a paragraph block
            let mut para_lines = vec![line];
            i += 1;
            while i < lines.len() {
                let next = lines[i];
                if next.trim().is_empty() {
                    i += 1;
                    break;
                }
                if is_block_line(next) || parse_atx_heading(next).is_some() {
                    break;
                }
                para_lines.push(next);
                i += 1;
            }
            let content = para_lines.join("\n").trim().to_string();
            if !content.is_empty() {
                blocks.push(RawBlock {
                    indent: 0,
                    content_lines: vec![content],
                    properties: BTreeMap::new(),
                    marker: None,
                    priority: None,
                    heading_level: None,
                    block_id: None,
                });
            }
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
            block.marker = TaskMarker::parse(m);
        }

        // Set priority
        if let Some(ref p) = raw.priority {
            block.priority = Priority::parse(p);
        }

        // Set heading level
        if let Some(hl) = raw.heading_level {
            block.meta.heading_level = Some(hl);
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

/// Detect fenced code block opener, returning the language (or empty string).
pub(crate) fn match_fence(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    if let Some(rest) = trimmed.strip_prefix("```") {
        return Some(rest.trim().to_string());
    }
    if let Some(rest) = trimmed.strip_prefix("~~~") {
        return Some(rest.trim().to_string());
    }
    None
}

/// Parse ATX heading marker. Returns heading level (1-6) if line starts with `#{1,6} `.
pub(crate) fn parse_atx_heading(line: &str) -> Option<u8> {
    let trimmed = line.trim_start();
    let mut count = 0u8;
    for ch in trimmed.chars() {
        if ch == '#' {
            count += 1;
            if count > 6 {
                return None;
            }
        } else if ch == ' ' {
            if count > 0 {
                return Some(count);
            } else {
                return None;
            }
        } else {
            return None;
        }
    }
    None
}

/// Strip ATX heading markers from the start of a line.
pub(crate) fn strip_atx_marker(line: &str) -> &str {
    let trimmed = line.trim_start();
    if let Some(rest) = trimmed.strip_prefix('#') {
        let after = rest.trim_start_matches('#');
        after.trim_start()
    } else {
        trimmed
    }
}

/// Check if a line is a thematic break (`---`, `***`, `___` with optional spaces).
pub(crate) fn is_thematic_break(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.len() < 3 {
        return false;
    }
    let chars: Vec<char> = trimmed.chars().collect();
    let first = chars[0];
    if first != '-' && first != '*' && first != '_' {
        return false;
    }
    chars.iter().all(|c| *c == first || *c == ' ')
}

/// Check if a line starts with an ordered list marker (e.g., `1. `, `42. `).
pub(crate) fn is_ordered_list_item(text: &str) -> bool {
    let rest = text.trim_start();
    if let Some(rest) = rest.strip_prefix(|c: char| c.is_ascii_digit()) {
        let after = rest.trim_start_matches(|c: char| c.is_ascii_digit());
        after.starts_with(". ")
    } else {
        false
    }
}

/// Strip list marker (`- `, `* `, `+ `, or `N. `) from the start of a line.
pub(crate) fn strip_list_marker(text: &str) -> String {
    if let Some(rest) = text
        .strip_prefix("- ")
        .or_else(|| text.strip_prefix("* "))
        .or_else(|| text.strip_prefix("+ "))
    {
        return rest.trim_start().to_string();
    }
    if let Some(rest) = text.strip_prefix(|c: char| c.is_ascii_digit()) {
        let after_num = rest.trim_start_matches(|c: char| c.is_ascii_digit());
        if let Some(content) = after_num.strip_prefix(". ") {
            return content.trim_start().to_string();
        }
    }
    text.to_string()
}
