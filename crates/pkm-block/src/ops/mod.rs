use crate::block::{Block, BlockId, TaskMarker};
use crate::tree::BlockTree;
use chrono::Utc;
use pkm_core::PkmError;
use uuid::Uuid;

/// Create a new block and append it after `after_id` (None = first child of parent, or first root).
pub fn create_block(
    tree: &mut BlockTree,
    parent_id: Option<BlockId>,
    after_id: Option<BlockId>,
    content: String,
) -> Result<BlockId, PkmError> {
    if let Some(pid) = parent_id {
        if !tree.contains(pid) {
            return Err(PkmError::NotFound(format!("Parent block not found: {pid}")));
        }
    }
    if let Some(aid) = after_id {
        if !tree.contains(aid) {
            return Err(PkmError::BlockNotFound(aid.to_string()));
        }
        // `after_id` is a sibling pointer: it must belong to the same parent
        // group as the new block, otherwise relinking would cross parents.
        if tree.get(aid).and_then(|block| block.parent_id) != parent_id {
            return Err(PkmError::Validation(format!(
                "left block is not a child of the target parent: {aid}"
            )));
        }
    }

    let id = Uuid::new_v4();
    let new_block = Block {
        id,
        content,
        parent_id,
        left_id: after_id,
        properties: Default::default(),
        marker: None,
        priority: None,
        meta: Default::default(),
        created_at: Utc::now(),
        modified_at: Utc::now(),
    };

    // Relink the sibling that currently follows `after_id` inside the *same*
    // parent group. `left_id == None` is a head pointer local to a group, so the
    // successor lookup must never scan another parent's siblings.
    let next_id = successor_in_group(tree, parent_id, after_id, None);
    if let Some(next_block_id) = next_id {
        if let Some(next_block) = tree.get_mut(next_block_id) {
            next_block.left_id = Some(id);
        }
    }

    tree.insert(new_block);
    Ok(id)
}

/// Insert an existing block into the tree (e.g., from parsing).
pub fn insert_block(tree: &mut BlockTree, block: Block) {
    tree.insert(block);
}

/// Delete a block and all its descendants.
pub fn delete_block(tree: &mut BlockTree, block_id: BlockId) -> Result<Vec<BlockId>, PkmError> {
    if !tree.contains(block_id) {
        return Err(PkmError::BlockNotFound(block_id.to_string()));
    }

    let block = tree.get(block_id).unwrap();
    let left_id = block.left_id;
    let parent_id = block.parent_id;

    // Collect all descendants to delete
    let subtree_ids: Vec<BlockId> = tree.subtree(block_id).iter().map(|b| b.id).collect();

    // Fix the left_id chain inside the deleted block's own sibling group: the
    // block after this one should now point to this one's left.
    let next_id = successor_in_group(tree, parent_id, Some(block_id), None);
    if let Some(next_block_id) = next_id {
        if let Some(next_block) = tree.get_mut(next_block_id) {
            next_block.left_id = left_id;
        }
    }

    // Remove all blocks in subtree
    for id in &subtree_ids {
        tree.remove(*id);
    }

    Ok(subtree_ids)
}

/// Move a block (and its subtree) to a new parent, after a given sibling.
pub fn move_block(
    tree: &mut BlockTree,
    block_id: BlockId,
    new_parent_id: Option<BlockId>,
    new_left_id: Option<BlockId>,
) -> Result<(), PkmError> {
    if !tree.contains(block_id) {
        return Err(PkmError::BlockNotFound(block_id.to_string()));
    }
    if let Some(pid) = new_parent_id {
        if !tree.contains(pid) {
            return Err(PkmError::NotFound(format!("Parent block not found: {pid}")));
        }
    }
    if let Some(lid) = new_left_id {
        if !tree.contains(lid) {
            return Err(PkmError::BlockNotFound(lid.to_string()));
        }
        // The destination predecessor must be a sibling of the moved block in
        // its new group. Rejecting cross-parent pointers here keeps the
        // `left_id` chain consistent before any relinking happens.
        if lid == block_id || tree.get(lid).and_then(|block| block.parent_id) != new_parent_id {
            return Err(PkmError::Validation(format!(
                "left block is not a child of the target parent: {lid}"
            )));
        }
    }

    // Check for cycles: can't move a block under itself
    if let Some(pid) = new_parent_id {
        if tree.ancestors(pid).iter().any(|b| b.id == block_id) {
            return Err(PkmError::CycleDetected(
                "Operation would create a cycle".to_string(),
            ));
        }
    }
    if block_id == new_left_id.unwrap_or_default() || block_id == new_parent_id.unwrap_or_default()
    {
        // Moving to self as parent
        if new_parent_id == Some(block_id) {
            return Err(PkmError::CycleDetected(
                "Operation would create a cycle".to_string(),
            ));
        }
    }

    // Close the gap in the old group. The successor lookup is scoped to the
    // block's current parent so an unrelated group's head cannot be relinked.
    let old_block = tree.get(block_id).unwrap();
    let old_left = old_block.left_id;
    let old_parent = old_block.parent_id;
    let next_old = successor_in_group(tree, old_parent, Some(block_id), Some(block_id));
    if let Some(next_id) = next_old {
        if let Some(next) = tree.get_mut(next_id) {
            next.left_id = old_left;
        }
    }

    // Open the gap in the destination group: the sibling after `new_left_id`
    // should point to the moved block. Excluding the moved block keeps a
    // same-parent move from relinking itself.
    let next_new = successor_in_group(tree, new_parent_id, new_left_id, Some(block_id));
    if let Some(next_id) = next_new {
        if let Some(next) = tree.get_mut(next_id) {
            next.left_id = Some(block_id);
        }
    }

    // Update the moved block
    if let Some(block) = tree.get_mut(block_id) {
        block.parent_id = new_parent_id;
        block.left_id = new_left_id;
        block.modified_at = Utc::now();
    }

    Ok(())
}

/// Indent a block (make it a child of the previous sibling).
pub fn indent_block(tree: &mut BlockTree, block_id: BlockId) -> Result<(), PkmError> {
    if !tree.contains(block_id) {
        return Err(PkmError::BlockNotFound(block_id.to_string()));
    }

    let left_id = tree.get(block_id).and_then(|b| b.left_id);
    let new_parent = match left_id {
        Some(left) => left,
        None => {
            return Err(PkmError::Internal(
                "Invalid position for insertion".to_string(),
            ))
        } // Can't indent first child
    };

    // Find the last child of the new parent (to place after it)
    let last_child = tree.last_child(new_parent);
    let new_left = last_child.map(|b| b.id);

    move_block(tree, block_id, Some(new_parent), new_left)
}

/// Outdent a block (promote it to its parent's level, after its parent).
pub fn outdent_block(tree: &mut BlockTree, block_id: BlockId) -> Result<(), PkmError> {
    if !tree.contains(block_id) {
        return Err(PkmError::BlockNotFound(block_id.to_string()));
    }

    let parent_id = tree.get(block_id).and_then(|b| b.parent_id);
    let grandparent_id = match parent_id {
        Some(pid) => tree.get(pid).and_then(|b| b.parent_id),
        None => {
            return Err(PkmError::Internal(
                "Invalid position for insertion".to_string(),
            ))
        } // Root can't outdent
    };

    move_block(tree, block_id, grandparent_id, parent_id)
}

/// Split a block at the cursor position, creating a new sibling below.
pub fn split_block(
    tree: &mut BlockTree,
    block_id: BlockId,
    split_pos: usize,
) -> Result<BlockId, PkmError> {
    let block = tree
        .get(block_id)
        .ok_or(PkmError::BlockNotFound(block_id.to_string()))?;
    let original = block.content.clone();
    let parent = block.parent_id;

    let (keep, new_content) = if split_pos >= original.len() {
        (original, String::new())
    } else {
        let (a, b) = original.split_at(split_pos);
        (a.to_string(), b.to_string())
    };

    // Update current block
    if let Some(block) = tree.get_mut(block_id) {
        block.content = keep;
        block.modified_at = Utc::now();
    }

    // Create new block after this one
    create_block(tree, parent, Some(block_id), new_content)
}

/// Merge a block with its previous sibling.
pub fn merge_with_previous(tree: &mut BlockTree, block_id: BlockId) -> Result<(), PkmError> {
    if !tree.contains(block_id) {
        return Err(PkmError::BlockNotFound(block_id.to_string()));
    }

    let prev_id = tree.prev_sibling(block_id);
    let prev_id = match prev_id {
        Some(id) => id,
        None => {
            return Err(PkmError::Internal(
                "Invalid position for insertion".to_string(),
            ))
        } // No previous sibling
    };

    let block_content = tree.get(block_id).unwrap().content.clone();

    // Append this block's content to previous sibling
    if let Some(prev) = tree.get_mut(prev_id) {
        prev.content.push_str(&block_content);
        prev.modified_at = Utc::now();
    }

    // Move any children of the deleted block to the previous sibling
    let children: Vec<BlockId> = tree.children(block_id).iter().map(|b| b.id).collect();
    for child_id in &children {
        let last_child = tree.last_child(prev_id);
        let new_left = last_child.map(|b| b.id);
        if let Some(child) = tree.get_mut(*child_id) {
            child.parent_id = Some(prev_id);
            child.left_id = new_left;
        }
    }

    delete_block(tree, block_id)?;
    Ok(())
}

/// Cycle a block's task marker through TODO → DOING → DONE → (clear).
pub fn toggle_task(
    tree: &mut BlockTree,
    block_id: BlockId,
) -> Result<Option<TaskMarker>, PkmError> {
    let block = tree
        .get_mut(block_id)
        .ok_or(PkmError::BlockNotFound(block_id.to_string()))?;
    let new_marker = match block.marker {
        None => Some(TaskMarker::Todo),
        Some(TaskMarker::Todo) => Some(TaskMarker::Doing),
        Some(TaskMarker::Doing) => Some(TaskMarker::Done),
        Some(TaskMarker::Done) => None,
        Some(other) => Some(other), // Preserve custom markers
    };
    block.marker = new_marker;
    block.modified_at = Utc::now();
    Ok(new_marker)
}

/// Toggle collapse state of a block (if it has children).
pub fn toggle_collapsed(tree: &mut BlockTree, block_id: BlockId) -> Result<bool, PkmError> {
    let block = tree
        .get_mut(block_id)
        .ok_or(PkmError::BlockNotFound(block_id.to_string()))?;
    block.meta.collapsed = !block.meta.collapsed;
    block.modified_at = Utc::now();
    Ok(block.meta.collapsed)
}

/// Update block content.
pub fn update_content(
    tree: &mut BlockTree,
    block_id: BlockId,
    content: String,
) -> Result<(), PkmError> {
    let block = tree
        .get_mut(block_id)
        .ok_or(PkmError::BlockNotFound(block_id.to_string()))?;
    block.content = content;
    block.modified_at = Utc::now();
    Ok(())
}

/// Set a property on a block.
pub fn set_property(
    tree: &mut BlockTree,
    block_id: BlockId,
    key: &str,
    value: &str,
) -> Result<(), PkmError> {
    let block = tree
        .get_mut(block_id)
        .ok_or(PkmError::BlockNotFound(block_id.to_string()))?;
    block.properties.insert(key.to_string(), value.to_string());
    block.modified_at = Utc::now();
    Ok(())
}

/// Remove a property from a block.
pub fn remove_property(tree: &mut BlockTree, block_id: BlockId, key: &str) -> Result<(), PkmError> {
    let block = tree
        .get_mut(block_id)
        .ok_or(PkmError::BlockNotFound(block_id.to_string()))?;
    block.properties.remove(key);
    block.modified_at = Utc::now();
    Ok(())
}

/// Siblings of `parent_id` in deterministic document order (`children()` for a
/// block, `roots()` for `None`).
///
/// Sibling relinking must stay inside one parent group: `left_id == None` is a
/// head pointer *within* a group, so scanning every block in the tree lets a
/// first child of an unrelated parent answer a query about another group's head
/// and get relinked across parents.
fn group(tree: &BlockTree, parent_id: Option<BlockId>) -> Vec<&Block> {
    match parent_id {
        Some(parent) => tree.children(parent),
        None => tree.roots(),
    }
}

/// The sibling inside `parent_id`'s group that currently follows `left_id`.
///
/// `exclude` skips a block that must not be relinked (the block being moved).
/// The scan is group-scoped so a `left_id == None` query can only resolve to the
/// head of the requested group.
fn successor_in_group(
    tree: &BlockTree,
    parent_id: Option<BlockId>,
    left_id: Option<BlockId>,
    exclude: Option<BlockId>,
) -> Option<BlockId> {
    group(tree, parent_id)
        .into_iter()
        .find(|block| block.left_id == left_id && Some(block.id) != exclude)
        .map(|block| block.id)
}

#[cfg(test)]
mod tests;
