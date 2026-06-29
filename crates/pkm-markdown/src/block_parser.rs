//! Block-aware markdown parser.
//!
//! Parses markdown body content into a tree of blocks using indentation to
//! determine hierarchy. Supports Logseq-style properties with `.` prefix,
//! task markers, and block references.

use pkm_block::{Block, Priority, TaskMarker};
use std::collections::{BTreeMap, HashMap};
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
    heading_level: Option<u8>,
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
                            heading_level = value.parse::<u8>().ok().filter(|l| (1..=6).contains(l));
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

/// Convert plain markdown body into a Vec of Blocks using line-by-line state-machine parsing.
///
/// Processes each line individually, detecting block types on ANY line (not just the first
/// line of a chunk). Preserves all inline formatting (wiki-links, bold, italic, code, tags)
/// as raw text for the frontend's post-processing to handle.
pub fn convert_body_to_blocks(body: &str) -> Vec<Block> {
    if body.trim().is_empty() {
        return Vec::new();
    }

    let mut blocks: Vec<Block> = Vec::new();
    let lines: Vec<&str> = body.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        // Skip blank lines
        if line.trim().is_empty() {
            i += 1;
            continue;
        }

        // Fenced code block
        if let Some(lang) = match_fence(line) {
            let mut code_lines = Vec::new();
            i += 1;
            while i < lines.len() {
                if lines[i].trim_start().starts_with("```")
                    || lines[i].trim_start().starts_with("~~~")
                {
                    i += 1;
                    break;
                }
                code_lines.push(lines[i]);
                i += 1;
            }
            let code_body = code_lines.join("\n");
            let content = if lang.is_empty() {
                format!("```\n{}\n```", code_body)
            } else {
                format!("```{}\n{}\n```", lang, code_body)
            };
            blocks.push(Block::new(Uuid::new_v4(), content));
            continue;
        }

        // ATX heading — each heading line produces exactly one block
        if let Some(hl) = parse_atx_heading(line) {
            let content = strip_atx_marker(line).trim().to_string();
            let mut block = Block::new(Uuid::new_v4(), content);
            block.meta.heading_level = Some(hl);
            blocks.push(block);
            i += 1;
            continue;
        }

        // Thematic break (standalone line)
        if is_thematic_break(line) {
            blocks.push(Block::new(Uuid::new_v4(), "---".to_string()));
            i += 1;
            continue;
        }

        // Blockquote
        if line.trim_start().starts_with('>') {
            let mut quote_lines = Vec::new();
            while i < lines.len() {
                let l = lines[i];
                if l.trim().is_empty() {
                    i += 1;
                    break;
                }
                let trimmed = l.trim_start();
                if let Some(rest) = trimmed.strip_prefix("> ") {
                    quote_lines.push(rest);
                } else if let Some(rest) = trimmed.strip_prefix('>') {
                    quote_lines.push(rest);
                } else {
                    quote_lines.push(trimmed);
                }
                i += 1;
            }
            let content = format!("> {}", quote_lines.join("\n"));
            blocks.push(Block::new(Uuid::new_v4(), content));
            continue;
        }

        // List items: one block per item, processing consecutive list lines
        let trimmed = line.trim_start();
        if trimmed.starts_with("- ")
            || trimmed.starts_with("* ")
            || trimmed.starts_with("+ ")
            || is_ordered_list_item(trimmed)
        {
            let mut list_blocks: Vec<Block> = Vec::new();
            let mut list_parent_stack: Vec<(usize, Uuid)> = Vec::new();

            while i < lines.len() {
                let cl = lines[i];
                let ct = cl.trim_start();
                if cl.trim().is_empty() {
                    i += 1;
                    break;
                }
                if !ct.starts_with("- ")
                    && !ct.starts_with("* ")
                    && !ct.starts_with("+ ")
                    && !is_ordered_list_item(ct)
                {
                    // Non-list line ends the list group
                    break;
                }
                let indent = cl.len() - cl.trim_start().len();
                let depth = indent / 2;
                let content = strip_list_marker(ct);
                let id = Uuid::new_v4();
                let mut block = Block::new(id, content);

                while let Some(&(d, _)) = list_parent_stack.last() {
                    if d >= depth {
                        list_parent_stack.pop();
                    } else {
                        break;
                    }
                }
                if let Some(&(_, parent_id)) = list_parent_stack.last() {
                    block.parent_id = Some(parent_id);
                }
                list_parent_stack.push((depth, block.id));
                list_blocks.push(block);
                i += 1;
            }
            blocks.extend(list_blocks);
            continue;
        }

        // Default: paragraph block. Accumulate lines until we hit a blank line
        // or a line that starts a new block type.
        let mut para_lines = vec![line];
        i += 1;
        while i < lines.len() {
            let nl = lines[i];
            if nl.trim().is_empty() {
                i += 1;
                break;
            }
            let nt = nl.trim_start();
            if parse_atx_heading(nl).is_some()
                || is_thematic_break(nl)
                || nt.starts_with('>')
                || nt.starts_with("- ")
                || nt.starts_with("* ")
                || nt.starts_with("+ ")
                || is_ordered_list_item(nt)
                || match_fence(nl).is_some()
            {
                break;
            }
            para_lines.push(nl);
            i += 1;
        }
        let content = para_lines.join("\n").trim().to_string();
        if !content.is_empty() {
            blocks.push(Block::new(Uuid::new_v4(), content));
        }
    }

    // Assign left_id for sibling ordering
    let mut last_at_depth: BTreeMap<usize, Uuid> = BTreeMap::new();
    let mut depth_map: HashMap<Uuid, usize> = HashMap::new();
    for block in &blocks {
        let depth = if let Some(pid) = block.parent_id {
            depth_map.get(&pid).copied().unwrap_or(0) + 1
        } else {
            0
        };
        depth_map.insert(block.id, depth);
    }
    let mut result: Vec<Block> = Vec::new();
    for mut block in blocks {
        let depth = depth_map.get(&block.id).copied().unwrap_or(0);
        if let Some(&prev_id) = last_at_depth.get(&depth) {
            block.left_id = Some(prev_id);
        }
        last_at_depth.insert(depth, block.id);
        result.push(block);
    }
    result
}

/// Detect fenced code block opener, returning the language (or empty string).
fn match_fence(line: &str) -> Option<String> {
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
fn parse_atx_heading(line: &str) -> Option<u8> {
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
fn strip_atx_marker(line: &str) -> &str {
    let trimmed = line.trim_start();
    if let Some(rest) = trimmed.strip_prefix(|c: char| c == '#') {
        let after = rest.trim_start_matches(|c: char| c == '#');
        after.trim_start()
    } else {
        trimmed
    }
}

/// Check if a line is a thematic break (`---`, `***`, `___` with optional spaces).
fn is_thematic_break(line: &str) -> bool {
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
fn is_ordered_list_item(text: &str) -> bool {
    let rest = text.trim_start();
    if let Some(rest) = rest.strip_prefix(|c: char| c.is_ascii_digit()) {
        let after = rest.trim_start_matches(|c: char| c.is_ascii_digit());
        after.starts_with(". ")
    } else {
        false
    }
}

/// Strip list marker (`- `, `* `, `+ `, or `N. `) from the start of a line.
fn strip_list_marker(text: &str) -> String {
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

/// Parse a full markdown document using only the plain-text converter, ignoring any
/// existing `- ` block syntax. Used for reindexing where the file should be treated
/// as plain markdown regardless of prior editor saves.
///
/// Returns (frontmatter, body_text, blocks).
pub fn parse_document_as_plain_markdown(raw: &str) -> (pkm_core::Frontmatter, String, Vec<Block>) {
    let (frontmatter, body) = crate::parser::parse_frontmatter(raw);
    let blocks = convert_body_to_blocks(&body);
    (frontmatter, body, blocks)
}

/// Parse a full markdown document into its frontmatter, body, and blocks.
///
/// Returns (frontmatter, body_text, blocks).
pub fn parse_document(raw: &str) -> (pkm_core::Frontmatter, String, Vec<Block>) {
    let (frontmatter, body) = crate::parser::parse_frontmatter(raw);
    let raw_blocks = parse_raw_blocks(&body);
    let blocks = if raw_blocks.is_empty() {
        let converted = convert_body_to_blocks(&body);
        if !converted.is_empty() {
            converted
        } else {
            build_block_tree(&raw_blocks)
        }
    } else {
        build_block_tree(&raw_blocks)
    };
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
    fn test_convert_body_to_blocks_basic() {
        let body = "## Backend\n\n[[Node.js]] / [[Express]] or [[Fastify]]\n";
        let blocks = convert_body_to_blocks(body);
        assert_eq!(blocks.len(), 2, "should produce 2 blocks");
        assert_eq!(blocks[0].content, "Backend");
        assert_eq!(blocks[0].meta.heading_level, Some(2));
        assert_eq!(blocks[1].content, "[[Node.js]] / [[Express]] or [[Fastify]]");
        assert!(blocks[1].meta.heading_level.is_none());
    }

    #[test]
    fn test_convert_body_to_blocks_full_sample() {
        let body = "[[React]] with [[TypeScript]] and [[Tailwind CSS]]\n\nResponsive design with CSS Grid and Flexbox\n\n## Backend\n\n[[Node.js]] / [[Express]] or [[Fastify]]\n";
        let blocks = convert_body_to_blocks(body);
        assert_eq!(blocks.len(), 4, "should produce 4 blocks: 2 paras, 1 heading, 1 para");
        assert_eq!(blocks[0].content, "[[React]] with [[TypeScript]] and [[Tailwind CSS]]");
        assert_eq!(blocks[1].content, "Responsive design with CSS Grid and Flexbox");
        assert_eq!(blocks[2].content, "Backend");
        assert_eq!(blocks[2].meta.heading_level, Some(2));
        assert_eq!(blocks[3].content, "[[Node.js]] / [[Express]] or [[Fastify]]");
    }

    #[test]
    fn test_parse_document_fallback() {
        let raw = "## Test Heading\n\nSome paragraph with **bold** text.\n";
        let (fm, _body, blocks) = parse_document(raw);
        assert!(fm.title.is_none());
        assert_eq!(blocks.len(), 2, "should fall back to converter and produce 2 blocks");
        assert_eq!(blocks[0].content, "Test Heading");
        assert_eq!(blocks[0].meta.heading_level, Some(2));
        assert_eq!(blocks[1].content, "Some paragraph with **bold** text.");
    }

    #[test]
    fn test_convert_heading_after_text_no_blank_line() {
        let body = "Some intro text\n## Algorithms\nMore text here\n";
        let blocks = convert_body_to_blocks(body);
        assert_eq!(blocks.len(), 3, "should produce 3 blocks: para, heading, para");
        assert_eq!(blocks[0].content, "Some intro text");
        assert!(blocks[0].meta.heading_level.is_none());
        assert_eq!(blocks[1].content, "Algorithms");
        assert_eq!(blocks[1].meta.heading_level, Some(2));
        assert_eq!(blocks[2].content, "More text here");
        assert!(blocks[2].meta.heading_level.is_none());
    }

    #[test]
    fn test_convert_user_content_exact() {
        let body = "Supervised learning (classification, regression)\n\nUnsupervised learning (clustering, dimensionality reduction)\n\nReinforcement learning (agents, environments)\n\nSemi-supervised and self-supervised learning\\\n## Algorithms\n\nLinear/Logistic regression\n\nDecision trees and Random Forests\n\nSVM (Support Vector Machines)\n\nk-Nearest Neighbors\n\nNeural networks (see [[[Neural-Networks]]]())\\\n## Pipeline\\\n1. Data collection and cleaning\\\n2. Feature engineering and selection\\\n3. Model selection and training\\\n4. Evaluation and validation\\\n5. Deployment and monitoring\\\n## Related\n\n[[[AI]]]() — broader AI context\n\n[[[Python]]]() — scikit-learn, pandas, NumPy\n\n[[[Mathematics]]]() — statistics and linear algebra\n\n[[[Databases]]]() — data storage for ML\\\n#machinelearning #ml #datascience #ai";
        let blocks = convert_body_to_blocks(body);
        // At minimum: paras before ## Algorithms, then Algos heading, then more paragraphs, 
        // then ## Pipeline heading, then 5 ordered list items, then ## Related heading, then more paragraphs
        assert!(blocks.len() >= 12, "should produce at least 12 blocks, got {}", blocks.len());
        
        // Find the ## Algorithms heading
        let algo = blocks.iter().find(|b| b.content == "Algorithms" && b.meta.heading_level == Some(2));
        assert!(algo.is_some(), "should find ## Algorithms heading block");
        
        // Find the ## Pipeline heading
        let pipe = blocks.iter().find(|b| b.content == "Pipeline\\" && b.meta.heading_level == Some(2));
        assert!(pipe.is_some(), "should find ## Pipeline heading block");
        
        // Find the ## Related heading
        let related = blocks.iter().find(|b| b.content == "Related" && b.meta.heading_level == Some(2));
        assert!(related.is_some(), "should find ## Related heading block");
        
        // Should have ordered list items (1. 2. 3. 4. 5.)
        let list_items: Vec<_> = blocks.iter().filter(|b| b.content.contains("Data collection")).collect();
        assert!(!list_items.is_empty(), "should find ordered list items");
    }

    #[test]
    fn test_convert_preserves_wiki_links() {
        let body = "[[React]] with [[TypeScript]] and [[Tailwind CSS]]\n\n## Backend\n\n[[Node.js]] / [[Express]] or [[Fastify]]\n";
        let blocks = convert_body_to_blocks(body);
        assert_eq!(blocks.len(), 3, "should produce 3 blocks: para, heading, para");
        assert!(blocks[0].content.contains("[[React]]"));
        assert!(blocks[0].content.contains("[[TypeScript]]"));
        assert!(blocks[0].content.contains("[[Tailwind CSS]]"));
        assert_eq!(blocks[1].content, "Backend");
        assert_eq!(blocks[1].meta.heading_level, Some(2));
        assert!(blocks[2].content.contains("[[Node.js]]"));
    }

    #[test]
    fn test_convert_preserves_bold_italic() {
        let body = "**bold** and *italic* and `code` and ~~strike~~\n\nSome plain text";
        let blocks = convert_body_to_blocks(body);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].content, "**bold** and *italic* and `code` and ~~strike~~");
        assert_eq!(blocks[1].content, "Some plain text");
    }

    #[test]
    fn test_convert_detects_atx_headings() {
        let body = "# H1\n\n## H2\n\n### H3\n";
        let blocks = convert_body_to_blocks(body);
        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[0].content, "H1");
        assert_eq!(blocks[0].meta.heading_level, Some(1));
        assert_eq!(blocks[1].content, "H2");
        assert_eq!(blocks[1].meta.heading_level, Some(2));
        assert_eq!(blocks[2].content, "H3");
        assert_eq!(blocks[2].meta.heading_level, Some(3));
    }

    #[test]
    fn test_convert_preserves_emoji_lines() {
        let body = "🐍 The Ouroboros Symbol\n\nSome text here.\n";
        let blocks = convert_body_to_blocks(body);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].content, "🐍 The Ouroboros Symbol");
        assert_eq!(blocks[1].content, "Some text here.");
    }

    #[test]
    fn test_convert_list_items() {
        let body = "- Item 1\n- Item 2\n  - Nested item\n";
        let blocks = convert_body_to_blocks(body);
        // 3 items in one chunk: Item 1, Item 2, Nested item
        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[0].content, "Item 1");
        assert_eq!(blocks[1].content, "Item 2");
        assert_eq!(blocks[2].content, "Nested item");
        // Nested item has a parent
        assert!(blocks[2].parent_id.is_some());
        assert_eq!(blocks[2].parent_id, Some(blocks[1].id));
    }

    #[test]
    fn test_parse_atx_heading() {
        assert_eq!(parse_atx_heading("# H1"), Some(1));
        assert_eq!(parse_atx_heading("## H2"), Some(2));
        assert_eq!(parse_atx_heading("### H3"), Some(3));
        assert_eq!(parse_atx_heading("#### H4"), Some(4));
        assert_eq!(parse_atx_heading("##### H5"), Some(5));
        assert_eq!(parse_atx_heading("###### H6"), Some(6));
        assert_eq!(parse_atx_heading("not a heading"), None);
        assert_eq!(parse_atx_heading("####### too many"), None);
        assert_eq!(parse_atx_heading("🐍 emoji line"), None);
    }

    #[test]
    fn test_is_thematic_break() {
        assert!(is_thematic_break("---"));
        assert!(is_thematic_break("***"));
        assert!(is_thematic_break("___"));
        assert!(is_thematic_break("  ---  "));
        assert!(!is_thematic_break("--"));
        assert!(!is_thematic_break("not a break"));
    }

    #[test]
    fn test_strip_atx_marker() {
        assert_eq!(strip_atx_marker("## Backend"), "Backend");
        assert_eq!(strip_atx_marker("# Only"), "Only");
        assert_eq!(strip_atx_marker("### Deep"), "Deep");
        assert_eq!(strip_atx_marker("No hash"), "No hash");
        assert_eq!(strip_atx_marker(""), "");
    }

    #[test]
    fn test_parse_document_preserves_block_syntax() {
        let raw = "---\ntitle: Existing\n---\n- First block\n  .id: 65f8a1e2-3a4b-4c5d-6e7f-8a9b0c1d2e3f\n- Second block\n";
        let (fm, _body, blocks) = parse_document(raw);
        assert_eq!(fm.title, Some("Existing".into()));
        assert_eq!(blocks.len(), 2, "should still parse - blocks normally");
        assert_eq!(blocks[0].content, "First block");
    }

    #[test]
    fn test_heading_level_round_trip() {
        let id = uuid::Uuid::new_v4();
        let mut block = pkm_block::Block::new(id, "Hello".into());
        block.meta.heading_level = Some(3);
        let blocks = vec![block];
        let serialized = serialize_blocks(&blocks);
        assert!(serialized.contains(".heading-level: 3"));
        // Re-parse
        let (_fm, _body, reparsed) = parse_document(&serialized);
        assert_eq!(reparsed.len(), 1);
        assert_eq!(reparsed[0].meta.heading_level, Some(3));
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
