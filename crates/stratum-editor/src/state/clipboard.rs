//! Clipboard payloads, copy/cut/paste and paste normalization.
//!
//! Paste text is normalized before insertion so a paste from a terminal, an
//! email or a web page cannot smuggle control characters, CRLF line endings or
//! runaway blank lines into a note. Markdown pastes split into blocks and turn
//! leading list markers into indent depth.

use std::collections::HashSet;

use pkm_block::{Block, BlockId};
use pkm_core::PkmResult;
use uuid::Uuid;

use super::ops::child_ids;
use super::splice::{ContentChange, LinkChange};
use super::text::{clamp_offset, ordered_offsets};
use super::{Caret, EditOp, EditorState};
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

    /// The plain text of the active selection, or `None` when nothing is
    /// selected. Unlike [`Self::copy_selection`] this is read-only and does not
    /// touch the clipboard.
    pub fn selected_text(&self) -> Option<String> {
        self.selection.map(|_| self.selection_payload().plain_text)
    }

    /// Copy the current selection and delete it.
    pub fn cut_selection(&mut self) -> PkmResult<ClipboardPayload> {
        let payload = self.copy_selection();
        self.delete_selection()?;
        Ok(payload)
    }

    /// Paste a clipboard payload at the caret.
    ///
    /// A multi-block payload first deletes the active selection (its own history
    /// entry), then inserts the payload as one undoable splice and re-links the
    /// sibling that used to follow the caret block.
    pub fn paste_payload(&mut self, payload: &ClipboardPayload) -> PkmResult<()> {
        if payload.blocks.len() <= 1 {
            return self.insert_text(&normalize_paste(&payload.plain_text));
        }
        self.delete_selection()?;
        let caret = self.require_caret()?;
        let parent = self.doc.block(caret.block_id).and_then(|b| b.parent_id);
        let mut inserted: Vec<Block> = Vec::new();
        let mut left = Some(caret.block_id);
        let mut last_id = None;
        for dto in &payload.blocks {
            let mut block = dto_to_block(dto.clone());
            block.id = Uuid::new_v4();
            block.parent_id = parent;
            block.left_id = left;
            let id = block.id;
            inserted.push(block);
            left = Some(id);
            last_id = Some(id);
        }
        let Some(last_id) = last_id else {
            return Ok(());
        };
        // The block that followed the caret block must now follow the last
        // inserted block instead. Only same-parent siblings can be re-linked.
        let mut links = Vec::new();
        if let Some(next) = self
            .doc
            .tree()
            .all_blocks()
            .find(|b| b.left_id == Some(caret.block_id) && b.parent_id == parent)
        {
            links.push(LinkChange {
                block_id: next.id,
                old_parent: next.parent_id,
                new_parent: next.parent_id,
                old_left: next.left_id,
                new_left: Some(last_id),
            });
        }
        let op = EditOp::SpliceBlocks {
            removed: Vec::new(),
            inserted,
            links,
            contents: Vec::new(),
        };
        self.apply(op)?;
        let len = self.block_content(last_id)?.len();
        self.caret = Some(Caret {
            block_id: last_id,
            byte_offset: len,
        });
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
        let Some(start) = ids.iter().position(|id| *id == sel.anchor.block_id) else {
            return ClipboardPayload {
                plain_text: String::new(),
                blocks: Vec::new(),
            };
        };
        let Some(end) = ids.iter().position(|id| *id == sel.head.block_id) else {
            return ClipboardPayload {
                plain_text: String::new(),
                blocks: Vec::new(),
            };
        };
        let (lo, hi) = (start.min(end), start.max(end));
        let (earlier, later) = if start <= end {
            (sel.anchor, sel.head)
        } else {
            (sel.head, sel.anchor)
        };
        let earlier_content = self
            .doc
            .block(earlier.block_id)
            .map(|b| b.content.clone())
            .unwrap_or_default();
        let later_content = self
            .doc
            .block(later.block_id)
            .map(|b| b.content.clone())
            .unwrap_or_default();
        let earlier_offset = clamp_offset(&earlier_content, earlier.byte_offset);
        let later_offset = clamp_offset(&later_content, later.byte_offset);
        let mut blocks: Vec<Block> = Vec::new();
        for (index, id) in ids[lo..=hi].iter().enumerate() {
            let Some(block) = self.doc.block(*id).cloned() else {
                continue;
            };
            let mut block = block;
            if index == 0 {
                block.content = earlier_content[earlier_offset..].to_string();
            } else if index == hi - lo {
                block.content = later_content[..later_offset].to_string();
            }
            blocks.push(block);
        }
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
        let Some(start) = ids.iter().position(|id| *id == sel.anchor.block_id) else {
            return Ok(());
        };
        let Some(end) = ids.iter().position(|id| *id == sel.head.block_id) else {
            return Ok(());
        };
        let (lo, hi) = (start.min(end), start.max(end));
        let (earlier, later) = if start <= end {
            (sel.anchor, sel.head)
        } else {
            (sel.head, sel.anchor)
        };
        let earlier_id = ids[lo];
        let later_id = ids[hi];
        let earlier_content = self.block_content(earlier_id)?;
        let later_content = self.block_content(later_id)?;
        let earlier_offset = clamp_offset(&earlier_content, earlier.byte_offset);
        let later_offset = clamp_offset(&later_content, later.byte_offset);
        let merged = format!(
            "{}{}",
            &earlier_content[..earlier_offset],
            &later_content[later_offset..]
        );

        // Snapshot every block from just after the survivor through the later
        // block, in document order so parents precede their descendants.
        let removed_ids: Vec<BlockId> = ids[lo + 1..=hi].to_vec();
        let removed_set: HashSet<BlockId> = removed_ids.iter().copied().collect();
        let removed: Vec<Block> = removed_ids
            .iter()
            .filter_map(|id| self.doc.block(*id).cloned())
            .collect();

        let tree = self.doc.tree();
        let mut links: Vec<LinkChange> = Vec::new();
        let mut covered: HashSet<BlockId> = HashSet::new();

        // Surviving children of the later block are appended after the earlier
        // block's surviving children, preserving their sibling order.
        let mut last = child_ids(tree, earlier_id)
            .into_iter()
            .rfind(|id| !removed_set.contains(id));
        for child in child_ids(tree, later_id) {
            if removed_set.contains(&child) {
                continue;
            }
            let old_parent = tree.get(child).and_then(|b| b.parent_id);
            let old_left = tree.get(child).and_then(|b| b.left_id);
            links.push(LinkChange {
                block_id: child,
                old_parent,
                new_parent: Some(earlier_id),
                old_left,
                new_left: last,
            });
            covered.insert(child);
            last = Some(child);
        }

        // Any surviving block whose left pointer dangles into the removed set
        // is re-linked to the nearest surviving sibling. A different parent
        // means the nearest candidate is not a sibling, so the block becomes a
        // first child instead.
        let surviving: Vec<Block> = tree
            .all_blocks()
            .filter(|b| !removed_set.contains(&b.id))
            .cloned()
            .collect();
        for block in surviving {
            let Some(left) = block.left_id else {
                continue;
            };
            if !removed_set.contains(&left) || covered.contains(&block.id) {
                continue;
            }
            let mut cursor = Some(left);
            let mut candidate = None;
            while let Some(id) = cursor {
                if !removed_set.contains(&id) {
                    candidate = Some(id);
                    break;
                }
                cursor = tree.get(id).and_then(|b| b.left_id);
            }
            let new_left = match candidate {
                Some(id) => {
                    let candidate_parent = tree.get(id).and_then(|b| b.parent_id);
                    if candidate_parent == block.parent_id {
                        Some(id)
                    } else {
                        None
                    }
                }
                None => None,
            };
            if new_left != block.left_id {
                links.push(LinkChange {
                    block_id: block.id,
                    old_parent: block.parent_id,
                    new_parent: block.parent_id,
                    old_left: block.left_id,
                    new_left,
                });
                covered.insert(block.id);
            }
        }

        let op = EditOp::SpliceBlocks {
            removed,
            inserted: Vec::new(),
            links,
            contents: vec![ContentChange {
                block_id: earlier_id,
                old: earlier_content,
                new: merged,
            }],
        };
        self.apply(op)?;
        self.caret = Some(Caret {
            block_id: earlier_id,
            byte_offset: earlier_offset,
        });
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
