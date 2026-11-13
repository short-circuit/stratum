//! Tool behaviour tests for [`stratum_canvas::CanvasState`].
//!
//! Covers shape/frame creation, soft delete, duplicate, nudge snapping, text
//! editing, and the undo/redo journal including the redo-cleared rule.

use stratum_canvas::{
    BoardDocument, CanvasAction, CanvasState, ElementKind, Modifiers, Point, PointerEvent,
    SnapConfig, Tool,
};

/// Build a pointer event with no modifiers.
fn point(x: f64, y: f64) -> PointerEvent {
    PointerEvent {
        point: Point::new(x, y),
        screen: Point::new(x, y),
        modifiers: Modifiers::default(),
    }
}

/// Drag-create a shape with the given tool and return the new element id.
fn create_shape(canvas: &mut CanvasState, tool: Tool, end: (f64, f64)) -> String {
    canvas.set_tool(tool);
    canvas.pointer_down(point(0.0, 0.0));
    canvas.pointer_move(point(end.0, end.1));
    canvas.pointer_up(point(end.0, end.1));
    canvas.selection()[0].clone()
}

#[test]
fn shape_tools_are_single_shot_and_select_the_result() {
    let mut canvas = CanvasState::new(BoardDocument::empty());
    let id = create_shape(&mut canvas, Tool::Rectangle, (50.0, 30.0));
    assert_eq!(canvas.tool(), Tool::Select);
    assert_eq!(canvas.selection(), std::slice::from_ref(&id));
    let element = canvas.document().element(&id).unwrap();
    assert_eq!(element.kind(), ElementKind::Rectangle);
    assert_eq!((element.width(), element.height()), (50.0, 30.0));
}

#[test]
fn undo_and_redo_restore_creation() {
    let mut canvas = CanvasState::new(BoardDocument::empty());
    let id = create_shape(&mut canvas, Tool::Ellipse, (40.0, 40.0));
    assert!(canvas.document().element(&id).is_some());

    assert_eq!(canvas.undo(), CanvasAction::DocumentChanged);
    assert!(canvas.document().element(&id).is_none());

    assert_eq!(canvas.redo(), CanvasAction::DocumentChanged);
    assert!(canvas.document().element(&id).is_some());
    assert_eq!(
        canvas.document().element(&id).unwrap().kind(),
        ElementKind::Ellipse
    );
}

#[test]
fn a_new_mutation_clears_the_redo_stack() {
    let mut canvas = CanvasState::new(BoardDocument::empty());
    let first = create_shape(&mut canvas, Tool::Rectangle, (20.0, 20.0));
    canvas.undo();
    assert!(canvas.can_redo());

    let second = create_shape(&mut canvas, Tool::Diamond, (30.0, 30.0));
    assert!(!canvas.can_redo(), "new mutation must clear redo");
    assert_ne!(first, second);

    assert_eq!(canvas.redo(), CanvasAction::None);
    assert!(canvas.document().element(&first).is_none());
}

#[test]
fn delete_is_a_soft_delete_and_undo_restores_visibility() {
    let mut canvas = CanvasState::new(BoardDocument::empty());
    let id = create_shape(&mut canvas, Tool::Rectangle, (50.0, 50.0));
    assert_eq!(canvas.delete_selection(), CanvasAction::DocumentChanged);
    assert!(canvas.document().element(&id).unwrap().is_deleted());
    assert_eq!(canvas.document().visible_elements().count(), 0);

    assert_eq!(canvas.undo(), CanvasAction::DocumentChanged);
    assert!(!canvas.document().element(&id).unwrap().is_deleted());
    assert_eq!(canvas.document().visible_elements().count(), 1);
}

#[test]
fn duplicate_offsets_the_copy_and_undo_removes_it() {
    let mut canvas = CanvasState::new(BoardDocument::empty());
    let id = create_shape(&mut canvas, Tool::Rectangle, (40.0, 40.0));
    assert_eq!(canvas.duplicate_selection(), CanvasAction::DocumentChanged);
    assert_eq!(canvas.document().elements().len(), 2);
    let copy = canvas.selection()[0].clone();
    assert_ne!(copy, id);
    let original = canvas.document().element(&id).unwrap();
    let duplicated = canvas.document().element(&copy).unwrap();
    assert_eq!(duplicated.x(), original.x() + 10.0);
    assert_eq!(duplicated.y(), original.y() + 10.0);

    assert_eq!(canvas.undo(), CanvasAction::DocumentChanged);
    assert!(canvas.document().element(&copy).is_none());
    assert!(canvas.document().element(&id).is_some());
}

#[test]
fn nudge_applies_grid_snapping_and_undo_reverts_it() {
    let mut canvas = CanvasState::new(BoardDocument::empty());
    canvas.set_snap(SnapConfig {
        grid: Some(20.0),
        ..SnapConfig::default()
    });
    let id = create_shape(&mut canvas, Tool::Rectangle, (40.0, 40.0));
    let start = canvas.document().element(&id).unwrap().x();
    assert_eq!(
        canvas.nudge_selection(13.0, 13.0),
        CanvasAction::DocumentChanged
    );
    let element = canvas.document().element(&id).unwrap();
    assert_eq!(element.x(), 20.0);
    assert_eq!(element.y(), 20.0);
    assert_eq!(element.x().rem_euclid(20.0), 0.0);

    canvas.undo();
    assert_eq!(canvas.document().element(&id).unwrap().x(), start);
}

#[test]
fn group_and_ungroup_are_reversible() {
    let mut canvas = CanvasState::new(BoardDocument::empty());
    let first = create_shape(&mut canvas, Tool::Rectangle, (30.0, 30.0));
    let second = create_shape(&mut canvas, Tool::Ellipse, (80.0, 80.0));
    canvas.select_all();
    assert_eq!(canvas.group_selection(), CanvasAction::DocumentChanged);
    let group = canvas.document().element(&first).unwrap().group_ids();
    assert_eq!(group.len(), 1);
    assert_eq!(
        canvas.document().element(&second).unwrap().group_ids(),
        group
    );

    canvas.undo();
    assert!(canvas
        .document()
        .element(&first)
        .unwrap()
        .group_ids()
        .is_empty());
    assert!(canvas
        .document()
        .element(&second)
        .unwrap()
        .group_ids()
        .is_empty());
}

#[test]
fn frame_creation_names_and_selects_the_frame() {
    let mut canvas = CanvasState::new(BoardDocument::empty());
    let id = create_shape(&mut canvas, Tool::Frame, (320.0, 240.0));
    assert_eq!(canvas.tool(), Tool::Select);
    let frame = canvas.document().element(&id).unwrap();
    assert_eq!(frame.kind(), ElementKind::Frame);
    assert_eq!(frame.frame_name().as_deref(), Some("Frame"));

    canvas.rename_frame(&id, "Architecture").unwrap();
    assert_eq!(
        canvas
            .document()
            .element(&id)
            .unwrap()
            .frame_name()
            .as_deref(),
        Some("Architecture")
    );
    canvas.undo();
    assert_eq!(
        canvas
            .document()
            .element(&id)
            .unwrap()
            .frame_name()
            .as_deref(),
        Some("Frame")
    );
}

#[test]
fn text_editing_commits_and_undo_removes_the_new_element() {
    let mut canvas = CanvasState::new(BoardDocument::empty());
    canvas.set_tool(Tool::Text);
    let action = canvas.pointer_down(point(10.0, 10.0));
    let CanvasAction::TextEditing(id) = action else {
        panic!("expected a text edit session, got {action:?}");
    };
    canvas.update_text("Hello");
    assert_eq!(canvas.text_editing(), Some("Hello"));
    assert_eq!(canvas.commit_text(), CanvasAction::DocumentChanged);
    assert_eq!(
        canvas.document().element(&id).unwrap().text().as_deref(),
        Some("Hello")
    );
    assert_eq!(canvas.selection(), std::slice::from_ref(&id));

    assert_eq!(canvas.undo(), CanvasAction::DocumentChanged);
    assert!(canvas.document().element(&id).is_none());
}

#[test]
fn arrow_polyline_finishes_with_two_points() {
    let mut canvas = CanvasState::new(BoardDocument::empty());
    canvas.set_tool(Tool::Arrow);
    canvas.pointer_down(point(0.0, 0.0));
    canvas.pointer_down(point(100.0, 50.0));
    assert_eq!(canvas.finish_linear(), CanvasAction::DocumentChanged);
    let id = canvas.selection()[0].clone();
    let arrow = canvas.document().element(&id).unwrap();
    assert_eq!(arrow.kind(), ElementKind::Arrow);
    assert_eq!(arrow.points().len(), 2);
    assert!(arrow.end_arrowhead().is_some());
    assert_eq!((arrow.width(), arrow.height()), (100.0, 50.0));
}

#[test]
fn undo_is_a_no_op_on_a_clean_canvas() {
    let mut canvas = CanvasState::new(BoardDocument::empty());
    assert_eq!(canvas.undo(), CanvasAction::None);
    assert_eq!(canvas.redo(), CanvasAction::None);
    assert!(!canvas.can_undo());
    assert!(!canvas.can_redo());
}
