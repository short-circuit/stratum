//! Style edits, alignment and distribution for [`CanvasState`].
//!
//! Split out of `tools_edit.rs` to keep every source file under the 500-line
//! limit. The operations mirror the Excalidraw editor's selection actions:
//!
//! * [`StylePatch`] applies a partial style update to every selected element
//!   and records one undo delta per element.
//! * [`Align`] aligns the selection against its shared bounding box.
//! * [`Axis`] distributes the selection with equal gaps between elements.
//!
//! Locked and soft-deleted elements are ignored. Moving a selected frame also
//! moves its children unless a child is itself selected, in which case its own
//! target wins.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use crate::element::{Element, Style};
use crate::element_kind::{
    ElementKind, FillStyle, FontFamily, StrokeSharpness, StrokeStyle, TextAlign, VerticalAlign,
};
use crate::geometry::{element_rect, Rect};
use crate::tools::{CanvasAction, CanvasState};

/// Partial style update applied to every selected element.
///
/// Only the fields set to `Some` are written; everything else keeps the
/// element's current value. Applying an empty patch is a no-op.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StylePatch {
    /// Stroke colour (`strokeColor`).
    pub stroke_color: Option<String>,
    /// Fill colour (`backgroundColor`).
    pub background_color: Option<String>,
    /// Fill pattern (`fillStyle`).
    pub fill_style: Option<FillStyle>,
    /// Stroke width in world units (`strokeWidth`).
    pub stroke_width: Option<f64>,
    /// Stroke dash style (`strokeStyle`).
    pub stroke_style: Option<StrokeStyle>,
    /// Hand-drawn roughness (`roughness`).
    pub roughness: Option<f64>,
    /// Opacity percentage (`opacity`).
    pub opacity: Option<f64>,
    /// Corner sharpness; clears any explicit `roundness` when set.
    pub stroke_sharpness: Option<StrokeSharpness>,
    /// Font family (`fontFamily`).
    pub font_family: Option<FontFamily>,
    /// Font size in world units (`fontSize`).
    pub font_size: Option<f64>,
    /// Horizontal text alignment (`textAlign`).
    pub text_align: Option<TextAlign>,
    /// Vertical text alignment (`verticalAlign`).
    pub vertical_align: Option<VerticalAlign>,
    /// Line height multiplier (`lineHeight`).
    pub line_height: Option<f64>,
}

impl StylePatch {
    /// Whether the patch changes nothing.
    pub fn is_empty(&self) -> bool {
        self.stroke_color.is_none()
            && self.background_color.is_none()
            && self.fill_style.is_none()
            && self.stroke_width.is_none()
            && self.stroke_style.is_none()
            && self.roughness.is_none()
            && self.opacity.is_none()
            && self.stroke_sharpness.is_none()
            && self.font_family.is_none()
            && self.font_size.is_none()
            && self.text_align.is_none()
            && self.vertical_align.is_none()
            && self.line_height.is_none()
    }

    /// Apply the patch to a style. Returns whether the style changed.
    pub fn apply(&self, style: &mut Style) -> bool {
        let before = style.clone();
        if let Some(value) = &self.stroke_color {
            style.stroke_color = value.clone();
        }
        if let Some(value) = &self.background_color {
            style.background_color = value.clone();
        }
        if let Some(value) = self.fill_style {
            style.fill_style = value;
        }
        if let Some(value) = self.stroke_width {
            style.stroke_width = value;
        }
        if let Some(value) = self.stroke_style {
            style.stroke_style = value;
        }
        if let Some(value) = self.roughness {
            style.roughness = value;
        }
        if let Some(value) = self.opacity {
            style.opacity = value;
        }
        if let Some(value) = self.stroke_sharpness {
            style.stroke_sharpness = value;
            // The canonical `roundness` object wins over the legacy field, so
            // clear it whenever the patch explicitly picks a sharpness.
            style.roundness = None;
        }
        if let Some(value) = self.font_family {
            style.font_family = value;
        }
        if let Some(value) = self.font_size {
            style.font_size = value;
        }
        if let Some(value) = self.text_align {
            style.text_align = value;
        }
        if let Some(value) = self.vertical_align {
            style.vertical_align = value;
        }
        if let Some(value) = self.line_height {
            style.line_height = value;
        }
        *style != before
    }
}

/// Alignment target for [`CanvasState::align_selection`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    /// Align left edges to the selection's left edge.
    Left,
    /// Align horizontal centres to the selection's centre.
    CenterX,
    /// Align right edges to the selection's right edge.
    Right,
    /// Align top edges to the selection's top edge.
    Top,
    /// Align vertical centres to the selection's centre.
    CenterY,
    /// Align bottom edges to the selection's bottom edge.
    Bottom,
}

/// Axis for [`CanvasState::distribute_selection`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    /// Distribute left-to-right with equal horizontal gaps.
    Horizontal,
    /// Distribute top-to-bottom with equal vertical gaps.
    Vertical,
}

impl CanvasState {
    /// Apply a partial style update to every selected element.
    ///
    /// Each element update is recorded in the undo journal. Locked elements are
    /// skipped.
    pub fn set_selection_style(&mut self, patch: &StylePatch) -> CanvasAction {
        if patch.is_empty() {
            return CanvasAction::None;
        }
        let mut changed = false;
        for id in self.selection.clone() {
            let Some(current) = self.doc.element(&id).cloned() else {
                continue;
            };
            if current.locked() || current.is_deleted() {
                continue;
            }
            let mut style = current.style();
            if !patch.apply(&mut style) {
                continue;
            }
            let before = current.clone();
            if let Some(element) = self.doc.element_mut(&id) {
                element.set_style(&style);
            }
            let after = self.doc.element(&id).cloned().unwrap_or(current);
            self.record_update(&id, before, after);
            changed = true;
        }
        if changed {
            CanvasAction::DocumentChanged
        } else {
            CanvasAction::None
        }
    }

    /// Align the selected elements against their shared bounding box.
    ///
    /// Locked and deleted elements are ignored; a selection of fewer than two
    /// movable elements is a no-op. Moving a selected frame also moves its
    /// children.
    pub fn align_selection(&mut self, align: Align) -> CanvasAction {
        let rects = self.selected_rects();
        if rects.len() < 2 {
            return CanvasAction::None;
        }
        let union = rects
            .iter()
            .map(|(_, rect)| *rect)
            .reduce(|acc, rect| acc.union(&rect))
            .unwrap_or(Rect::new(0.0, 0.0, 0.0, 0.0));
        let mut deltas: BTreeMap<String, (f64, f64)> = BTreeMap::new();
        for (id, rect) in rects {
            let (dx, dy) = match align {
                Align::Left => (union.x - rect.x, 0.0),
                Align::CenterX => (union.center().x - rect.center().x, 0.0),
                Align::Right => (union.right() - rect.right(), 0.0),
                Align::Top => (0.0, union.y - rect.y),
                Align::CenterY => (0.0, union.center().y - rect.center().y),
                Align::Bottom => (0.0, union.bottom() - rect.bottom()),
            };
            deltas.insert(id, (dx, dy));
        }
        self.expand_frame_deltas(&mut deltas);
        if self.apply_deltas(&deltas) {
            CanvasAction::DocumentChanged
        } else {
            CanvasAction::None
        }
    }

    /// Distribute the selected elements with equal gaps along an axis.
    ///
    /// The outermost elements keep their positions and the middle elements are
    /// spread between them. Requires at least three movable elements; locked
    /// and deleted elements are ignored.
    pub fn distribute_selection(&mut self, axis: Axis) -> CanvasAction {
        let mut rects = self.selected_rects();
        if rects.len() < 3 {
            return CanvasAction::None;
        }
        rects.sort_by(|left, right| {
            let ordering = match axis {
                Axis::Horizontal => left.1.x.partial_cmp(&right.1.x),
                Axis::Vertical => left.1.y.partial_cmp(&right.1.y),
            };
            ordering
                .unwrap_or(Ordering::Equal)
                .then_with(|| left.0.cmp(&right.0))
        });
        let first = rects.first().map(|(_, rect)| *rect);
        let last = rects.last().map(|(_, rect)| *rect);
        let (Some(first), Some(last)) = (first, last) else {
            return CanvasAction::None;
        };
        let total_size: f64 = rects
            .iter()
            .map(|(_, rect)| match axis {
                Axis::Horizontal => rect.w,
                Axis::Vertical => rect.h,
            })
            .sum();
        let span = match axis {
            Axis::Horizontal => last.right() - first.x,
            Axis::Vertical => last.bottom() - first.y,
        };
        let gap = (span - total_size) / (rects.len() as f64 - 1.0);
        let mut cursor = match axis {
            Axis::Horizontal => first.x,
            Axis::Vertical => first.y,
        };
        let mut deltas: BTreeMap<String, (f64, f64)> = BTreeMap::new();
        for (id, rect) in rects {
            let (dx, dy) = match axis {
                Axis::Horizontal => (cursor - rect.x, 0.0),
                Axis::Vertical => (0.0, cursor - rect.y),
            };
            deltas.insert(id, (dx, dy));
            cursor += match axis {
                Axis::Horizontal => rect.w,
                Axis::Vertical => rect.h,
            } + gap;
        }
        self.expand_frame_deltas(&mut deltas);
        if self.apply_deltas(&deltas) {
            CanvasAction::DocumentChanged
        } else {
            CanvasAction::None
        }
    }

    /// Selected, movable elements with their stored rectangles.
    fn selected_rects(&self) -> Vec<(String, Rect)> {
        self.selection
            .iter()
            .filter_map(|id| {
                let element = self.doc.element(id)?;
                if element.is_deleted() || element.locked() {
                    return None;
                }
                Some((id.clone(), element_rect(element)))
            })
            .collect()
    }

    /// Give frame children the same delta as their selected frame unless the
    /// child is already selected.
    fn expand_frame_deltas(&self, deltas: &mut BTreeMap<String, (f64, f64)>) {
        let frame_ids = deltas.keys().cloned().collect::<Vec<_>>();
        for id in frame_ids {
            let Some(frame) = self.doc.element(&id) else {
                continue;
            };
            if frame.kind() != ElementKind::Frame {
                continue;
            }
            let Some(delta) = deltas.get(&id).copied() else {
                continue;
            };
            for child in self.doc.frame_children(&id) {
                deltas.entry(child.id().to_string()).or_insert(delta);
            }
        }
    }

    /// Apply position deltas and record an undo entry for each moved element.
    fn apply_deltas(&mut self, deltas: &BTreeMap<String, (f64, f64)>) -> bool {
        let mut changed = false;
        for (id, (dx, dy)) in deltas {
            if dx.abs() < f64::EPSILON && dy.abs() < f64::EPSILON {
                continue;
            }
            let Some(current) = self.doc.element(id).cloned() else {
                continue;
            };
            if current.locked() || current.is_deleted() {
                continue;
            }
            let before: Element = current.clone();
            if let Some(element) = self.doc.element_mut(id) {
                let x = element.x() + dx;
                let y = element.y() + dy;
                element.set_x(x);
                element.set_y(y);
            }
            let after = self.doc.element(id).cloned().unwrap_or(current);
            self.record_update(id, before, after);
            changed = true;
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::BoardDocument;

    fn rectangle(id: &str, x: f64, y: f64) -> Element {
        Element::new(ElementKind::Rectangle, id, x, y, 20.0, 20.0)
    }

    fn canvas_with(elements: Vec<Element>) -> CanvasState {
        let mut doc = BoardDocument::empty();
        for element in elements {
            doc.add_element(element).unwrap();
        }
        let mut canvas = CanvasState::new(doc);
        canvas.select_all();
        canvas
    }

    #[test]
    fn style_patch_updates_selected_elements() {
        let mut canvas = canvas_with(vec![rectangle("r1", 0.0, 0.0)]);
        let patch = StylePatch {
            stroke_color: Some("#ff0000".to_string()),
            fill_style: Some(FillStyle::Solid),
            opacity: Some(50.0),
            stroke_sharpness: Some(StrokeSharpness::Round),
            ..StylePatch::default()
        };
        assert_eq!(
            canvas.set_selection_style(&patch),
            CanvasAction::DocumentChanged
        );
        let style = canvas.document().element("r1").unwrap().style();
        assert_eq!(style.stroke_color, "#ff0000");
        assert_eq!(style.fill_style, FillStyle::Solid);
        assert_eq!(style.opacity, 50.0);
        assert_eq!(style.stroke_sharpness, StrokeSharpness::Round);
        assert!(canvas.can_undo());
    }

    #[test]
    fn empty_style_patch_is_a_no_op() {
        let mut canvas = canvas_with(vec![rectangle("r1", 0.0, 0.0)]);
        assert_eq!(
            canvas.set_selection_style(&StylePatch::default()),
            CanvasAction::None
        );
        assert!(!canvas.can_undo());
    }

    #[test]
    fn align_left_uses_the_selection_bounds() {
        let mut canvas = canvas_with(vec![
            rectangle("r1", 0.0, 0.0),
            rectangle("r2", 100.0, 40.0),
            rectangle("r3", 200.0, 80.0),
        ]);
        assert_eq!(
            canvas.align_selection(Align::Left),
            CanvasAction::DocumentChanged
        );
        for id in ["r1", "r2", "r3"] {
            assert_eq!(canvas.document().element(id).unwrap().x(), 0.0);
        }
    }

    #[test]
    fn align_skips_locked_elements() {
        let mut locked = rectangle("locked", 500.0, 0.0);
        locked.set_locked(true);
        let mut canvas = canvas_with(vec![rectangle("r1", 0.0, 0.0), locked]);
        canvas.align_selection(Align::Left);
        assert_eq!(canvas.document().element("r1").unwrap().x(), 0.0);
        assert_eq!(canvas.document().element("locked").unwrap().x(), 500.0);
    }

    #[test]
    fn distribute_horizontal_equalises_gaps() {
        let mut canvas = canvas_with(vec![
            rectangle("r1", 0.0, 0.0),
            rectangle("r2", 50.0, 0.0),
            rectangle("r3", 200.0, 0.0),
        ]);
        assert_eq!(
            canvas.distribute_selection(Axis::Horizontal),
            CanvasAction::DocumentChanged
        );
        let xs: Vec<f64> = ["r1", "r2", "r3"]
            .iter()
            .map(|id| canvas.document().element(id).unwrap().x())
            .collect();
        assert!((xs[1] - 100.0).abs() < 1e-9);
        assert!((xs[2] - 200.0).abs() < 1e-9);
    }

    #[test]
    fn distribute_requires_three_elements() {
        let mut canvas = canvas_with(vec![rectangle("r1", 0.0, 0.0), rectangle("r2", 50.0, 0.0)]);
        assert_eq!(
            canvas.distribute_selection(Axis::Horizontal),
            CanvasAction::None
        );
    }

    #[test]
    fn align_moves_frame_children() {
        let mut frame = Element::new(ElementKind::Frame, "f1", 100.0, 0.0, 200.0, 200.0);
        frame.set_frame_name(Some("Frame"));
        let mut child = rectangle("c1", 120.0, 20.0);
        child.set_frame_id(Some("f1"));
        let mut canvas = canvas_with(vec![rectangle("r1", 0.0, 0.0), frame, child]);
        canvas.selection = vec!["r1".to_string(), "f1".to_string()];
        canvas.align_selection(Align::Left);
        assert_eq!(canvas.document().element("f1").unwrap().x(), 0.0);
        assert_eq!(canvas.document().element("c1").unwrap().x(), 20.0);
    }
}
