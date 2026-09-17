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
