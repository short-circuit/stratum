//! Unit tests for destination-group block moves.

use pkm_block::Block;
use uuid::Uuid;

use super::*;
use crate::document::EditorDocument;

fn doc_with(blocks: Vec<Block>) -> EditorDocument {
    EditorDocument::from_blocks("pages/x.md", blocks).unwrap()
}

fn block(id: Uuid, content: &str) -> Block {
    Block::new(id, content.to_string())
}

fn sibling(id: Uuid, content: &str, left: Option<Uuid>) -> Block {
    let mut block = block(id, content);
    block.left_id = left;
    block
}

fn child(id: Uuid, content: &str, parent: Uuid, left: Option<Uuid>) -> Block {
    let mut block = block(id, content);
    block.parent_id = Some(parent);
    block.left_id = left;
    block
}

#[test]
fn reorder_to_head_updates_the_left_chain() {
    let (a, b, c) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    let mut state = EditorState::new(doc_with(vec![
        block(a, "A"),
        sibling(b, "B", Some(a)),
        sibling(c, "C", Some(b)),
    ]));

    assert!(state
        .move_block(
            c,
            MoveTarget {
                parent: None,
                index: 0
            }
        )
        .unwrap());

    assert_eq!(state.document().ordered_ids(), vec![c, a, b]);
    assert_eq!(state.document().block(c).unwrap().left_id, None);
    assert_eq!(state.document().block(a).unwrap().left_id, Some(c));
    assert_eq!(state.document().block(b).unwrap().left_id, Some(a));
}

#[test]
fn reorder_to_the_middle_updates_the_left_chain() {
    let (a, b, c, d) = (
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
    );
    let mut state = EditorState::new(doc_with(vec![
        block(a, "A"),
        sibling(b, "B", Some(a)),
        sibling(c, "C", Some(b)),
        sibling(d, "D", Some(c)),
    ]));

    assert!(state
        .move_block(
            a,
            MoveTarget {
                parent: None,
                index: 2
            }
        )
        .unwrap());

    assert_eq!(state.document().ordered_ids(), vec![b, c, a, d]);
    assert_eq!(state.document().block(b).unwrap().left_id, None);
    assert_eq!(state.document().block(c).unwrap().left_id, Some(b));
    assert_eq!(state.document().block(a).unwrap().left_id, Some(c));
    assert_eq!(state.document().block(d).unwrap().left_id, Some(a));
}

#[test]
fn reorder_to_the_end_updates_the_left_chain() {
    let (a, b, c) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    let mut state = EditorState::new(doc_with(vec![
        block(a, "A"),
        sibling(b, "B", Some(a)),
        sibling(c, "C", Some(b)),
    ]));

    assert!(state
        .move_block(
            a,
            MoveTarget {
                parent: None,
                index: 3
            }
        )
        .unwrap());

    assert_eq!(state.document().ordered_ids(), vec![b, c, a]);
    assert_eq!(state.document().block(b).unwrap().left_id, None);
    assert_eq!(state.document().block(c).unwrap().left_id, Some(b));
    assert_eq!(state.document().block(a).unwrap().left_id, Some(c));
}

#[test]
fn move_to_another_parent_carries_the_subtree() {
    let (p1, p2, moved, grandchild, existing) = (
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
    );
    let mut state = EditorState::new(doc_with(vec![
        block(p1, "P1"),
        sibling(p2, "P2", Some(p1)),
        child(moved, "Moved", p1, None),
        child(grandchild, "Grandchild", moved, None),
        child(existing, "Existing", p2, None),
    ]));

    assert!(state
        .move_block(
            moved,
            MoveTarget {
                parent: Some(p2),
                index: 1,
            },
        )
        .unwrap());

    assert_eq!(state.document().block(moved).unwrap().parent_id, Some(p2));
    assert_eq!(
        state.document().block(moved).unwrap().left_id,
        Some(existing)
    );
    // The subtree travels with the moved block.
    assert_eq!(
        state.document().block(grandchild).unwrap().parent_id,
        Some(moved)
    );
    assert_eq!(
        state.document().ordered_ids(),
        vec![p1, p2, existing, moved, grandchild]
    );
}

#[test]
fn move_into_a_descendant_is_rejected_without_history() {
    let (root, child_id, grandchild) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    let mut state = EditorState::new(doc_with(vec![
        block(root, "Root"),
        child(child_id, "Child", root, None),
        child(grandchild, "Grandchild", child_id, None),
    ]));
    let before = state.document().ordered_ids();

    let result = state.move_block(
        root,
        MoveTarget {
            parent: Some(grandchild),
            index: 0,
        },
    );

    assert!(matches!(result, Err(PkmError::CycleDetected(_))));
    assert_eq!(state.document().ordered_ids(), before);
    assert_eq!(state.history_depth(), 0);
}

#[test]
fn move_to_the_current_position_is_a_no_op() {
    let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
    let mut state = EditorState::new(doc_with(vec![block(a, "A"), sibling(b, "B", Some(a))]));

    assert!(!state
        .move_block(
            b,
            MoveTarget {
                parent: None,
                index: 1
            }
        )
        .unwrap());
    assert!(!state
        .move_block(
            a,
            MoveTarget {
                parent: None,
                index: 0
            }
        )
        .unwrap());
    assert_eq!(state.history_depth(), 0);
    assert_eq!(state.document().ordered_ids(), vec![a, b]);
}

#[test]
fn undo_and_redo_replay_a_reorder_as_one_entry() {
    let (a, b, c) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    let mut state = EditorState::new(doc_with(vec![
        block(a, "A"),
        sibling(b, "B", Some(a)),
        sibling(c, "C", Some(b)),
    ]));

    assert!(state
        .move_block(
            c,
            MoveTarget {
                parent: None,
                index: 0
            }
        )
        .unwrap());
    assert_eq!(state.history_depth(), 1);
    assert_eq!(state.document().ordered_ids(), vec![c, a, b]);

    assert!(state.undo().unwrap());
    assert_eq!(state.history_depth(), 0);
    assert_eq!(state.document().ordered_ids(), vec![a, b, c]);
    assert_eq!(state.document().block(c).unwrap().parent_id, None);
    assert_eq!(state.document().block(c).unwrap().left_id, Some(b));
    assert_eq!(state.document().block(b).unwrap().left_id, Some(a));
    assert_eq!(state.document().block(a).unwrap().left_id, None);

    assert!(state.redo().unwrap());
    assert_eq!(state.history_depth(), 1);
    assert_eq!(state.document().ordered_ids(), vec![c, a, b]);
    assert_eq!(state.document().block(c).unwrap().left_id, None);
    assert_eq!(state.document().block(a).unwrap().left_id, Some(c));
    assert_eq!(state.document().block(b).unwrap().left_id, Some(a));
}

#[test]
fn moving_a_root_to_the_head_keeps_other_group_heads_intact() {
    let (r1, r2, moving, p1, p2, c1, c2) = (
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
    );
    let mut state = EditorState::new(doc_with(vec![
        block(r1, "R1"),
        sibling(r2, "R2", Some(r1)),
        sibling(moving, "Moving", Some(r2)),
        sibling(p1, "P1", Some(moving)),
        sibling(p2, "P2", Some(p1)),
        child(c1, "C1", p1, None),
        child(c2, "C2", p2, None),
    ]));

    assert!(state
        .move_block(
            moving,
            MoveTarget {
                parent: None,
                index: 0
            }
        )
        .unwrap());

    assert_eq!(state.document().block(c1).unwrap().left_id, None);
    assert_eq!(state.document().block(c2).unwrap().left_id, None);
    assert_eq!(state.document().block(c1).unwrap().parent_id, Some(p1));
    assert_eq!(state.document().block(c2).unwrap().parent_id, Some(p2));
    assert_eq!(state.document().ordered_ids()[0], moving);
}
