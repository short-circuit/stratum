//! Bounded undo/redo stacks.
//!
//! Entries hold a concrete forward/inverse [`EditOp`] pair, never a document
//! snapshot. The stack is capped at 200 entries; a new edit clears the redo
//! stack; contiguous text insertions in the same block within 750 ms coalesce
//! into one entry so typing one word is one undo step.

use chrono::{DateTime, Utc};
use pkm_block::BlockTree;
use pkm_core::PkmResult;

use super::ops::{apply_op, EditOp};

/// One history entry.
#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub forward: EditOp,
    pub inverse: EditOp,
    pub timestamp: DateTime<Utc>,
}

/// Bounded undo/redo stacks.
#[derive(Debug, Default)]
pub struct History {
    undo: Vec<HistoryEntry>,
    redo: Vec<HistoryEntry>,
}

impl History {
    /// Create an empty history.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of undo entries.
    pub fn depth(&self) -> usize {
        self.undo.len()
    }

    /// Number of redo entries.
    pub fn redo_depth(&self) -> usize {
        self.redo.len()
    }

    /// Push a new edit, coalescing contiguous text insertions in the same block
    /// within 750 ms. A new edit always clears the redo stack.
    pub fn push(&mut self, forward: EditOp, inverse: EditOp, now: DateTime<Utc>) {
        self.redo.clear();
        if let (
            Some(last),
            EditOp::InsertText {
                block_id,
                offset,
                text,
            },
        ) = (self.undo.last_mut(), &forward)
        {
            if let EditOp::InsertText {
                block_id: last_block,
                offset: last_offset,
                text: last_text,
            } = &mut last.forward
            {
                let contiguous = *last_offset + last_text.len() == *offset;
                let recent = now
                    .signed_duration_since(last.timestamp)
                    .num_milliseconds()
                    .abs()
                    <= 750;
                if *last_block == *block_id && contiguous && recent {
                    last_text.push_str(text);
                    let combined = last_text.clone();
                    last.inverse = EditOp::DeleteText {
                        block_id: *block_id,
                        offset: *last_offset,
                        text: combined,
                    };
                    last.timestamp = now;
                    return;
                }
            }
        }
        self.undo.push(HistoryEntry {
            forward,
            inverse,
            timestamp: now,
        });
        if self.undo.len() > 200 {
            self.undo.remove(0);
        }
    }

    /// Apply the most recent inverse. Returns `Ok(false)` when empty.
    pub fn undo(&mut self, tree: &mut BlockTree) -> PkmResult<bool> {
        let Some(entry) = self.undo.pop() else {
            return Ok(false);
        };
        apply_op(tree, &entry.inverse)?;
        self.redo.push(entry);
        Ok(true)
    }

    /// Re-apply the most recent forward operation. Returns `Ok(false)` when empty.
    pub fn redo(&mut self, tree: &mut BlockTree) -> PkmResult<bool> {
        let Some(entry) = self.redo.pop() else {
            return Ok(false);
        };
        apply_op(tree, &entry.forward)?;
        self.undo.push(entry);
        Ok(true)
    }
}
