//! Reversible block-range splices.
//!
//! A [`LinkChange`] and [`ContentChange`] pair carry every parent/left and
//! content delta a range operation needs, so the same data can be replayed in
//! both directions. [`apply_splice`] removes and inserts full block snapshots
//! (parents before children) and then applies the explicit deltas; the caller
//! supplies the deltas for the forward direction, and `swapped()` produces the
//! inverse.

use pkm_block::ops;
use pkm_block::{Block, BlockId, BlockTree};
use pkm_core::{PkmError, PkmResult};

/// A parent/left link delta for one block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkChange {
    pub block_id: BlockId,
    pub old_parent: Option<BlockId>,
    pub new_parent: Option<BlockId>,
    pub old_left: Option<BlockId>,
    pub new_left: Option<BlockId>,
}

impl LinkChange {
    /// The inverse delta (old and new fields swapped).
    pub fn swapped(&self) -> LinkChange {
        LinkChange {
            block_id: self.block_id,
            old_parent: self.new_parent,
            new_parent: self.old_parent,
            old_left: self.new_left,
            new_left: self.old_left,
        }
    }
}

/// A content delta for one block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentChange {
    pub block_id: BlockId,
    pub old: String,
    pub new: String,
}

impl ContentChange {
    /// The inverse delta (old and new content swapped).
    pub fn swapped(&self) -> ContentChange {
        ContentChange {
            block_id: self.block_id,
            old: self.new.clone(),
            new: self.old.clone(),
        }
    }
}

/// Apply a reversible block splice to `tree`.
///
/// `removed`/`inserted` are full block snapshots ordered parents before
/// children; `links`/`contents` carry the explicit deltas. Because the caller
/// supplies every delta, the same data is replayable in both directions through
/// [`LinkChange::swapped`] and [`ContentChange::swapped`].
///
/// # Errors
///
/// Returns [`PkmError::BlockNotFound`] when a `removed` block is absent from the
/// tree, or whatever [`ops::update_content`] returns for a missing content
/// target.
pub(super) fn apply_splice(
    tree: &mut BlockTree,
    removed: &[Block],
    inserted: &[Block],
    links: &[LinkChange],
    contents: &[ContentChange],
) -> PkmResult<()> {
    for block in removed {
        if tree.remove(block.id).is_none() {
            return Err(PkmError::BlockNotFound(block.id.to_string()));
        }
    }
    for block in inserted {
        tree.insert(block.clone());
    }
    for link in links {
        // A delta can target a block that a later step removed; skipping it
        // keeps replay tolerant without hiding a missing content target.
        if let Some(block) = tree.get_mut(link.block_id) {
            block.parent_id = link.new_parent;
            block.left_id = link.new_left;
        }
    }
    for change in contents {
        ops::update_content(tree, change.block_id, change.new.clone())?;
    }
    Ok(())
}
