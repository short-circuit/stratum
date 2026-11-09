//! Clipboard payloads, copy/cut/paste and paste normalization.
//!
//! Paste text is normalized before insertion so a paste from a terminal, an
//! email or a web page cannot smuggle control characters, CRLF line endings or
//! runaway blank lines into a note. Markdown pastes split into blocks and turn
//! leading list markers into indent depth.

use pkm_block::ops;
use pkm_block::BlockId;
use pkm_core::PkmResult;
use uuid::Uuid;

use super::ops::link_after;
use super::text::ordered_offsets;
use super::{Caret, EditorState};
use crate::dto::{block_to_dto, blocks_to_dto, dto_to_block, BlockDto};

/// A clipboard payload produced by copy/cut.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardPayload {
    pub plain_text: String,
    pub blocks: Vec<BlockDto>,
}

impl EditorState {
    /// Copy the current selection into a clipboard payload.
    pub fn copy_selection(&mut self) -> ClipboardPayload {
        let payload = self.selection_payload();
        self.clipboard = Some(payload.clone());
        payload
    }

    /// Copy the current selection and delete it.
    pub fn cut_selection(&mut self) -> PkmResult<ClipboardPayload> {
        let payload = self.copy_selection();
        self.delete_selection()?;
        Ok(payload)
    }

    /// Paste a clipboard payload at the caret.
    pub fn paste_payload(&mut self, payload: &ClipboardPayload) -> PkmResult<()> {
        if payload.blocks.len() <= 1 {
            return self.insert_text(&normalize_paste(&payload.plain_text));
        }
        let caret = self.require_caret()?;
        let parent = self.doc.block(caret.block_id).and_then(|b| b.parent_id);
        let mut left = Some(caret.block_id);
        let mut last_id = None;
        for dto in &payload.blocks {
            let mut block = dto_to_block(dto.clone());
            block.id = Uuid::new_v4();
            block.parent_id = parent;
            block.left_id = left;
            let id = block.id;
            self.doc.mutate(|tree| {
                ops::insert_block(tree, block.clone());
                link_after(tree, left, id);
                Ok(())
            })?;
            left = Some(id);
            last_id = Some(id);
        }
        if let Some(id) = last_id {
            let len = self.block_content(id)?.len();
            self.caret = Some(Caret {
                block_id: id,
                byte_offset: len,
            });
        }
        Ok(())
    }

    /// Build a payload from the active selection (empty when none).
    pub(super) fn selection_payload(&self) -> ClipboardPayload {
        let Some(sel) = self.selection else {
            return ClipboardPayload {
                plain_text: String::new(),
                blocks: Vec::new(),
            };
        };
        if sel.anchor.block_id == sel.head.block_id {
            let Some(block) = self.doc.block(sel.anchor.block_id) else {
                return ClipboardPayload {
                    plain_text: String::new(),
                    blocks: Vec::new(),
                };
            };
            let (start, end) = ordered_offsets(sel);
            let text = block.content[start..end].to_string();
            let mut dto = block_to_dto(block);
            dto.content = text.clone();
            return ClipboardPayload {
                plain_text: text,
                blocks: vec![dto],
            };
        }
        let ids = self.doc.ordered_ids();
        let start = ids.iter().position(|id| *id == sel.anchor.block_id);
        let end = ids.iter().position(|id| *id == sel.head.block_id);
        let (lo, hi) = match (start, end) {
            (Some(a), Some(b)) => (a.min(b), a.max(b)),
            _ => (0, 0),
        };
        let blocks: Vec<pkm_block::Block> = ids[lo..=hi]
            .iter()
            .filter_map(|id| self.doc.block(*id).cloned())
            .collect();
        let plain_text = blocks
            .iter()
            .map(|b| b.content.clone())
            .collect::<Vec<_>>()
            .join("\n");
        ClipboardPayload {
            plain_text,
            blocks: blocks_to_dto(&blocks),
        }
    }

    /// Delete the active selection (no-op when none).
    pub(super) fn delete_selection(&mut self) -> PkmResult<()> {
        let Some(sel) = self.selection.take() else {
            return Ok(());
        };
        if sel.anchor.block_id == sel.head.block_id {
            let (start, end) = ordered_offsets(sel);
            if start == end {
                return Ok(());
            }
            let content = self.block_content(sel.anchor.block_id)?;
            let op = super::EditOp::DeleteText {
                block_id: sel.anchor.block_id,
                offset: start,
                text: content[start..end].to_string(),
            };
            self.apply(op)?;
            self.caret = Some(Caret {
                block_id: sel.anchor.block_id,
                byte_offset: start,
            });
            return Ok(());
        }
        let ids = self.doc.ordered_ids();
        let start = ids.iter().position(|id| *id == sel.anchor.block_id);
        let end = ids.iter().position(|id| *id == sel.head.block_id);
        if let (Some(a), Some(b)) = (start, end) {
            let (lo, hi) = (a.min(b), a.max(b));
            let to_delete: Vec<BlockId> = ids[lo..=hi].to_vec();
            self.doc.mutate(|tree| {
                for id in &to_delete {
                    if tree.contains(*id) {
                        ops::delete_block(tree, *id)?;
                    }
                }
                Ok(())
            })?;
            self.caret = Some(Caret {
                block_id: sel.head.block_id,
                byte_offset: sel.head.byte_offset,
            });
        }
        Ok(())
    }
}

/// Normalize pasted text.
///
/// CRLF/CR become LF; NUL and other control characters are stripped; tabs outside
/// fenced code become four spaces; trailing spaces are removed; more than two
/// consecutive blank lines collapse to two; the result is capped at 100 000
/// characters.
pub fn normalize_paste(text: &str) -> String {
    let mut out = String::new();
    let mut in_fence = false;
    let mut blank_run = 0usize;
    for raw_line in text.replace("\r\n", "\n").replace('\r', "\n").split('\n') {
        let line: String = raw_line
            .chars()
            .filter(|c| *c == '\t' || !c.is_control())
            .collect();
        let trimmed_end = line.trim_end();
        let is_fence = trimmed_end.trim_start().starts_with("```")
            || trimmed_end.trim_start().starts_with("~~~");
        let line = if in_fence {
            trimmed_end.to_string()
        } else {
            trimmed_end.replace('\t', "    ")
        };
        if is_fence {
            in_fence = !in_fence;
        }
        if line.is_empty() {
            blank_run += 1;
            if blank_run > 2 {
                continue;
            }
        } else {
            blank_run = 0;
        }
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&line);
    }
    cap_chars(&out, 100_000)
}

/// Split pasted markdown into blocks, converting leading list markers to indent
/// depth (two spaces per level).
pub fn paste_markdown(text: &str) -> Vec<BlockDto> {
    let normalized = normalize_paste(text);
    let mut blocks: Vec<BlockDto> = Vec::new();
    let mut stack: Vec<(usize, BlockId)> = Vec::new();
    let mut last_at_depth: Vec<Option<BlockId>> = Vec::new();
    for line in normalized.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        let depth = indent / 2;
        let content = strip_list_marker(line.trim_start());
        while stack.last().is_some_and(|(d, _)| *d >= depth) {
            stack.pop();
        }
        let parent = stack.last().map(|(_, id)| *id);
        if last_at_depth.len() <= depth {
            last_at_depth.resize(depth + 1, None);
        }
        let left = last_at_depth[depth];
        let id = Uuid::new_v4();
        blocks.push(BlockDto {
            id: id.to_string(),
            content: content.to_string(),
            parent_id: parent.map(|p| p.to_string()),
            left_id: left.map(|l| l.to_string()),
            properties: Vec::new(),
            marker: None,
            priority: None,
            collapsed: false,
            heading_level: None,
        });
        last_at_depth[depth] = Some(id);
        stack.push((depth, id));
    }
    blocks
}

fn cap_chars(text: &str, max: usize) -> String {
    text.chars().take(max).collect()
}

fn strip_list_marker(text: &str) -> String {
    if let Some(rest) = text
        .strip_prefix("- ")
        .or_else(|| text.strip_prefix("* "))
        .or_else(|| text.strip_prefix("+ "))
    {
        return rest.trim_start().to_string();
    }
    let mut chars = text.chars();
    let mut digits = String::new();
    while let Some(c) = chars.next() {
        if c.is_ascii_digit() {
            digits.push(c);
        } else if c == '.' && !digits.is_empty() {
            if let Some(' ') = chars.next() {
                return chars.as_str().trim_start().to_string();
            }
            break;
        } else {
            break;
        }
    }
    text.to_string()
}
