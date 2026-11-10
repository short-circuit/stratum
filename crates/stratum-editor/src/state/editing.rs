//! Caret-driven editing operations.
//!
//! These methods translate user intent (typing, Enter, Backspace, indent,
//! marker changes, span wrapping) into [`EditOp`]s and record their inverses in
//! the history. Offsets are clamped to UTF-8 char boundaries before any
//! structural operation.

use std::ops::Range;

use pkm_block::ops;
use pkm_block::{Block, BlockId, Priority, TaskMarker};
use pkm_core::{PkmError, PkmResult};
use uuid::Uuid;

use super::ops::{child_ids, EditOp};
use super::splice::LinkChange;
use super::text::{clamp_offset, next_char_boundary, prev_char_boundary, span_delimiters};
use super::{Caret, EditorState};
use crate::assets::asset_reference;
use crate::inline::SpanKind;

impl EditorState {
    /// Insert text at the caret, replacing an active selection.
    pub fn insert_text(&mut self, text: &str) -> PkmResult<()> {
        if text.is_empty() {
            return Ok(());
        }
        self.delete_selection()?;
        let caret = self.require_caret()?;
        let content = self.block_content(caret.block_id)?;
        let offset = clamp_offset(&content, caret.byte_offset);
        let op = EditOp::InsertText {
            block_id: caret.block_id,
            offset,
            text: text.to_string(),
        };
        self.apply(op)?;
        self.caret = Some(Caret {
            block_id: caret.block_id,
            byte_offset: offset + text.len(),
        });
        Ok(())
    }

    /// Split the current block at the caret (Enter).
    pub fn insert_newline(&mut self) -> PkmResult<()> {
        self.delete_selection()?;
        let caret = self.require_caret()?;
        let content = self.block_content(caret.block_id)?;
        let offset = clamp_offset(&content, caret.byte_offset);
        let parent = self.doc.block(caret.block_id).and_then(|b| b.parent_id);
        let new_block_id = Uuid::new_v4();
        let op = EditOp::SplitBlock {
            block_id: caret.block_id,
            new_block_id,
            new_parent: parent,
            new_left: Some(caret.block_id),
            prefix: content[..offset].to_string(),
            suffix: content[offset..].to_string(),
            // A user Enter split leaves existing children on the original block.
            children: Vec::new(),
        };
        self.apply(op)?;
        self.caret = Some(Caret {
            block_id: new_block_id,
            byte_offset: 0,
        });
        Ok(())
    }

    /// Delete backward (Backspace).
    pub fn backspace(&mut self) -> PkmResult<()> {
        self.delete_selection()?;
        let caret = self.require_caret()?;
        let content = self.block_content(caret.block_id)?;
        let offset = clamp_offset(&content, caret.byte_offset);
        if offset > 0 {
            let prev = prev_char_boundary(&content, offset);
            let op = EditOp::DeleteText {
                block_id: caret.block_id,
                offset: prev,
                text: content[prev..offset].to_string(),
            };
            self.apply(op)?;
            self.caret = Some(Caret {
                block_id: caret.block_id,
                byte_offset: prev,
            });
            return Ok(());
        }
        self.merge_block()
    }

    /// Delete forward (Delete).
    pub fn delete_forward(&mut self) -> PkmResult<()> {
        self.delete_selection()?;
        let caret = self.require_caret()?;
        let content = self.block_content(caret.block_id)?;
        let offset = clamp_offset(&content, caret.byte_offset);
        if offset < content.len() {
            let next = next_char_boundary(&content, offset);
            let op = EditOp::DeleteText {
                block_id: caret.block_id,
                offset,
                text: content[offset..next].to_string(),
            };
            self.apply(op)?;
            return Ok(());
        }
        // At the end of a block, merge the next sibling into this block.
        let Some(next_block) = self.doc.tree().next_sibling(caret.block_id) else {
            return Ok(());
        };
        let next_id = next_block.id;
        let next_content = next_block.content.clone();
        let next_parent = next_block.parent_id;
        let next_left = next_block.left_id;
        let children = child_ids(self.doc.tree(), next_id);
        let op = EditOp::MergeBlock {
            block_id: next_id,
            prev_id: caret.block_id,
            prev_prefix: content,
            merged_text: next_content,
            block_parent: next_parent,
            block_left: next_left,
            children,
        };
        self.apply(op)?;
        Ok(())
    }

    /// Merge the current block into its previous sibling.
    pub fn merge_block(&mut self) -> PkmResult<()> {
        let caret = self.require_caret()?;
        let Some(prev_id) = self.doc.tree().prev_sibling(caret.block_id) else {
            return Ok(());
        };
        let prev_content = self.block_content(prev_id)?;
        let block_content = self.block_content(caret.block_id)?;
        let block_parent = self.doc.block(caret.block_id).and_then(|b| b.parent_id);
        let block_left = self.doc.block(caret.block_id).and_then(|b| b.left_id);
        let children = child_ids(self.doc.tree(), caret.block_id);
        let op = EditOp::MergeBlock {
            block_id: caret.block_id,
            prev_id,
            prev_prefix: prev_content.clone(),
            merged_text: block_content,
            block_parent,
            block_left,
            children,
        };
        self.apply(op)?;
        self.caret = Some(Caret {
            block_id: prev_id,
            byte_offset: prev_content.len(),
        });
        Ok(())
    }

    /// Indent the current block under its previous sibling.
    pub fn indent(&mut self) -> PkmResult<()> {
        let caret = self.require_caret()?;
        let block = self
            .doc
            .block(caret.block_id)
            .ok_or_else(|| PkmError::BlockNotFound(caret.block_id.to_string()))?;
        let from_parent = block.parent_id;
        let from_left = block.left_id;
        self.doc
            .mutate(|tree| ops::indent_block(tree, caret.block_id))?;
        let moved = self
            .doc
            .block(caret.block_id)
            .ok_or_else(|| PkmError::BlockNotFound(caret.block_id.to_string()))?;
        let op = EditOp::Indent {
            block_id: caret.block_id,
            from_parent,
            from_left,
            to_parent: moved.parent_id,
            to_left: moved.left_id,
        };
        let inverse = op.inverse();
        self.history.push(op, inverse, chrono::Utc::now());
        Ok(())
    }

    /// Outdent the current block to its parent's level.
    pub fn outdent(&mut self) -> PkmResult<()> {
        let caret = self.require_caret()?;
        let block = self
            .doc
            .block(caret.block_id)
            .ok_or_else(|| PkmError::BlockNotFound(caret.block_id.to_string()))?;
        let from_parent = block.parent_id;
        let from_left = block.left_id;
        self.doc
            .mutate(|tree| ops::outdent_block(tree, caret.block_id))?;
        let moved = self
            .doc
            .block(caret.block_id)
            .ok_or_else(|| PkmError::BlockNotFound(caret.block_id.to_string()))?;
        let op = EditOp::Outdent {
            block_id: caret.block_id,
            from_parent,
            from_left,
            to_parent: moved.parent_id,
            to_left: moved.left_id,
        };
        let inverse = op.inverse();
        self.history.push(op, inverse, chrono::Utc::now());
        Ok(())
    }

    /// Move the current block above its previous sibling.
    pub fn move_up(&mut self) -> PkmResult<()> {
        let caret = self.require_caret()?;
        let block = self
            .doc
            .block(caret.block_id)
            .ok_or_else(|| PkmError::BlockNotFound(caret.block_id.to_string()))?;
        let Some(prev) = self.doc.tree().prev_sibling(caret.block_id) else {
            return Ok(());
        };
        let prev_block = self
            .doc
            .block(prev)
            .ok_or_else(|| PkmError::BlockNotFound(prev.to_string()))?;
        let from_parent = block.parent_id;
        let from_left = block.left_id;
        let to_left = prev_block.left_id;
        let op = EditOp::MoveBlock {
            block_id: caret.block_id,
            from_parent,
            from_left,
            to_parent: from_parent,
            to_left,
        };
        let inverse = op.inverse();
        self.doc
            .mutate(|tree| ops::move_block(tree, caret.block_id, from_parent, to_left))?;
        self.history.push(op, inverse, chrono::Utc::now());
        Ok(())
    }

    /// Move the current block below its next sibling.
    pub fn move_down(&mut self) -> PkmResult<()> {
        let caret = self.require_caret()?;
        let block = self
            .doc
            .block(caret.block_id)
            .ok_or_else(|| PkmError::BlockNotFound(caret.block_id.to_string()))?;
        let Some(next) = self.doc.tree().next_sibling(caret.block_id) else {
            return Ok(());
        };
        let from_parent = block.parent_id;
        let from_left = block.left_id;
        let to_left = Some(next.id);
        let op = EditOp::MoveBlock {
            block_id: caret.block_id,
            from_parent,
            from_left,
            to_parent: from_parent,
            to_left,
        };
        let inverse = op.inverse();
        self.doc
            .mutate(|tree| ops::move_block(tree, caret.block_id, from_parent, to_left))?;
        self.history.push(op, inverse, chrono::Utc::now());
        Ok(())
    }

    /// Set or clear the current block marker.
    pub fn set_marker(&mut self, marker: Option<TaskMarker>) -> PkmResult<()> {
        let caret = self.require_caret()?;
        let old = self
            .doc
            .block(caret.block_id)
            .ok_or_else(|| PkmError::BlockNotFound(caret.block_id.to_string()))?
            .marker;
        if old == marker {
            return Ok(());
        }
        let op = EditOp::SetMarker {
            block_id: caret.block_id,
            old,
            new: marker,
        };
        self.apply(op)
    }

    /// Clear the current block marker.
    pub fn clear_marker(&mut self) -> PkmResult<()> {
        self.set_marker(None)
    }

    /// Set or clear the current block priority.
    pub fn set_priority(&mut self, priority: Option<Priority>) -> PkmResult<()> {
        let caret = self.require_caret()?;
        let old = self
            .doc
            .block(caret.block_id)
            .ok_or_else(|| PkmError::BlockNotFound(caret.block_id.to_string()))?
            .priority;
        if old == priority {
            return Ok(());
        }
        let op = EditOp::SetPriority {
            block_id: caret.block_id,
            old,
            new: priority,
        };
        self.apply(op)
    }

    /// Wrap or unwrap `range` with the delimiters for `kind`.
    pub fn apply_span(
        &mut self,
        block_id: BlockId,
        range: Range<usize>,
        kind: SpanKind,
    ) -> PkmResult<()> {
        let content = self.block_content(block_id)?;
        let start = clamp_offset(&content, range.start);
        let end = clamp_offset(&content, range.end);
        if start > end {
            return Err(PkmError::Validation("invalid span range".into()));
        }
        let (open, close) = span_delimiters(&kind);
        let already = start >= open.len()
            && content[..start].ends_with(&open)
            && content[end..].starts_with(&close);
        let new_content = if already {
            let inner_start = start - open.len();
            let inner_end = end + close.len();
            format!(
                "{}{}{}",
                &content[..inner_start],
                &content[start..end],
                &content[inner_end..]
            )
        } else {
            format!(
                "{}{}{}{}{}",
                &content[..start],
                open,
                &content[start..end],
                close,
                &content[end..]
            )
        };
        let op = EditOp::SetContent {
            block_id,
            old: content,
            new: new_content,
        };
        self.apply(op)
    }

    /// Set or clear the heading level of the current block.
    pub fn set_heading_level(&mut self, level: Option<u8>) -> PkmResult<()> {
        let caret = self.require_caret()?;
        let old = self
            .doc
            .block(caret.block_id)
            .ok_or_else(|| PkmError::BlockNotFound(caret.block_id.to_string()))?
            .meta
            .heading_level;
        if old == level {
            return Ok(());
        }
        self.apply(EditOp::SetHeading {
            block_id: caret.block_id,
            old,
            new: level,
        })
    }

    /// Replace the whole content of one block and record the edit.
    ///
    /// Used by the view for block-kind conversions (fenced code, Mermaid and
    /// display math) and wiki-link completion; the replacement text is built by
    /// the caller but applied here so undo stays authoritative in the engine.
    pub fn replace_block_content(&mut self, block_id: BlockId, new: String) -> PkmResult<()> {
        let old = self.block_content(block_id)?;
        if old == new {
            return Ok(());
        }
        let op = EditOp::SetContent { block_id, old, new };
        self.apply(op)
    }

    /// Insert an asset embed reference as a new sibling block after the caret.
    ///
    /// Mirrors React `insertBlock(pagePath, "![[…]]")`: when the caret block has
    /// content, a sibling block carrying [`asset_reference`] is inserted after
    /// it (same parent, left chain re-linked) and the caret moves into the new
    /// block. When the caret block is empty its content is replaced instead.
    /// Either path is a single undoable history entry and marks the document
    /// dirty.
    ///
    /// # Errors
    ///
    /// Returns [`PkmError::Internal`] when no caret is placed, or
    /// [`PkmError::BlockNotFound`] when the caret block is missing.
    pub fn insert_asset_reference(&mut self, ref_path: &str) -> PkmResult<()> {
        let reference = asset_reference(ref_path);
        let caret = self.require_caret()?;
        let content = self.block_content(caret.block_id)?;
        if content.is_empty() {
            self.apply(EditOp::SetContent {
                block_id: caret.block_id,
                old: content,
                new: reference.clone(),
            })?;
            self.selection = None;
            self.caret = Some(Caret {
                block_id: caret.block_id,
                byte_offset: reference.len(),
            });
            return Ok(());
        }

        let caret_parent = self.doc.block(caret.block_id).and_then(|b| b.parent_id);
        let new_block_id = Uuid::new_v4();
        let mut inserted = Block::new(new_block_id, reference.clone());
        inserted.parent_id = caret_parent;
        inserted.left_id = Some(caret.block_id);

        // Re-link the sibling that followed the caret so the chain stays whole.
        let mut links = Vec::new();
        if let Some(next) =
            self.doc.tree().all_blocks().find(|block| {
                block.left_id == Some(caret.block_id) && block.parent_id == caret_parent
            })
        {
            links.push(LinkChange {
                block_id: next.id,
                old_parent: next.parent_id,
                new_parent: next.parent_id,
                old_left: next.left_id,
                new_left: Some(new_block_id),
            });
        }

        self.apply(EditOp::SpliceBlocks {
            removed: Vec::new(),
            inserted: vec![inserted],
            links,
            contents: Vec::new(),
        })?;
        self.selection = None;
        self.caret = Some(Caret {
            block_id: new_block_id,
            byte_offset: reference.len(),
        });
        Ok(())
    }

    /// Toggle collapse state on the current block.
    pub fn toggle_collapsed(&mut self) -> PkmResult<bool> {
        let caret = self.require_caret()?;
        let mut collapsed = false;
        self.doc.mutate(|tree| {
            collapsed = ops::toggle_collapsed(tree, caret.block_id)?;
            Ok(())
        })?;
        Ok(collapsed)
    }

    /// Undo the most recent edit.
    pub fn undo(&mut self) -> PkmResult<bool> {
        if self.history.depth() == 0 {
            return Ok(false);
        }
        let mut applied = false;
        let history = &mut self.history;
        self.doc.mutate(|tree| {
            applied = history.undo(tree)?;
            Ok(())
        })?;
        Ok(applied)
    }

    /// Redo the most recent undone edit.
    pub fn redo(&mut self) -> PkmResult<bool> {
        if self.history.redo_depth() == 0 {
            return Ok(false);
        }
        let mut applied = false;
        let history = &mut self.history;
        self.doc.mutate(|tree| {
            applied = history.redo(tree)?;
            Ok(())
        })?;
        Ok(applied)
    }
}
