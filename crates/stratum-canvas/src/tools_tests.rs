//! Tool state-machine tests split out of `tools.rs` to keep that file under
//! the repository length limit.

#[cfg(test)]
mod tests {
    use crate::*;

    fn point(x: f64, y: f64) -> PointerEvent {
        PointerEvent {
            point: Point::new(x, y),
            screen: Point::new(x, y),
            modifiers: Modifiers::default(),
        }
    }

    #[test]
    fn shape_drag_creates_and_selects() {
        let mut canvas = CanvasState::new(BoardDocument::empty());
        canvas.set_tool(Tool::Rectangle);
        canvas.pointer_down(point(10.0, 10.0));
        canvas.pointer_move(point(60.0, 40.0));
        canvas.pointer_up(point(60.0, 40.0));
        assert_eq!(canvas.tool(), Tool::Select);
        assert_eq!(canvas.selection().len(), 1);
        let element = canvas.document().element(&canvas.selection()[0]).unwrap();
        assert_eq!(element.width(), 50.0);
        assert_eq!(element.height(), 30.0);
    }

    #[test]
    fn select_ids_ignores_unknown_ids() {
        let mut canvas = CanvasState::new(crate::fixture::sample_board());
        let action = canvas.select_ids(&["fixture-rect".to_string(), "missing".to_string()]);
        assert_eq!(action, CanvasAction::SelectionChanged);
        assert_eq!(canvas.selection(), ["fixture-rect".to_string()]);
        let current = canvas.selection().to_vec();
        assert_eq!(canvas.select_ids(&current), CanvasAction::None);
    }

    #[test]
    fn eraser_soft_deletes() {
        let mut canvas = CanvasState::new(BoardDocument::empty());
        canvas.set_tool(Tool::Rectangle);
        canvas.pointer_down(point(0.0, 0.0));
        canvas.pointer_move(point(50.0, 50.0));
        canvas.pointer_up(point(50.0, 50.0));
        let id = canvas.selection()[0].clone();
        canvas.set_tool(Tool::Eraser);
        canvas.pointer_down(point(10.0, 10.0));
        canvas.pointer_up(point(10.0, 10.0));
        assert!(canvas.document().element(&id).unwrap().is_deleted());
        assert_eq!(canvas.document().visible_elements().count(), 0);
    }
}
