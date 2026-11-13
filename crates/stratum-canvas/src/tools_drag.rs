//! Pointer drag and release handling for [`CanvasState`].
//!
//! Split out of `tools.rs` to keep every source file under the 500-line limit.

use crate::element::Element;
use crate::element_kind::ElementKind;
use crate::geometry::{
    element_rect, hit_test, hit_test_all, marquee_select, resize_handle_rects, resize_rect,
    snap_rect, Point, Rect,
};
use crate::history::Delta;
use crate::tools::{CanvasAction, CanvasState, Drag, Modifiers, PointerEvent};

impl CanvasState {
    /// Pointer drag.
    pub fn pointer_move(&mut self, ev: PointerEvent) -> CanvasAction {
        self.finalize_pending();
        let drag = std::mem::replace(&mut self.drag, Drag::None);
        let (next, action) = match drag {
            Drag::None => (Drag::None, CanvasAction::None),
            Drag::Pan { last } => {
                self.viewport.x += ev.screen.x - last.x;
                self.viewport.y += ev.screen.y - last.y;
                (Drag::Pan { last: ev.screen }, CanvasAction::None)
            }
            Drag::Marquee { start, additive } => {
                let rect = Rect::new(start.x, start.y, ev.point.x - start.x, ev.point.y - start.y);
                let hits = marquee_select(self.doc.elements(), rect);
                let mut selection = if additive {
                    self.selection.clone()
                } else {
                    Vec::new()
                };
                for id in hits {
                    if !selection.contains(&id) {
                        selection.push(id);
                    }
                }
                let changed = selection != self.selection;
                self.selection = selection;
                (
                    Drag::Marquee { start, additive },
                    if changed {
                        CanvasAction::SelectionChanged
                    } else {
                        CanvasAction::None
                    },
                )
            }
            Drag::Move {
                start,
                origins,
                original_rect,
            } => {
                let delta = ev.point - start;
                let snap = self.snap_delta(&original_rect, delta, &origins, ev.modifiers);
                let offset = delta + snap;
                for (id, ox, oy) in &origins {
                    if let Some(element) = self.doc.element_mut(id) {
                        element.set_x(ox + offset.x);
                        element.set_y(oy + offset.y);
                    }
                }
                (
                    Drag::Move {
                        start,
                        origins,
                        original_rect,
                    },
                    CanvasAction::DocumentChanged,
                )
            }
            Drag::Resize {
                id,
                handle,
                start,
                original,
            } => {
                let rect = resize_rect(&original, handle, ev.point - start);
                if let Some(element) = self.doc.element_mut(&id) {
                    element.set_x(rect.x);
                    element.set_y(rect.y);
                    element.set_width(rect.w);
                    element.set_height(rect.h);
                }
                (
                    Drag::Resize {
                        id,
                        handle,
                        start,
                        original,
                    },
                    CanvasAction::DocumentChanged,
                )
            }
            Drag::CreateShape { id, start, kind } => {
                let rect = drag_rect(start, ev.point, ev.modifiers.shift);
                self.set_rect(&id, rect);
                (
                    Drag::CreateShape { id, start, kind },
                    CanvasAction::DocumentChanged,
                )
            }
            Drag::Freedraw { id, mut points } => {
                points.push(ev.point);
                self.sync_absolute_points(&id, &points);
                (Drag::Freedraw { id, points }, CanvasAction::DocumentChanged)
            }
            Drag::FrameCreate { id, start } => {
                let rect = drag_rect(start, ev.point, ev.modifiers.shift);
                self.set_rect(&id, rect);
                (
                    Drag::FrameCreate { id, start },
                    CanvasAction::DocumentChanged,
                )
            }
            Drag::Erase { mut erased } => {
                let hits = hit_test_all(self.doc.elements(), ev.point, self.viewport.zoom);
                let mut changed = false;
                for id in hits {
                    if erased.contains(&id) {
                        continue;
                    }
                    if let Some(element) = self.doc.element_mut(&id) {
                        if !element.is_deleted() {
                            element.set_deleted(true);
                            erased.push(id);
                            changed = true;
                        }
                    }
                }
                (
                    Drag::Erase { erased },
                    if changed {
                        CanvasAction::DocumentChanged
                    } else {
                        CanvasAction::None
                    },
                )
            }
        };
        self.drag = next;
        action
    }

    /// Pointer release.
    pub fn pointer_up(&mut self, ev: PointerEvent) -> CanvasAction {
        self.finalize_pending();
        let drag = std::mem::replace(&mut self.drag, Drag::None);
        match drag {
            Drag::None | Drag::Pan { .. } => CanvasAction::None,
            Drag::Marquee { start, additive } => {
                let rect = Rect::new(start.x, start.y, ev.point.x - start.x, ev.point.y - start.y);
                let hits = marquee_select(self.doc.elements(), rect);
                let mut selection = if additive {
                    self.selection.clone()
                } else {
                    Vec::new()
                };
                for id in hits {
                    if !selection.contains(&id) {
                        selection.push(id);
                    }
                }
                self.selection = selection;
                CanvasAction::SelectionChanged
            }
            Drag::Move {
                start,
                origins,
                original_rect,
            } => {
                let delta = ev.point - start;
                let offset = delta + self.snap_delta(&original_rect, delta, &origins, ev.modifiers);
                if offset.length() <= 0.01 {
                    return CanvasAction::None;
                }
                for (id, ox, oy) in &origins {
                    if let Some(current) = self.doc.element(id).cloned() {
                        let mut before = current.clone();
                        before.set_x(*ox);
                        before.set_y(*oy);
                        self.record_update(id, before, current);
                    }
                }
                self.assign_frames(&origins);
                CanvasAction::DocumentChanged
            }
            Drag::Resize { id, original, .. } => {
                let Some(current) = self.doc.element(&id).cloned() else {
                    return CanvasAction::None;
                };
                let mut before = current.clone();
                before.set_x(original.x);
                before.set_y(original.y);
                before.set_width(original.w);
                before.set_height(original.h);
                self.record_update(&id, before, current);
                CanvasAction::DocumentChanged
            }
            Drag::CreateShape { id, start, kind } => {
                let fallback = Element::new(kind, "", 0.0, 0.0, 0.0, 0.0);
                let current = self.doc.element(&id).cloned().unwrap_or(fallback);
                let rect = if current.width() < 2.0 && current.height() < 2.0 {
                    Rect::new(start.x, start.y, 100.0, 100.0)
                } else {
                    element_rect(&current)
                };
                self.set_rect(&id, rect);
                if let Some(element) = self.doc.element(&id).cloned() {
                    let index = self.doc.elements().len().saturating_sub(1);
                    self.history.record(Delta::Insert { index, element });
                    self.selection = vec![id];
                }
                self.tool = crate::tools::Tool::Select;
                CanvasAction::DocumentChanged
            }
            Drag::Freedraw { id, points } => {
                self.sync_absolute_points(&id, &points);
                if let Some(element) = self.doc.element(&id).cloned() {
                    let index = self.doc.elements().len().saturating_sub(1);
                    self.history.record(Delta::Insert { index, element });
                    self.selection = vec![id];
                }
                CanvasAction::DocumentChanged
            }
            Drag::FrameCreate { id, start } => {
                let fallback = Element::new(ElementKind::Frame, "", 0.0, 0.0, 0.0, 0.0);
                let current = self.doc.element(&id).cloned().unwrap_or(fallback);
                let rect = if current.width() < 2.0 && current.height() < 2.0 {
                    Rect::new(start.x, start.y, 320.0, 240.0)
                } else {
                    element_rect(&current)
                };
                self.set_rect(&id, rect);
                if let Some(element) = self.doc.element(&id).cloned() {
                    let index = self.doc.elements().len().saturating_sub(1);
                    self.history.record(Delta::Insert { index, element });
                    self.selection = vec![id];
                }
                self.tool = crate::tools::Tool::Select;
                CanvasAction::DocumentChanged
            }
            Drag::Erase { erased } => {
                if erased.is_empty() {
                    return CanvasAction::None;
                }
                for id in erased {
                    if let Some(current) = self.doc.element(&id).cloned() {
                        let mut before = current.clone();
                        before.set_deleted(false);
                        self.record_update(&id, before, current);
                    }
                }
                CanvasAction::DocumentChanged
            }
        }
    }

    /// Selection press handling for the select tool.
    pub(crate) fn select_down(&mut self, ev: PointerEvent) -> CanvasAction {
        if self.selection.len() == 1 {
            if let Some(id) = self.selection.first().cloned() {
                if let Some(element) = self.doc.element(&id) {
                    let rect = element_rect(element);
                    for (handle, handle_rect) in resize_handle_rects(&rect).iter().enumerate() {
                        if handle_rect.contains(ev.point) {
                            self.drag = Drag::Resize {
                                id: id.clone(),
                                handle,
                                start: ev.point,
                                original: rect,
                            };
                            return CanvasAction::None;
                        }
                    }
                }
            }
        }
        let hit = hit_test(self.doc.elements(), ev.point, self.viewport.zoom);
        let Some(hit) = hit else {
            let additive = ev.modifiers.shift;
            self.drag = Drag::Marquee {
                start: ev.point,
                additive,
            };
            if !additive && !self.selection.is_empty() {
                self.selection.clear();
                return CanvasAction::SelectionChanged;
            }
            return CanvasAction::None;
        };
        if ev.modifiers.shift {
            if let Some(index) = self.selection.iter().position(|id| id == &hit) {
                self.selection.remove(index);
            } else {
                self.selection.push(hit);
            }
            return CanvasAction::SelectionChanged;
        }
        let selection_changed = self.selection != vec![hit.clone()];
        if selection_changed {
            self.selection = vec![hit.clone()];
        }
        if ev.modifiers.alt {
            let selection = self.selection.clone();
            let new_ids = self.duplicate_elements(&selection, 0.0, 0.0);
            if !new_ids.is_empty() {
                self.selection = new_ids;
            }
        }
        self.begin_move(ev.point);
        if ev.modifiers.alt {
            CanvasAction::DocumentChanged
        } else if selection_changed {
            CanvasAction::SelectionChanged
        } else {
            CanvasAction::None
        }
    }

    /// Start dragging the current selection.
    fn begin_move(&mut self, start: Point) {
        let selection = self.selection.clone();
        let ids = self.movable_ids(&selection);
        let origins: Vec<(String, f64, f64)> = ids
            .iter()
            .filter_map(|id| self.doc.element(id).map(|e| (id.clone(), e.x(), e.y())))
            .collect();
        let rects: Vec<Rect> = ids
            .iter()
            .filter_map(|id| self.doc.element(id).map(element_rect))
            .collect();
        let original_rect = rects
            .into_iter()
            .reduce(|acc, rect| acc.union(&rect))
            .unwrap_or(Rect::new(start.x, start.y, 0.0, 0.0));
        self.drag = Drag::Move {
            start,
            origins,
            original_rect,
        };
    }

    /// Snap delta for a move. Alt disables snapping.
    fn snap_delta(
        &self,
        moving: &Rect,
        delta: Point,
        origins: &[(String, f64, f64)],
        modifiers: Modifiers,
    ) -> Point {
        if modifiers.alt {
            return Point::new(0.0, 0.0);
        }
        let translated = Rect::new(moving.x + delta.x, moving.y + delta.y, moving.w, moving.h);
        let excluded: Vec<&str> = origins.iter().map(|(id, _, _)| id.as_str()).collect();
        let others: Vec<Rect> = self
            .doc
            .visible_elements()
            .filter(|element| !excluded.contains(&element.id()))
            .map(element_rect)
            .collect();
        snap_rect(&translated, &others, &self.snap, self.viewport.zoom)
    }

    /// Assign moved elements to a frame when their centre lands inside one.
    fn assign_frames(&mut self, origins: &[(String, f64, f64)]) {
        let frames: Vec<(String, Rect)> = self
            .doc
            .frames()
            .into_iter()
            .map(|frame| (frame.id().to_string(), element_rect(frame)))
            .collect();
        for (id, _, _) in origins {
            let Some(current) = self.doc.element(id).cloned() else {
                continue;
            };
            if current.kind() == ElementKind::Frame || current.kind().is_linear() {
                continue;
            }
            let center = element_rect(&current).center();
            let target = frames
                .iter()
                .find(|(frame_id, rect)| frame_id != id && rect.contains(center))
                .map(|(frame_id, _)| frame_id.clone());
            if current.frame_id() != target {
                let mut before = current.clone();
                before.set_frame_id(current.frame_id().as_deref());
                if let Some(element) = self.doc.element_mut(id) {
                    element.set_frame_id(target.as_deref());
                }
                let after = self.doc.element(id).cloned().unwrap_or(current);
                self.record_update(id, before, after);
            }
        }
    }
}

/// Rectangle from a drag, optionally forced square.
pub(crate) fn drag_rect(start: Point, end: Point, square: bool) -> Rect {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    if square {
        let size = dx.abs().max(dy.abs());
        let x = if dx < 0.0 { start.x - size } else { start.x };
        let y = if dy < 0.0 { start.y - size } else { start.y };
        return Rect::new(x, y, size, size);
    }
    Rect::new(start.x, start.y, dx, dy).normalized()
}
