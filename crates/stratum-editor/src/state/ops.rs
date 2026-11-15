//! Reversible edit operations and their application to a block tree.
//!
//! Every operation is a concrete forward/inverse pair. Structural operations
//! carry enough information to be replayed in both directions without a document
//! snapshot: a split records the new block identity plus the children that move
//! with it, and a merge records the deleted block's children so the inverse
//! split can restore the subtree.

use pkm_block::ops;
use pkm_block::{Block, BlockId, BlockTree, Priority, TaskMarker};
use pkm_core::{PkmError, PkmResult};

use super::splice::{apply_splice, ContentChange, LinkChange};
use super::text::clamp_offset;

/// One reversible edit operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditOp {
    InsertText {
        block_id: BlockId,
        offset: usize,
        text: String,
    },
    DeleteText {
        block_id: BlockId,
        offset: usize,
        text: String,
    },
    SplitBlock {
        block_id: BlockId,
        new_block_id: BlockId,
        new_parent: Option<BlockId>,
        new_left: Option<BlockId>,
        prefix: String,
        suffix: String,
        /// Children that move with the new block. Empty for a user Enter split
        /// (children stay with the original block); populated by the inverse of
        /// a merge so undo restores the deleted subtree.
        children: Vec<BlockId>,
    },
    MergeBlock {
        block_id: BlockId,
        prev_id: BlockId,
        prev_prefix: String,
        merged_text: String,
        block_parent: Option<BlockId>,
        block_left: Option<BlockId>,
        /// Children of `block_id` at merge time, so the inverse split can restore
        /// them.
        children: Vec<BlockId>,
    },
    Indent {
        block_id: BlockId,
        from_parent: Option<BlockId>,
        from_left: Option<BlockId>,
        to_parent: Option<BlockId>,
        to_left: Option<BlockId>,
    },
    Outdent {
        block_id: BlockId,
        from_parent: Option<BlockId>,
        from_left: Option<BlockId>,
        to_parent: Option<BlockId>,
        to_left: Option<BlockId>,
    },
    MoveBlock {
        block_id: BlockId,
        from_parent: Option<BlockId>,
        from_left: Option<BlockId>,
        to_parent: Option<BlockId>,
        to_left: Option<BlockId>,
    },
    SetMarker {
        block_id: BlockId,
        old: Option<TaskMarker>,
        new: Option<TaskMarker>,
    },
    SetPriority {
        block_id: BlockId,
        old: Option<Priority>,
        new: Option<Priority>,
    },
    SetProperty {
        block_id: BlockId,
        key: String,
        old: Option<String>,
        new: Option<String>,
    },
    SetContent {
        block_id: BlockId,
        old: String,
        new: String,
    },
    SetHeading {
        block_id: BlockId,
        old: Option<u8>,
        new: Option<u8>,
    },
    /// A block-range splice: remove `removed` snapshots, insert `inserted`
    /// snapshots (parents before children) and apply the explicit `links` and
    /// `contents` deltas. Cross-block delete and multi-block paste use this so
    /// the whole range operation is a single undoable history entry.
    SpliceBlocks {
        removed: Vec<Block>,
        inserted: Vec<Block>,
        links: Vec<LinkChange>,
        contents: Vec<ContentChange>,
    },
}

impl EditOp {
    /// The inverse operation for undo/redo.
    pub fn inverse(&self) -> EditOp {
        match self {
            EditOp::InsertText {
                block_id,
                offset,
                text,
            } => EditOp::DeleteText {
                block_id: *block_id,
                offset: *offset,
                text: text.clone(),
            },
            EditOp::DeleteText {
                block_id,
                offset,
                text,
            } => EditOp::InsertText {
                block_id: *block_id,
                offset: *offset,
                text: text.clone(),
            },
            EditOp::SplitBlock {
                block_id,
                new_block_id,
                new_parent,
                new_left,
                prefix,
                suffix,
                children,
            } => EditOp::MergeBlock {
                block_id: *new_block_id,
                prev_id: *block_id,
                prev_prefix: prefix.clone(),
                merged_text: suffix.clone(),
                block_parent: *new_parent,
                block_left: *new_left,
                children: children.clone(),
            },
            EditOp::MergeBlock {
                block_id,
                prev_id,
                prev_prefix,
                merged_text,
                block_parent,
                block_left,
                children,
            } => EditOp::SplitBlock {
                block_id: *prev_id,
                new_block_id: *block_id,
                new_parent: *block_parent,
                new_left: *block_left,
                prefix: prev_prefix.clone(),
                suffix: merged_text.clone(),
                children: children.clone(),
            },
            EditOp::Indent {
                block_id,
                from_parent,
                from_left,
                to_parent,
                to_left,
            } => EditOp::Outdent {
                block_id: *block_id,
                from_parent: *to_parent,
                from_left: *to_left,
                to_parent: *from_parent,
                to_left: *from_left,
            },
            EditOp::Outdent {
                block_id,
                from_parent,
                from_left,
                to_parent,
                to_left,
            } => EditOp::Indent {
                block_id: *block_id,
                from_parent: *to_parent,
                from_left: *to_left,
                to_parent: *from_parent,
                to_left: *from_left,
            },
            EditOp::MoveBlock {
                block_id,
                from_parent,
                from_left,
                to_parent,
                to_left,
            } => EditOp::MoveBlock {
                block_id: *block_id,
                from_parent: *to_parent,
                from_left: *to_left,
                to_parent: *from_parent,
                to_left: *from_left,
            },
            EditOp::SetMarker { block_id, old, new } => EditOp::SetMarker {
                block_id: *block_id,
                old: *new,
                new: *old,
            },
            EditOp::SetPriority { block_id, old, new } => EditOp::SetPriority {
                block_id: *block_id,
                old: *new,
                new: *old,
            },
            EditOp::SetProperty {
                block_id,
                key,
                old,
                new,
            } => EditOp::SetProperty {
                block_id: *block_id,
                key: key.clone(),
                old: new.clone(),
                new: old.clone(),
            },
            EditOp::SetContent { block_id, old, new } => EditOp::SetContent {
                block_id: *block_id,
                old: new.clone(),
                new: old.clone(),
            },
            EditOp::SetHeading { block_id, old, new } => EditOp::SetHeading {
                block_id: *block_id,
                old: *new,
                new: *old,
            },
            EditOp::SpliceBlocks {
                removed,
                inserted,
                links,
                contents,
            } => EditOp::SpliceBlocks {
                removed: inserted.clone(),
                inserted: removed.clone(),
                links: links.iter().map(LinkChange::swapped).collect(),
                contents: contents.iter().map(ContentChange::swapped).collect(),
            },
        }
    }
}

/// Apply one operation to `tree`.
pub(super) fn apply_op(tree: &mut BlockTree, op: &EditOp) -> PkmResult<()> {
    match op {
        EditOp::InsertText {
            block_id,
            offset,
            text,
        } => {
            let content = tree
                .get(*block_id)
                .ok_or_else(|| PkmError::BlockNotFound(block_id.to_string()))?
                .content
                .clone();
            let at = clamp_offset(&content, *offset);
            let new = format!("{}{}{}", &content[..at], text, &content[at..]);
            ops::update_content(tree, *block_id, new)
        }
        EditOp::DeleteText {
            block_id,
            offset,
            text,
        } => {
            let content = tree
                .get(*block_id)
                .ok_or_else(|| PkmError::BlockNotFound(block_id.to_string()))?
                .content
                .clone();
            let at = clamp_offset(&content, *offset);
            let end = (at + text.len()).min(content.len());
            let new = format!("{}{}", &content[..at], &content[end..]);
            ops::update_content(tree, *block_id, new)
        }
        EditOp::SplitBlock {
            block_id,
            new_block_id,
            new_parent,
            new_left,
            prefix,
            suffix,
            children,
        } => {
            ops::update_content(tree, *block_id, prefix.clone())?;
            let mut block = Block::new(*new_block_id, suffix.clone());
            block.parent_id = *new_parent;
            block.left_id = *new_left;
            ops::insert_block(tree, block);
            link_after(tree, *new_left, *new_block_id);
            // Re-home any children that travel with the new block. Their left
            // chain stays intact because they were already siblings.
            for child in children {
                if let Some(block) = tree.get_mut(*child) {
                    block.parent_id = Some(*new_block_id);
                }
            }
            Ok(())
        }
        EditOp::MergeBlock {
            block_id,
            prev_id,
            prev_prefix,
            merged_text,
            block_parent: _,
            block_left: _,
            children: _,
        } => {
            let merged = format!("{prev_prefix}{merged_text}");
            ops::update_content(tree, *prev_id, merged)?;
            // Append the merged block's children to the surviving previous
            // block, preserving their order. The op's `children` field is the
            // restore list for the inverse split; the live tree is authoritative
            // for the forward direction.
            let moved = child_ids(tree, *block_id);
            let mut last = tree.last_child(*prev_id).map(|b| b.id);
            for child in moved {
                if let Some(block) = tree.get_mut(child) {
                    block.parent_id = Some(*prev_id);
                    block.left_id = last;
                }
                last = Some(child);
            }
            ops::delete_block(tree, *block_id)?;
            Ok(())
        }
        EditOp::Indent {
            block_id,
            to_parent,
            to_left,
            ..
        }
        | EditOp::Outdent {
            block_id,
            to_parent,
            to_left,
            ..
        }
        | EditOp::MoveBlock {
            block_id,
            to_parent,
            to_left,
            ..
        } => ops::move_block(tree, *block_id, *to_parent, *to_left),
        EditOp::SetMarker { block_id, new, .. } => {
            let block = tree
                .get_mut(*block_id)
                .ok_or_else(|| PkmError::BlockNotFound(block_id.to_string()))?;
            block.marker = *new;
            Ok(())
        }
        EditOp::SetPriority { block_id, new, .. } => {
            let block = tree
                .get_mut(*block_id)
                .ok_or_else(|| PkmError::BlockNotFound(block_id.to_string()))?;
            block.priority = *new;
            Ok(())
        }
        EditOp::SetProperty {
            block_id, key, new, ..
        } => match new {
            Some(value) => ops::set_property(tree, *block_id, key, value),
            None => ops::remove_property(tree, *block_id, key),
        },
        EditOp::SetContent { block_id, new, .. } => {
            ops::update_content(tree, *block_id, new.clone())
        }
        EditOp::SetHeading { block_id, new, .. } => {
            let block = tree
                .get_mut(*block_id)
                .ok_or_else(|| PkmError::BlockNotFound(block_id.to_string()))?;
            block.meta.heading_level = *new;
            Ok(())
        }
        EditOp::SpliceBlocks {
            removed,
            inserted,
            links,
            contents,
        } => apply_splice(tree, removed, inserted, links, contents),
    }
}

/// Children of `block_id` in sibling order.
pub(super) fn child_ids(tree: &BlockTree, block_id: BlockId) -> Vec<BlockId> {
    let mut children: Vec<BlockId> = tree.children(block_id).iter().map(|b| b.id).collect();
    // `children` returns storage order; walk the left chain from the first
    // child so a restored subtree keeps its original sibling order.
    if children.len() > 1 {
        let mut ordered = Vec::with_capacity(children.len());
        let mut cursor = children
            .iter()
            .find(|id| tree.get(**id).and_then(|b| b.left_id).is_none())
            .copied();
        while let Some(id) = cursor {
            ordered.push(id);
            cursor = children
                .iter()
                .find(|other| tree.get(**other).and_then(|b| b.left_id) == Some(id))
                .copied();
        }
        if ordered.len() == children.len() {
            children = ordered;
        }
    }
    children
}

/// Point the sibling that followed `after` at `new_id`.
pub(super) fn link_after(tree: &mut BlockTree, after: Option<BlockId>, new_id: BlockId) {
    let next = tree
        .all_blocks()
        .find(|b| b.left_id == after && b.id != new_id)
        .map(|b| b.id);
    if let Some(next) = next {
        if let Some(block) = tree.get_mut(next) {
            block.left_id = Some(new_id);
        }
    }
}
