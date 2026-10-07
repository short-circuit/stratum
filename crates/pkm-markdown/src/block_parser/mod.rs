//! Block-aware markdown parser.
//!
//! Parses markdown body content into a tree of blocks using indentation to
//! determine hierarchy. Supports Logseq-style properties with `.` prefix,
//! task markers, and block references.

pub(crate) mod raw;
mod serialize;

pub(crate) use raw::{
    build_block_tree, is_ordered_list_item, is_thematic_break, match_fence, parse_atx_heading,
    parse_raw_blocks, strip_atx_marker, strip_list_marker,
};
pub use serialize::{assemble_blocks_markdown, serialize_blocks};

use pkm_block::Block;
use std::collections::{BTreeMap, HashMap};
use uuid::Uuid;

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

        // Fenced code block — also match when prefixed by `- ` block syntax
        let fence_lang = match_fence(line).or_else(|| {
            let s = line.trim_start();
            s.strip_prefix("- ").and_then(match_fence)
        });
        if let Some(lang) = fence_lang {
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

                // If list content looks like a fenced code block opener
                // (e.g. `- ```mermaid`), drop out of the list handler so
                // the outer loop's code fence handler processes it instead.
                if content.starts_with("```") || content.starts_with("~~~") {
                    break;
                }

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

#[cfg(test)]
mod tests;
