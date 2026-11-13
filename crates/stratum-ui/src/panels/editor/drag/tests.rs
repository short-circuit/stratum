//! Window-free tests for the drag geometry and drop-target resolution.

use ::core::prelude::v1::test;
use gpui_kit::{point, px};
use stratum_editor::{
    dto_to_blocks, BlockDto, DecoratedRow, EditorDocument, EditorState, MoveTarget, VirtualLayout,
};
use uuid::Uuid;

use super::*;
use crate::panels::editor::build_rows_for_state;
use crate::panels::editor::rows::{row_text_left, ROW_DEPTH_INDENT, ROW_WIDTH};

fn dto(id: Uuid, content: &str, parent: Option<Uuid>, left: Option<Uuid>) -> BlockDto {
    BlockDto {
        id: id.to_string(),
        content: content.to_string(),
        parent_id: parent.map(|parent| parent.to_string()),
        left_id: left.map(|left| left.to_string()),
        properties: Vec::new(),
        marker: None,
        priority: None,
        collapsed: false,
        heading_level: None,
    }
}

fn state_with(blocks: Vec<BlockDto>) -> EditorState {
    EditorState::new(
        EditorDocument::from_blocks("pages/x.md", dto_to_blocks(blocks)).expect("document"),
    )
}

fn rows_for(state: &EditorState) -> Vec<DecoratedRow> {
    build_rows_for_state(state, false)
}

fn row_of(rows: &[DecoratedRow], id: Uuid) -> usize {
    rows.iter()
        .position(|row| row.block_id == id)
        .expect("row for block")
}

#[test]
fn gutter_hit_test_is_left_of_the_row_text() {
    let (parent, nested) = (Uuid::new_v4(), Uuid::new_v4());
    let state = state_with(vec![
        dto(parent, "Parent", None, None),
        dto(nested, "Nested", Some(parent), None),
    ]);
    let rows = rows_for(&state);
    let layout = VirtualLayout::from_rows(&rows);
    let index = row_of(&rows, nested);
    let depth = rows[index].depth;
    let y = layout.offsets()[index] + 1.0;

    assert_eq!(
        gutter_row_at(&rows, &layout, row_text_left(depth) - 1.0, y),
        Some(index)
    );
    assert_eq!(
        gutter_row_at(&rows, &layout, row_text_left(depth) + 1.0, y),
        None
    );
}

#[test]
fn drop_zone_boundaries_split_the_row_into_quarters() {
    assert_eq!(drop_zone(100.0, 0.0), DropZone::Above);
    assert_eq!(drop_zone(100.0, 24.0), DropZone::Above);
    assert_eq!(drop_zone(100.0, 25.0), DropZone::Into);
    assert_eq!(drop_zone(100.0, 74.0), DropZone::Into);
    assert_eq!(drop_zone(100.0, 75.0), DropZone::Below);
    assert_eq!(drop_zone(100.0, 99.0), DropZone::Below);
}

#[test]
fn drop_targets_resolve_against_the_destination_group() {
    let (parent, first, grandchild, second, dragged) = (
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
    );
    let state = state_with(vec![
        dto(parent, "Parent", None, None),
        dto(first, "First", Some(parent), None),
        dto(grandchild, "Grandchild", Some(first), None),
        dto(second, "Second", Some(parent), Some(first)),
        dto(dragged, "Dragged", None, Some(parent)),
    ]);
    let rows = rows_for(&state);
    let first_row = row_of(&rows, first);

    let above = drop_target_for(&state, &rows, first_row, DropZone::Above, dragged).unwrap();
    assert_eq!(above.parent, Some(parent));
    assert_eq!(above.index, 0);
    assert_eq!(above.zone, DropZone::Above);
    assert_eq!(above.row_index, first_row);

    let below = drop_target_for(&state, &rows, first_row, DropZone::Below, dragged).unwrap();
    assert_eq!(below.parent, Some(parent));
    assert_eq!(below.index, 1);

    let into = drop_target_for(&state, &rows, first_row, DropZone::Into, dragged).unwrap();
    assert_eq!(into.parent, Some(first));
    assert_eq!(into.index, 1);
}

#[test]
fn drop_targets_are_suppressed_for_cycles_and_no_ops() {
    let (parent, first, grandchild, second) = (
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
    );
    let state = state_with(vec![
        dto(parent, "Parent", None, None),
        dto(first, "First", Some(parent), None),
        dto(grandchild, "Grandchild", Some(first), None),
        dto(second, "Second", Some(parent), Some(first)),
    ]);
    let rows = rows_for(&state);

    // The hovered row is the dragged block itself for Above/Below.
    assert_eq!(
        drop_target_for(&state, &rows, row_of(&rows, first), DropZone::Above, first),
        None
    );
    // Dropping the parent into its own descendant is a cycle.
    assert_eq!(
        drop_target_for(
            &state,
            &rows,
            row_of(&rows, grandchild),
            DropZone::Into,
            parent
        ),
        None
    );
    // First is already the first child of Parent.
    assert_eq!(
        drop_target_for(&state, &rows, row_of(&rows, second), DropZone::Above, first),
        None
    );
}

#[test]
fn indicator_rect_follows_depth_and_row_edge() {
    let (parent, nested, grandchild) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    let state = state_with(vec![
        dto(parent, "Parent", None, None),
        dto(nested, "Nested", Some(parent), None),
        dto(grandchild, "Grandchild", Some(nested), None),
    ]);
    let rows = rows_for(&state);
    let layout = VirtualLayout::from_rows(&rows);
    let index = row_of(&rows, grandchild);
    let depth = rows[index].depth;
    let top = layout.offsets()[index];
    let height = layout.heights()[index];

    let sibling_target = DropTarget {
        parent: Some(nested),
        index: 0,
        row_index: index,
        zone: DropZone::Above,
    };
    let (x, y, width) = drop_indicator_rect(&rows, &layout, &sibling_target).unwrap();
    assert_eq!(x, row_text_left(depth));
    assert_eq!(y, top);
    assert_eq!(width, (ROW_WIDTH - 12.0 - x).max(40.0));

    let into_target = DropTarget {
        parent: Some(grandchild),
        index: 0,
        row_index: index,
        zone: DropZone::Into,
    };
    let (x, y, width) = drop_indicator_rect(&rows, &layout, &into_target).unwrap();
    assert_eq!(x, row_text_left(depth) + ROW_DEPTH_INDENT);
    assert_eq!(y, top + height);
    assert_eq!(width, (ROW_WIDTH - 12.0 - x).max(40.0));
}

#[test]
fn drag_threshold_requires_real_pointer_travel() {
    let origin = point(px(10.0), px(10.0));
    assert!(!drag_has_started(origin, point(px(13.0), px(10.0))));
    assert!(!drag_has_started(origin, point(px(12.0), px(12.0))));
    assert!(drag_has_started(origin, point(px(15.0), px(10.0))));
    assert!(drag_has_started(origin, point(px(13.0), px(14.0))));
}

#[test]
fn resolved_drop_target_drives_the_engine_move() {
    let (a, b, c) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    let mut state = state_with(vec![
        dto(a, "A", None, None),
        dto(b, "B", None, Some(a)),
        dto(c, "C", None, Some(b)),
    ]);
    let rows = rows_for(&state);
    let target = drop_target_for(&state, &rows, row_of(&rows, c), DropZone::Below, a).unwrap();

    assert_eq!(target.parent, None);
    assert_eq!(target.index, 2);
    assert!(state
        .move_block(
            a,
            MoveTarget {
                parent: target.parent,
                index: target.index,
            },
        )
        .unwrap());
    assert_eq!(state.document().ordered_ids(), vec![b, c, a]);
}
