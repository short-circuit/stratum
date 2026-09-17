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
