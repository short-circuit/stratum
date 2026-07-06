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

    // Find the block that currently has left_id == after_id (i.e., was after `after_id`)
    // and update its left_id to point to the new block
    let next_id = find_block_with_left_id(tree, after_id);
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

    // Collect all descendants to delete
    let subtree_ids: Vec<BlockId> = tree.subtree(block_id).iter().map(|b| b.id).collect();

    // Fix the left_id chain: the block after this one should now point to this one's left
    let next_id = find_block_with_left_id(tree, Some(block_id));
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
    }

    // Check for cycles: can't move a block under itself
    if let Some(pid) = new_parent_id {
        if tree.ancestors(pid).iter().any(|b| b.id == block_id) {
            return Err(PkmError::CycleDetected("Operation would create a cycle".to_string()));
        }
    }
    if block_id == new_left_id.unwrap_or_default() || block_id == new_parent_id.unwrap_or_default()
    {
        // Moving to self as parent
        if new_parent_id == Some(block_id) {
            return Err(PkmError::CycleDetected("Operation would create a cycle".to_string()));
        }
    }

    // Close gap in old position
    let old_block = tree.get(block_id).unwrap();
    let old_left = old_block.left_id;
    let next_old = find_block_with_left_id(tree, Some(block_id));
    if let Some(next_id) = next_old {
        if let Some(next) = tree.get_mut(next_id) {
            next.left_id = old_left;
        }
    }

    // Close gap in new position: the block after new_left_id should point to block_id
    let next_new = find_block_with_left_id(tree, new_left_id);
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
        None => return Err(PkmError::Internal("Invalid position for insertion".to_string())), // Can't indent first child
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
        None => return Err(PkmError::Internal("Invalid position for insertion".to_string())), // Root can't outdent
    };

    move_block(tree, block_id, grandparent_id, parent_id)
}

/// Split a block at the cursor position, creating a new sibling below.
pub fn split_block(
    tree: &mut BlockTree,
    block_id: BlockId,
    split_pos: usize,
) -> Result<BlockId, PkmError> {
    let block = tree.get(block_id).ok_or(PkmError::BlockNotFound(block_id.to_string()))?;
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
        None => return Err(PkmError::Internal("Invalid position for insertion".to_string())), // No previous sibling
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
pub fn toggle_task(tree: &mut BlockTree, block_id: BlockId) -> Result<Option<TaskMarker>, PkmError> {
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

/// Find the block whose left_id matches the given id.
fn find_block_with_left_id(tree: &BlockTree, left_id: Option<BlockId>) -> Option<BlockId> {
    for block in tree.all_blocks() {
        if block.left_id == left_id {
            return Some(block.id);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::BlockTree;

    #[test]
    fn test_create_root_block() {
        let mut tree = BlockTree::new();
        let id = create_block(&mut tree, None, None, "First".into()).unwrap();
        assert_eq!(tree.len(), 1);
        assert_eq!(tree.get(id).unwrap().content, "First");
        assert!(tree.get(id).unwrap().is_root());
    }

    #[test]
    fn test_create_child_block() {
        let mut tree = BlockTree::new();
        let root = create_block(&mut tree, None, None, "Root".into()).unwrap();
        let child = create_block(&mut tree, Some(root), None, "Child".into()).unwrap();

        assert_eq!(tree.children(root).len(), 1);
        assert_eq!(tree.children(root)[0].id, child);
        assert_eq!(tree.parent(child).unwrap().id, root);
    }

    #[test]
    fn test_create_after_existing() {
        let mut tree = BlockTree::new();
        let a = create_block(&mut tree, None, None, "A".into()).unwrap();
        let b = create_block(&mut tree, None, Some(a), "B".into()).unwrap();

        let roots = tree.roots();
        assert_eq!(roots.len(), 2);
        assert_eq!(roots[0].id, a);
        assert_eq!(roots[1].id, b);
    }

    #[test]
    fn test_create_with_nonexistent_parent_fails() {
        let mut tree = BlockTree::new();
        let result = create_block(&mut tree, Some(Uuid::new_v4()), None, "X".into());
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_block() {
        let mut tree = BlockTree::new();
        let a = create_block(&mut tree, None, None, "A".into()).unwrap();
        let b = create_block(&mut tree, None, Some(a), "B".into()).unwrap();

        let deleted = delete_block(&mut tree, a).unwrap();
        assert_eq!(deleted.len(), 1);
        assert_eq!(tree.len(), 1);
        assert!(!tree.contains(a));
        assert!(tree.contains(b));
    }

    #[test]
    fn test_delete_block_with_children() {
        let mut tree = BlockTree::new();
        let root = create_block(&mut tree, None, None, "Root".into()).unwrap();
        let child = create_block(&mut tree, Some(root), None, "Child".into()).unwrap();
        let _gc = create_block(&mut tree, Some(child), None, "GC".into()).unwrap();

        let deleted = delete_block(&mut tree, root).unwrap();
        assert_eq!(deleted.len(), 3);
        assert!(tree.is_empty());
    }

    #[test]
    fn test_move_block() {
        let mut tree = BlockTree::new();
        let root = create_block(&mut tree, None, None, "Root".into()).unwrap();
        let a = create_block(&mut tree, None, None, "A".into()).unwrap();
        let b = create_block(&mut tree, Some(root), None, "B".into()).unwrap();

        move_block(&mut tree, a, Some(root), Some(b)).unwrap();

        let children = tree.children(root);
        assert_eq!(children.len(), 2);
        assert!(children.iter().any(|child| child.id == a));
        assert!(children.iter().any(|child| child.id == b));
    }

    #[test]
    fn test_move_cycle_prevention() {
        let mut tree = BlockTree::new();
        let root = create_block(&mut tree, None, None, "Root".into()).unwrap();
        let child = create_block(&mut tree, Some(root), None, "Child".into()).unwrap();

        // Can't move root under child
        let result = move_block(&mut tree, root, Some(child), None);
        assert!(result.is_err());

        // Can't move block under itself
        let result = move_block(&mut tree, root, Some(root), None);
        assert!(result.is_err());
    }

    #[test]
    fn test_indent_block() {
        let mut tree = BlockTree::new();
        let a = create_block(&mut tree, None, None, "A".into()).unwrap();
        let b = create_block(&mut tree, None, Some(a), "B".into()).unwrap();

        indent_block(&mut tree, b).unwrap();

        assert_eq!(tree.parent(b).unwrap().id, a);
    }

    #[test]
    fn test_indent_first_child_fails() {
        let mut tree = BlockTree::new();
        let a = create_block(&mut tree, None, None, "A".into()).unwrap();

        let result = indent_block(&mut tree, a);
        assert!(result.is_err());
    }

    #[test]
    fn test_outdent_block() {
        let mut tree = BlockTree::new();
        let root = create_block(&mut tree, None, None, "Root".into()).unwrap();
        let child = create_block(&mut tree, Some(root), None, "Child".into()).unwrap();
        let gc = create_block(&mut tree, Some(child), None, "GC".into()).unwrap();

        outdent_block(&mut tree, gc).unwrap();

        // GC should now be a child of root, sibling of child
        assert_eq!(tree.parent(gc).unwrap().id, root);
        let children = tree.children(root);
        assert_eq!(children.len(), 2);
    }

    #[test]
    fn test_outdent_root_fails() {
        let mut tree = BlockTree::new();
        let root = create_block(&mut tree, None, None, "Root".into()).unwrap();

        let result = outdent_block(&mut tree, root);
        assert!(result.is_err());
    }

    #[test]
    fn test_split_block() {
        let mut tree = BlockTree::new();
        let a = create_block(&mut tree, None, None, "Hello World".into()).unwrap();

        let b = split_block(&mut tree, a, 6).unwrap();

        assert_eq!(tree.get(a).unwrap().content, "Hello ");
        assert_eq!(tree.get(b).unwrap().content, "World");
    }

    #[test]
    fn test_merge_with_previous() {
        let mut tree = BlockTree::new();
        let a = create_block(&mut tree, None, None, "Hello".into()).unwrap();
        let b = create_block(&mut tree, None, Some(a), " World".into()).unwrap();

        merge_with_previous(&mut tree, b).unwrap();

        assert_eq!(tree.len(), 1);
        assert_eq!(tree.get(a).unwrap().content, "Hello World");
        assert!(!tree.contains(b));
    }

    #[test]
    fn test_toggle_task() {
        let mut tree = BlockTree::new();
        let id = create_block(&mut tree, None, None, "Item".into()).unwrap();

        let marker = toggle_task(&mut tree, id).unwrap();
        assert_eq!(marker, Some(TaskMarker::Todo));

        let marker = toggle_task(&mut tree, id).unwrap();
        assert_eq!(marker, Some(TaskMarker::Doing));

        let marker = toggle_task(&mut tree, id).unwrap();
        assert_eq!(marker, Some(TaskMarker::Done));

        let marker = toggle_task(&mut tree, id).unwrap();
        assert_eq!(marker, None);
    }

    #[test]
    fn test_toggle_collapsed() {
        let mut tree = BlockTree::new();
        let id = create_block(&mut tree, None, None, "Parent".into()).unwrap();
        create_block(&mut tree, Some(id), None, "Child".into()).unwrap();

        let collapsed = toggle_collapsed(&mut tree, id).unwrap();
        assert!(collapsed);

        let collapsed = toggle_collapsed(&mut tree, id).unwrap();
        assert!(!collapsed);
    }

    #[test]
    fn test_update_content() {
        let mut tree = BlockTree::new();
        let id = create_block(&mut tree, None, None, "Original".into()).unwrap();

        update_content(&mut tree, id, "Updated".into()).unwrap();
        assert_eq!(tree.get(id).unwrap().content, "Updated");
    }

    #[test]
    fn test_set_and_remove_property() {
        let mut tree = BlockTree::new();
        let id = create_block(&mut tree, None, None, "Block".into()).unwrap();

        set_property(&mut tree, id, "type", "meeting").unwrap();
        assert_eq!(
            tree.get(id).unwrap().properties.get("type").unwrap(),
            "meeting"
        );

        remove_property(&mut tree, id, "type").unwrap();
        assert!(!tree.get(id).unwrap().properties.contains_key("type"));
    }
}
