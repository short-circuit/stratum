//! Block moves resolved from a destination group.
//!
//! [`EditorState::move_block`] reorders one block (with its subtree) inside the
//! document and records the change as a reversible [`EditOp::MoveBlock`] pair,
//! exactly like [`EditorState::move_up`] and [`EditorState::move_down`]. The
//! destination is expressed as a [`MoveTarget`]: a parent plus an index counted
//! after the moved block has been removed from its current group.
//!
//! The destination group is resolved through [`pkm_block::BlockTree::children`]
//! or [`pkm_block::BlockTree::roots`], so the move stays deterministic and never
//! depends on `HashMap` iteration order.

use pkm_block::ops;
use pkm_block::BlockId;
use pkm_core::{PkmError, PkmResult};

use super::ops::EditOp;
use super::EditorState;

/// A resolved destination for a block move.
///
/// `index` counts the destination parent's children *after* the moved block (and
/// its subtree) has been removed, so index 0 means "first child" and
/// `index == children.len()` means "last child".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoveTarget {
    pub parent: Option<BlockId>,
    pub index: usize,
}

impl EditorState {
    /// Move `block_id` (with its subtree) to `target`.
    ///
    /// Returns `Ok(false)` when the block already sits at the destination (no
    /// history entry). The move is recorded as one reversible
    /// [`EditOp::MoveBlock`] entry, exactly like `move_up`/`move_down`.
    ///
    /// # Errors
    ///
    /// Returns [`PkmError::BlockNotFound`] when the block or destination parent
    /// is missing and [`PkmError::CycleDetected`] when the destination parent is
    /// the block itself or one of its descendants.
    pub fn move_block(&mut self, block_id: BlockId, target: MoveTarget) -> PkmResult<bool> {
        let block = self
            .doc
            .block(block_id)
            .ok_or_else(|| PkmError::BlockNotFound(block_id.to_string()))?;
        let from_parent = block.parent_id;
        let from_left = block.left_id;

        if let Some(parent_id) = target.parent {
            if !self.doc.tree().contains(parent_id) {
                return Err(PkmError::BlockNotFound(parent_id.to_string()));
            }
            // `ancestors()` includes the block itself, so this rejects both
            // `parent == block_id` and any descendant destination.
            if self
                .doc
                .tree()
                .ancestors(parent_id)
                .iter()
                .any(|ancestor| ancestor.id == block_id)
            {
                return Err(PkmError::CycleDetected(
                    "Operation would create a cycle".to_string(),
                ));
            }
        }

        // The index counts siblings after the moved block is removed. Filtering
        // the block out here also keeps a same-parent move from resolving to its
        // own old slot.
        let destination: Vec<BlockId> = match target.parent {
            Some(parent_id) => self
                .doc
                .tree()
                .children(parent_id)
                .into_iter()
                .map(|sibling| sibling.id)
                .collect(),
            None => self
                .doc
                .tree()
                .roots()
                .into_iter()
                .map(|sibling| sibling.id)
                .collect(),
        };
        let siblings: Vec<BlockId> = destination
            .into_iter()
            .filter(|sibling| *sibling != block_id)
            .collect();
        let index = target.index.min(siblings.len());
        let to_left = index.checked_sub(1).map(|slot| siblings[slot]);

        if from_parent == target.parent && from_left == to_left {
            return Ok(false);
        }

        self.doc
            .mutate(|tree| ops::move_block(tree, block_id, target.parent, to_left))?;

        // Record the actual post-move pointers, mirroring indent/outdent: the
        // engine may normalise the destination (for example when the requested
        // index is clamped), and the history must replay what really happened.
        let moved = self
            .doc
            .block(block_id)
            .ok_or_else(|| PkmError::BlockNotFound(block_id.to_string()))?;
        let op = EditOp::MoveBlock {
            block_id,
            from_parent,
            from_left,
            to_parent: moved.parent_id,
            to_left: moved.left_id,
        };
        let inverse = op.inverse();
        self.history.push(op, inverse, chrono::Utc::now());
        Ok(true)
    }
}

#[cfg(test)]
mod tests;
