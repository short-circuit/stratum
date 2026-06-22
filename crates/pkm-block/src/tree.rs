use crate::block::{Block, BlockId};
use std::collections::{HashMap, HashSet};
#[allow(unused_imports)]
use uuid::Uuid;

/// A tree of blocks using `parent_id` + `left_id` for O(1) sibling ordering.
///
/// This is Logseq's model:
/// - `parent_id` defines hierarchy (None = root block)
/// - `left_id` defines sibling ordering (None = first child in parent order)
///
/// Moving a subtree only requires updating the moved node's `parent_id` and `left_id`,
/// plus the `left_id` of the node that was after it (to close the gap).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct BlockTree {
    blocks: HashMap<BlockId, Block>,
    root_order: Vec<BlockId>, // cached root-level ordering
    dirty: bool,
}

impl BlockTree {
    pub fn new() -> Self {
        Self {
            blocks: HashMap::new(),
            root_order: Vec::new(),
            dirty: false,
        }
    }

    pub fn insert(&mut self, block: Block) {
        self.blocks.insert(block.id, block);
        self.dirty = true;
    }

    pub fn remove(&mut self, id: BlockId) -> Option<Block> {
        let removed = self.blocks.remove(&id);
        self.dirty = true;
        removed
    }

    pub fn get(&self, id: BlockId) -> Option<&Block> {
        self.blocks.get(&id)
    }

    pub fn get_mut(&mut self, id: BlockId) -> Option<&mut Block> {
        self.blocks.get_mut(&id)
    }

    pub fn contains(&self, id: BlockId) -> bool {
        self.blocks.contains_key(&id)
    }

    pub fn len(&self) -> usize {
        self.blocks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }

    pub fn all_blocks(&self) -> impl Iterator<Item = &Block> {
        self.blocks.values()
    }

    pub fn iter_ids(&self) -> impl Iterator<Item = &BlockId> {
        self.blocks.keys()
    }

    /// Get root blocks in display order (via left_id linked list).
    pub fn roots(&self) -> Vec<&Block> {
        self.collect_siblings(None)
    }

    /// Get children of a block in display order.
    pub fn children(&self, parent_id: BlockId) -> Vec<&Block> {
        self.collect_siblings(Some(parent_id))
    }

    /// Get the first child of a block.
    pub fn first_child(&self, parent_id: BlockId) -> Option<&Block> {
        self.blocks
            .values()
            .find(|b| b.parent_id == Some(parent_id) && b.left_id.is_none())
    }

    /// Get the last child of a block.
    pub fn last_child(&self, parent_id: BlockId) -> Option<&Block> {
        let children: Vec<&Block> = self.children(parent_id);
        children.into_iter().last()
    }

    /// Get the parent block.
    pub fn parent(&self, child_id: BlockId) -> Option<&Block> {
        self.get(child_id)
            .and_then(|b| b.parent_id)
            .and_then(|pid| self.get(pid))
    }

    /// Get the next sibling (block to the right).
    pub fn next_sibling(&self, block_id: BlockId) -> Option<&Block> {
        self.blocks
            .values()
            .find(|b| b.left_id == Some(block_id))
    }

    /// Get the previous sibling (block to the left).
    pub fn prev_sibling(&self, block_id: BlockId) -> Option<BlockId> {
        self.get(block_id).and_then(|b| b.left_id)
    }

    /// Get all descendants of a block (including itself).
    pub fn subtree(&self, root_id: BlockId) -> Vec<&Block> {
        let mut result = Vec::new();
        let mut queue = vec![root_id];
        while let Some(id) = queue.pop() {
            if let Some(block) = self.get(id) {
                result.push(block);
                for child in self.children(id) {
                    queue.push(child.id);
                }
            }
        }
        result
    }

    /// Get all ancestors of a block (parent chain, including self).
    pub fn ancestors(&self, block_id: BlockId) -> Vec<&Block> {
        let mut result = Vec::new();
        let mut current = block_id;
        while let Some(block) = self.get(current) {
            result.push(block);
            match block.parent_id {
                Some(pid) => current = pid,
                None => break,
            }
        }
        result.reverse();
        result
    }

    /// Get the depth of a block (0 for roots).
    pub fn depth(&self, block_id: BlockId) -> usize {
        let mut depth = 0;
        let mut current = block_id;
        while let Some(block) = self.get(current) {
            match block.parent_id {
                Some(pid) => {
                    depth += 1;
                    current = pid;
                }
                None => break,
            }
        }
        depth
    }

    /// Get all blocks in depth-first traversal order.
    pub fn depth_first_order(&self) -> Vec<&Block> {
        let mut result = Vec::new();
        for root in self.roots() {
            self.collect_depth_first(root.id, &mut result);
        }
        result
    }

    fn collect_depth_first<'a>(&'a self, block_id: BlockId, result: &mut Vec<&'a Block>) {
        if let Some(block) = self.get(block_id) {
            result.push(block);
            for child in self.children(block_id) {
                self.collect_depth_first(child.id, result);
            }
        }
    }

    /// Check if any block in the tree has a given task marker.
    pub fn find_by_marker(&self, marker: crate::block::TaskMarker) -> Vec<&Block> {
        self.blocks
            .values()
            .filter(|b| b.marker == Some(marker))
            .collect()
    }

    /// Check if any block has a given property set.
    pub fn find_by_property(&self, key: &str, value: &str) -> Vec<&Block> {
        self.blocks
            .values()
            .filter(|b| b.properties.get(key).map(|v| v.as_str()) == Some(value))
            .collect()
    }

    /// Build a set of all block IDs in the tree.
    pub fn id_set(&self) -> HashSet<BlockId> {
        self.blocks.keys().copied().collect()
    }

    /// Collect siblings in order for a given parent (None = roots).
    fn collect_siblings(&self, parent_id: Option<BlockId>) -> Vec<&Block> {
        let children: Vec<&Block> = self
            .blocks
            .values()
            .filter(|b| b.parent_id == parent_id)
            .collect();

        if children.is_empty() {
            return Vec::new();
        }

        // Find the head (block with no left_id for this parent)
        let mut head = children
            .iter()
            .find(|b| b.left_id.is_none())
            .copied();

        let mut result = Vec::new();
        let mut seen = HashSet::new();

        while let Some(block) = head {
            if !seen.insert(block.id) {
                break; // cycle detection
            }
            result.push(block);
            // Find next block whose left_id is this block's id
            head = children
                .iter()
                .find(|b| b.left_id == Some(block.id))
                .copied();
        }

        // Fallback: if left_id chain is broken, add remaining siblings in insertion order
        if result.len() < children.len() {
            let result_ids: HashSet<BlockId> = result.iter().map(|b| b.id).collect();
            for child in &children {
                if !result_ids.contains(&child.id) {
                    result.push(child);
                }
            }
        }

        result
    }

    /// Rebuild root ordering cache (call after structural changes).
    pub fn rebuild_cache(&mut self) {
        self.root_order = self.roots().iter().map(|b| b.id).collect();
        self.dirty = false;
    }

    /// Convert the tree into a sorted vec of blocks (for iteration in order).
    pub fn into_sorted_vec(self) -> Vec<Block> {
        let mut result = Vec::with_capacity(self.blocks.len());

        let root_ids: Vec<BlockId> = {
            let roots = self
                .blocks
                .values()
                .filter(|b| b.parent_id.is_none())
                .collect::<Vec<_>>();

            let mut seen = HashSet::new();
            let mut head = roots.iter().find(|b| b.left_id.is_none()).map(|b| b.id);
            let mut ordered = Vec::new();
            while let Some(id) = head {
                if seen.insert(id) {
                    ordered.push(id);
                }
                head = roots
                    .iter()
                    .find(|b| b.left_id == Some(id))
                    .map(|b| b.id);
            }
            for root in &roots {
                if !seen.contains(&root.id) {
                    ordered.push(root.id);
                }
            }
            ordered
        };

        let mut seen = HashSet::new();
        for root_id in root_ids {
            let mut stack = vec![root_id];
            while let Some(id) = stack.pop() {
                if !seen.insert(id) {
                    continue;
                }
                if let Some(block) = self.blocks.get(&id) {
                    result.push(block.clone());
                    let children = self
                        .blocks
                        .values()
                        .filter(|b| b.parent_id == Some(id))
                        .collect::<Vec<_>>();
                    if !children.is_empty() {
                        let mut head = children
                            .iter()
                            .find(|b| b.left_id.is_none())
                            .map(|b| b.id);
                        let mut ordered_children = Vec::new();
                        let mut child_seen = HashSet::new();
                        while let Some(cid) = head {
                            if child_seen.insert(cid) {
                                ordered_children.push(cid);
                            }
                            head = children
                                .iter()
                                .find(|b| b.left_id == Some(cid))
                                .map(|b| b.id);
                        }
                        for child in &children {
                            if !child_seen.contains(&child.id) {
                                ordered_children.push(child.id);
                            }
                        }
                        ordered_children.reverse(); // reverse for stack (LIFO)
                        stack.extend(ordered_children);
                    }
                }
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::Block;

    fn make_block(id: BlockId, content: &str, parent: Option<BlockId>, left: Option<BlockId>) -> Block {
        let mut b = Block::new(id, content.to_string());
        b.parent_id = parent;
        b.left_id = left;
        b
    }

    #[test]
    fn test_empty_tree() {
        let tree = BlockTree::new();
        assert!(tree.is_empty());
        assert_eq!(tree.len(), 0);
        assert!(tree.roots().is_empty());
    }

    #[test]
    fn test_insert_and_get() {
        let mut tree = BlockTree::new();
        let id = Uuid::new_v4();
        let block = Block::new(id, "Hello".into());
        tree.insert(block.clone());
        assert_eq!(tree.len(), 1);
        assert_eq!(tree.get(id).unwrap().content, "Hello");
    }

    #[test]
    fn test_root_blocks() {
        let mut tree = BlockTree::new();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();

        // a <- b <- c (linked list of roots)
        tree.insert(make_block(a, "A", None, None));
        tree.insert(make_block(b, "B", None, Some(a)));
        tree.insert(make_block(c, "C", None, Some(b)));

        let roots = tree.roots();
        assert_eq!(roots.len(), 3);
        assert_eq!(roots[0].id, a);
        assert_eq!(roots[1].id, b);
        assert_eq!(roots[2].id, c);
    }

    #[test]
    fn test_children() {
        let mut tree = BlockTree::new();
        let root = Uuid::new_v4();
        let child_a = Uuid::new_v4();
        let child_b = Uuid::new_v4();

        tree.insert(make_block(root, "Root", None, None));
        tree.insert(make_block(child_a, "Child A", Some(root), None));
        tree.insert(make_block(child_b, "Child B", Some(root), Some(child_a)));

        let children = tree.children(root);
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].id, child_a);
        assert_eq!(children[1].id, child_b);
        assert_eq!(tree.first_child(root).unwrap().id, child_a);
        assert_eq!(tree.last_child(root).unwrap().id, child_b);
    }

    #[test]
    fn test_parent_and_siblings() {
        let mut tree = BlockTree::new();
        let root = Uuid::new_v4();
        let child_a = Uuid::new_v4();
        let child_b = Uuid::new_v4();

        tree.insert(make_block(root, "Root", None, None));
        tree.insert(make_block(child_a, "A", Some(root), None));
        tree.insert(make_block(child_b, "B", Some(root), Some(child_a)));

        assert_eq!(tree.parent(child_a).unwrap().id, root);
        assert_eq!(tree.parent(child_b).unwrap().id, root);
        assert_eq!(tree.next_sibling(child_a).unwrap().id, child_b);
        assert!(tree.next_sibling(child_b).is_none());
        assert_eq!(tree.prev_sibling(child_b), Some(child_a));
        assert!(tree.prev_sibling(child_a).is_none());
    }

    #[test]
    fn test_depth() {
        let mut tree = BlockTree::new();
        let root = Uuid::new_v4();
        let child = Uuid::new_v4();
        let grandchild = Uuid::new_v4();

        tree.insert(make_block(root, "Root", None, None));
        tree.insert(make_block(child, "Child", Some(root), None));
        tree.insert(make_block(grandchild, "Grandchild", Some(child), None));

        assert_eq!(tree.depth(root), 0);
        assert_eq!(tree.depth(child), 1);
        assert_eq!(tree.depth(grandchild), 2);
    }

    #[test]
    fn test_subtree() {
        let mut tree = BlockTree::new();
        let root = Uuid::new_v4();
        let child_a = Uuid::new_v4();
        let child_b = Uuid::new_v4();
        let grandchild = Uuid::new_v4();

        tree.insert(make_block(root, "Root", None, None));
        tree.insert(make_block(child_a, "A", Some(root), None));
        tree.insert(make_block(child_b, "B", Some(root), Some(child_a)));
        tree.insert(make_block(grandchild, "GC", Some(child_a), None));

        let subtree = tree.subtree(root);
        assert_eq!(subtree.len(), 4);
    }

    #[test]
    fn test_ancestors() {
        let mut tree = BlockTree::new();
        let root = Uuid::new_v4();
        let child = Uuid::new_v4();
        let grandchild = Uuid::new_v4();

        tree.insert(make_block(root, "Root", None, None));
        tree.insert(make_block(child, "Child", Some(root), None));
        tree.insert(make_block(grandchild, "GC", Some(child), None));

        let ancestors = tree.ancestors(grandchild);
        assert_eq!(ancestors.len(), 3);
        assert_eq!(ancestors[0].id, root);
        assert_eq!(ancestors[1].id, child);
        assert_eq!(ancestors[2].id, grandchild);
    }

    #[test]
    fn test_find_by_marker() {
        let mut tree = BlockTree::new();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();

        let block_a = Block::new(a, "TODO item".into()).with_marker(crate::block::TaskMarker::Todo);
        let block_b = Block::new(b, "Done item".into()).with_marker(crate::block::TaskMarker::Done);
        let block_c = Block::new(c, "Another TODO".into()).with_marker(crate::block::TaskMarker::Todo);

        tree.insert(block_a);
        tree.insert(block_b);
        tree.insert(block_c);

        let todos = tree.find_by_marker(crate::block::TaskMarker::Todo);
        assert_eq!(todos.len(), 2);
        let dones = tree.find_by_marker(crate::block::TaskMarker::Done);
        assert_eq!(dones.len(), 1);
    }

    #[test]
    fn test_find_by_property() {
        let mut tree = BlockTree::new();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();

        let block_a = Block::new(a, "Item".into()).with_property("type", "meeting");
        let block_b = Block::new(b, "Item 2".into()).with_property("type", "note");

        tree.insert(block_a);
        tree.insert(block_b);

        let meetings = tree.find_by_property("type", "meeting");
        assert_eq!(meetings.len(), 1);
        assert_eq!(meetings[0].id, a);
    }

    #[test]
    fn test_remove() {
        let mut tree = BlockTree::new();
        let id = Uuid::new_v4();
        tree.insert(Block::new(id, "Test".into()));
        assert_eq!(tree.len(), 1);
        let removed = tree.remove(id);
        assert!(removed.is_some());
        assert!(tree.is_empty());
    }

    #[test]
    fn test_depth_first_order() {
        let mut tree = BlockTree::new();
        let root = Uuid::new_v4();
        let child_a = Uuid::new_v4();
        let child_b = Uuid::new_v4();
        let gc = Uuid::new_v4();

        tree.insert(make_block(root, "Root", None, None));
        tree.insert(make_block(child_a, "A", Some(root), None));
        tree.insert(make_block(child_b, "B", Some(child_a), None));
        tree.insert(make_block(gc, "C", Some(root), Some(child_a)));

        let order: Vec<Uuid> = tree.depth_first_order().iter().map(|b| b.id).collect();
        assert_eq!(order[0], root);
        // A and its children should come before C (which follows A in sibling order)
        assert!(order.contains(&child_a));
        assert!(order.contains(&gc));
    }

    #[test]
    fn test_into_sorted_vec() {
        let mut tree = BlockTree::new();
        let root = Uuid::new_v4();
        let child = Uuid::new_v4();

        tree.insert(make_block(root, "Root", None, None));
        tree.insert(make_block(child, "Child", Some(root), None));

        let sorted = tree.into_sorted_vec();
        assert_eq!(sorted.len(), 2);
        assert_eq!(sorted[0].id, root);
        assert_eq!(sorted[1].id, child);
    }
}
