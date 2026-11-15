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

#[test]
fn move_root_to_head_does_not_relink_other_groups_first_child() {
    let mut tree = BlockTree::new();
    let r1 = create_block(&mut tree, None, None, "R1".into()).unwrap();
    let r2 = create_block(&mut tree, None, Some(r1), "R2".into()).unwrap();
    let moving = create_block(&mut tree, None, Some(r2), "Moving".into()).unwrap();
    let p1 = create_block(&mut tree, None, Some(moving), "P1".into()).unwrap();
    let p2 = create_block(&mut tree, None, Some(p1), "P2".into()).unwrap();
    let c1 = create_block(&mut tree, Some(p1), None, "C1".into()).unwrap();
    let c2 = create_block(&mut tree, Some(p2), None, "C2".into()).unwrap();

    // Move the root block to index 0 of the root group. Both unrelated parent
    // groups already have a `left_id == None` first child; neither may be
    // relinked to the moved block.
    move_block(&mut tree, moving, None, None).unwrap();

    assert_eq!(tree.get(c1).unwrap().left_id, None);
    assert_eq!(tree.get(c2).unwrap().left_id, None);
    assert_eq!(tree.children(p1)[0].id, c1);
    assert_eq!(tree.children(p2)[0].id, c2);
    assert_eq!(tree.roots()[0].id, moving);
    assert_eq!(tree.roots()[1].id, r1);
}

#[test]
fn move_block_rejects_left_id_from_a_different_parent() {
    let mut tree = BlockTree::new();
    let parent = create_block(&mut tree, None, None, "Parent".into()).unwrap();
    create_block(&mut tree, Some(parent), None, "Child".into()).unwrap();
    let r1 = create_block(&mut tree, None, None, "R1".into()).unwrap();
    let r2 = create_block(&mut tree, None, Some(r1), "R2".into()).unwrap();

    let result = move_block(&mut tree, r2, Some(parent), Some(r1));

    assert!(matches!(result, Err(PkmError::Validation(_))));
    assert_eq!(tree.get(r2).unwrap().parent_id, None);
    assert_eq!(tree.get(r2).unwrap().left_id, Some(r1));
}

#[test]
fn create_first_child_leaves_other_group_heads_untouched() {
    let mut tree = BlockTree::new();
    let p1 = create_block(&mut tree, None, None, "P1".into()).unwrap();
    let p2 = create_block(&mut tree, None, Some(p1), "P2".into()).unwrap();
    let c1 = create_block(&mut tree, Some(p1), None, "C1".into()).unwrap();
    let c2 = create_block(&mut tree, Some(p2), None, "C2".into()).unwrap();

    let new = create_block(&mut tree, Some(p1), None, "New".into()).unwrap();

    assert_eq!(tree.children(p1)[0].id, new);
    assert_eq!(tree.get(c1).unwrap().left_id, Some(new));
    // The other group's head must still point nowhere.
    assert_eq!(tree.get(c2).unwrap().left_id, None);
    assert_eq!(tree.children(p2)[0].id, c2);
}

#[test]
fn delete_closes_the_chain_inside_the_deleted_blocks_group_only() {
    let mut tree = BlockTree::new();
    let r1 = create_block(&mut tree, None, None, "R1".into()).unwrap();
    let r2 = create_block(&mut tree, None, Some(r1), "R2".into()).unwrap();
    let r3 = create_block(&mut tree, None, Some(r2), "R3".into()).unwrap();
    let parent = create_block(&mut tree, None, Some(r3), "Parent".into()).unwrap();
    let x = create_block(&mut tree, Some(parent), None, "X".into()).unwrap();
    let y = create_block(&mut tree, Some(parent), Some(x), "Y".into()).unwrap();
    let z = create_block(&mut tree, Some(parent), Some(y), "Z".into()).unwrap();

    delete_block(&mut tree, y).unwrap();

    assert_eq!(tree.get(z).unwrap().left_id, Some(x));
    let children: Vec<_> = tree.children(parent).iter().map(|b| b.id).collect();
    assert_eq!(children, vec![x, z]);
    let roots: Vec<_> = tree.roots().iter().map(|b| b.id).collect();
    assert_eq!(roots, vec![r1, r2, r3, parent]);
}
