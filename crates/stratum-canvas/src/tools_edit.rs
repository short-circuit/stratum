//! Editing operations for [`CanvasState`]: delete, duplicate, group, lock,
//! z-order, nudge, undo/redo, viewport zoom and the journal helpers.
//!
//! Split out of `tools.rs` to keep every source file under the 500-line limit.

use std::collections::HashMap;

use pkm_core::{PkmError, PkmResult};

use crate::element::{Binding, Element};
use crate::element_kind::ElementKind;
use crate::geometry::{Point, Rect};
use crate::history::Delta;
use crate::tools::{CanvasAction, CanvasState};

impl CanvasState {
    /// Soft-delete the selection.
    pub fn delete_selection(&mut self) -> CanvasAction {
        let mut changed = false;
        for id in self.selection.clone() {
            let Some(current) = self.doc.element(&id).cloned() else {
                continue;
            };
            if current.is_deleted() {
                continue;
            }
            let mut before = current.clone();
            before.set_deleted(false);
            if let Some(element) = self.doc.element_mut(&id) {
                element.set_deleted(true);
            }
            let after = self.doc.element(&id).cloned().unwrap_or(current);
            self.record_update(&id, before, after);
            changed = true;
        }
        if changed {
            self.selection.clear();
            CanvasAction::DocumentChanged
        } else {
            CanvasAction::None
        }
    }

    /// Duplicate the selection with fresh ids, offset by `(10, 10)`.
    pub fn duplicate_selection(&mut self) -> CanvasAction {
        if self.selection.is_empty() {
            return CanvasAction::None;
        }
        let selection = self.selection.clone();
        let new_ids = self.duplicate_elements(&selection, 10.0, 10.0);
        if new_ids.is_empty() {
            return CanvasAction::None;
        }
        self.selection = new_ids;
        CanvasAction::DocumentChanged
    }

    /// Group the selection under a new group id.
    pub fn group_selection(&mut self) -> CanvasAction {
        if self.selection.is_empty() {
            return CanvasAction::None;
        }
        let group = uuid::Uuid::new_v4().to_string();
        let ids = self.selection.clone();
        let before = self.group_snapshot(&ids);
        let after: Vec<Vec<String>> = before
            .iter()
            .map(|groups| {
                let mut next = groups.clone();
                next.push(group.clone());
                next
            })
            .collect();
        for (id, groups) in ids.iter().zip(after.iter()) {
            if let Some(element) = self.doc.element_mut(id) {
                element.set_group_ids(groups);
            }
        }
        self.history.record(Delta::Group { ids, before, after });
        CanvasAction::DocumentChanged
    }

    /// Remove group membership from the selection.
    pub fn ungroup_selection(&mut self) -> CanvasAction {
        if self.selection.is_empty() {
            return CanvasAction::None;
        }
        let ids = self.selection.clone();
        let before = self.group_snapshot(&ids);
        let after = vec![Vec::new(); ids.len()];
        for (id, groups) in ids.iter().zip(after.iter()) {
            if let Some(element) = self.doc.element_mut(id) {
                element.set_group_ids(groups);
            }
        }
        self.history.record(Delta::Group { ids, before, after });
        CanvasAction::DocumentChanged
    }

    /// Toggle the locked flag on the selection.
    pub fn toggle_lock_selection(&mut self) -> CanvasAction {
        let mut changed = false;
        for id in self.selection.clone() {
            if let Some(current) = self.doc.element(&id).cloned() {
                let mut before = current.clone();
                before.set_locked(!current.locked());
                if let Some(element) = self.doc.element_mut(&id) {
                    element.set_locked(!current.locked());
                }
                let after = self.doc.element(&id).cloned().unwrap_or(current);
                self.record_update(&id, before, after);
                changed = true;
            }
        }
        if changed {
            CanvasAction::DocumentChanged
        } else {
            CanvasAction::None
        }
    }

    /// Move the selection to the top of the z-order.
    pub fn bring_to_front(&mut self) -> CanvasAction {
        self.reorder_selection(|len, index| len.saturating_sub(1).max(index))
    }

    /// Move the selection to the bottom of the z-order.
    pub fn send_to_back(&mut self) -> CanvasAction {
        self.reorder_selection(|_, _| 0)
    }

    /// Move the selection one step forward.
    pub fn bring_forward(&mut self) -> CanvasAction {
        self.reorder_selection(|len, index| (index + 1).min(len.saturating_sub(1)))
    }

    /// Move the selection one step backward.
    pub fn send_backward(&mut self) -> CanvasAction {
        self.reorder_selection(|_, index| index.saturating_sub(1))
    }

    /// Nudge the selection by `(dx, dy)` world units, applying grid snapping.
    pub fn nudge_selection(&mut self, dx: f64, dy: f64) -> CanvasAction {
        let selection = self.selection.clone();
        let ids = self.movable_ids(&selection);
        if ids.is_empty() {
            return CanvasAction::None;
        }
        let mut changed = false;
        for id in ids {
            let Some(current) = self.doc.element(&id).cloned() else {
                continue;
            };
            if current.locked() {
                continue;
            }
            let mut before = current.clone();
            before.set_x(current.x());
            before.set_y(current.y());
            let mut x = current.x() + dx;
            let mut y = current.y() + dy;
            if let Some(grid) = self.snap.grid {
                if grid > 0.0 {
                    x = (x / grid).round() * grid;
                    y = (y / grid).round() * grid;
                }
            }
            if let Some(element) = self.doc.element_mut(&id) {
                element.set_x(x);
                element.set_y(y);
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

    /// Undo the most recent mutation.
    pub fn undo(&mut self) -> CanvasAction {
        self.finalize_pending();
        match self.history.undo(&mut self.doc) {
            Ok(true) => CanvasAction::DocumentChanged,
            _ => CanvasAction::None,
        }
    }

    /// Redo the most recently undone mutation.
    pub fn redo(&mut self) -> CanvasAction {
        self.finalize_pending();
        match self.history.redo(&mut self.doc) {
            Ok(true) => CanvasAction::DocumentChanged,
            _ => CanvasAction::None,
        }
    }

    /// Whether an undo entry is available.
    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    /// Whether a redo entry is available.
    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }

    /// Zoom around a screen point, clamped to `0.1..=30`.
    pub fn zoom_at(&mut self, screen: Point, factor: f64) {
        let world = self.viewport.screen_to_world(screen);
        let zoom = (self.viewport.zoom * factor).clamp(0.1, 30.0);
        self.viewport.zoom = zoom;
        self.viewport.x = screen.x - world.x * zoom;
        self.viewport.y = screen.y - world.y * zoom;
    }

    /// Pan the viewport by a screen delta.
    pub fn pan_by(&mut self, screen_delta: Point) {
        self.viewport.x += screen_delta.x;
        self.viewport.y += screen_delta.y;
    }

    /// Finish an in-progress polyline (Enter / Esc / double-click).
    pub fn finish_linear(&mut self) -> CanvasAction {
        let Some(polyline) = self.polyline.take() else {
            return CanvasAction::None;
        };
        if polyline.points.len() < 2 {
            self.doc.remove_element(&polyline.id);
            return CanvasAction::None;
        }
        self.sync_absolute_points(&polyline.id, &polyline.points);
        if let Some(element) = self.doc.element(&polyline.id).cloned() {
            let index = self.doc.elements().len().saturating_sub(1);
            self.history.record(Delta::Insert { index, element });
            self.selection = vec![polyline.id];
        }
        CanvasAction::DocumentChanged
    }

    /// Rename a frame.
    ///
    /// # Errors
    ///
    /// Returns [`PkmError::NotFound`] when the id is unknown and
    /// [`PkmError::Canvas`] when the element is not a frame.
    pub fn rename_frame(&mut self, id: &str, name: &str) -> PkmResult<()> {
        let element = self
            .doc
            .element(id)
            .cloned()
            .ok_or_else(|| PkmError::NotFound(format!("frame {id}")))?;
        if element.kind() != ElementKind::Frame {
            return Err(PkmError::Canvas(format!("{id} is not a frame")));
        }
        let mut before = element.clone();
        before.set_frame_name(element.frame_name().as_deref());
        if let Some(target) = self.doc.element_mut(id) {
            target.set_frame_name(Some(name));
        }
        let after = self.doc.element(id).cloned().unwrap_or(element);
        self.record_update(id, before, after);
        Ok(())
    }

    /// Whether the document has unsaved changes.
    pub fn dirty(&self) -> bool {
        self.doc.is_dirty()
    }

    /// Duplicate elements, remapping internal references, and record inserts.
    pub(crate) fn duplicate_elements(&mut self, ids: &[String], dx: f64, dy: f64) -> Vec<String> {
        let originals: Vec<Element> = ids
            .iter()
            .filter_map(|id| self.doc.element(id).cloned())
            .collect();
        let mut id_map = HashMap::new();
        let mut group_map: HashMap<String, String> = HashMap::new();
        for element in &originals {
            id_map.insert(element.id().to_string(), uuid::Uuid::new_v4().to_string());
        }
        let mut new_ids = Vec::new();
        for mut element in originals {
            let Some(new_id) = id_map.get(element.id()).cloned() else {
                continue;
            };
            element.set_id(&new_id);
            element.set_x(element.x() + dx);
            element.set_y(element.y() + dy);
            let groups = element
                .group_ids()
                .into_iter()
                .map(|group| {
                    group_map
                        .entry(group)
                        .or_insert_with(|| uuid::Uuid::new_v4().to_string())
                        .clone()
                })
                .collect::<Vec<_>>();
            element.set_group_ids(&groups);
            let bindings = element
                .bound_elements()
                .into_iter()
                .map(|binding| Binding {
                    element_type: binding.element_type,
                    id: id_map.get(&binding.id).cloned().unwrap_or(binding.id),
                })
                .collect::<Vec<_>>();
            element.set_bound_elements(&bindings);
            if let Some(mut binding) = element.start_binding() {
                binding.element_id = id_map
                    .get(&binding.element_id)
                    .cloned()
                    .unwrap_or(binding.element_id);
                element.set_start_binding(Some(&binding));
            }
            if let Some(mut binding) = element.end_binding() {
                binding.element_id = id_map
                    .get(&binding.element_id)
                    .cloned()
                    .unwrap_or(binding.element_id);
                element.set_end_binding(Some(&binding));
            }
            if let Some(container) = element.container_id() {
                if let Some(mapped) = id_map.get(&container) {
                    element.set_extra("containerId", serde_json::Value::String(mapped.clone()));
                }
            }
            if let Some(frame) = element.frame_id() {
                if let Some(mapped) = id_map.get(&frame) {
                    element.set_frame_id(Some(mapped));
                }
            }
            let index = self.doc.elements().len();
            if self.doc.add_element(element.clone()).is_ok() {
                self.history.record(Delta::Insert { index, element });
                new_ids.push(new_id);
            }
        }
        new_ids
    }

    /// Selected ids plus frame children (dragging a frame moves its children).
    pub(crate) fn movable_ids(&self, selection: &[String]) -> Vec<String> {
        let mut ids = Vec::new();
        for id in selection {
            if !ids.contains(id) {
                ids.push(id.clone());
            }
            if let Some(element) = self.doc.element(id) {
                if element.kind() == ElementKind::Frame {
                    for child in self.doc.frame_children(id) {
                        let child_id = child.id().to_string();
                        if !ids.contains(&child_id) {
                            ids.push(child_id);
                        }
                    }
                }
            }
        }
        ids
    }

    /// Reorder the selection using a target-index function.
    fn reorder_selection<F>(&mut self, target: F) -> CanvasAction
    where
        F: Fn(usize, usize) -> usize,
    {
        let mut changed = false;
        for id in self.selection.clone() {
            let Some(from) = self.doc.elements().iter().position(|e| e.id() == id) else {
                continue;
            };
            let len = self.doc.elements().len();
            let to = target(len, from);
            if to == from {
                continue;
            }
            if self.doc.reorder(&id, to).is_ok() {
                self.history.record(Delta::Reorder { id, from, to });
                changed = true;
            }
        }
        if changed {
            CanvasAction::DocumentChanged
        } else {
            CanvasAction::None
        }
    }

    /// Current group ids for a set of elements.
    fn group_snapshot(&self, ids: &[String]) -> Vec<Vec<String>> {
        ids.iter()
            .map(|id| {
                self.doc
                    .element(id)
                    .map(|element| element.group_ids())
                    .unwrap_or_default()
            })
            .collect()
    }

    /// Record an element update delta.
    pub(crate) fn record_update(&mut self, id: &str, before: Element, after: Element) {
        self.history.record(Delta::Update {
            id: id.to_string(),
            before,
            after,
        });
    }

    /// Apply a rectangle to an element.
    pub(crate) fn set_rect(&mut self, id: &str, rect: Rect) {
        if let Some(element) = self.doc.element_mut(id) {
            element.set_x(rect.x);
            element.set_y(rect.y);
            element.set_width(rect.w);
            element.set_height(rect.h);
        }
    }

    /// Rebase absolute points onto the element origin.
    pub(crate) fn sync_absolute_points(&mut self, id: &str, points: &[Point]) {
        let min_x = points.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
        let min_y = points.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
        let max_x = points.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
        let max_y = points.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);
        let relative: Vec<Point> = points
            .iter()
            .map(|p| Point::new(p.x - min_x, p.y - min_y))
            .collect();
        if let Some(target) = self.doc.element_mut(id) {
            target.set_x(min_x);
            target.set_y(min_y);
            target.set_width(max_x - min_x);
            target.set_height(max_y - min_y);
            target.set_points(&relative);
        }
    }

    /// Finalize edits made through [`CanvasState::document_mut`].
    pub(crate) fn finalize_pending(&mut self) {
        let Some(before) = self.pending_before.take() else {
            return;
        };
        let after = self.doc.elements_snapshot();
        if before == after {
            return;
        }
        for (index, element) in before.iter().enumerate() {
            if !after.iter().any(|candidate| candidate.id() == element.id()) {
                self.history.record(Delta::Remove {
                    index,
                    element: element.clone(),
                });
            }
        }
        for (index, element) in after.iter().enumerate() {
            match before
                .iter()
                .find(|candidate| candidate.id() == element.id())
            {
                None => self.history.record(Delta::Insert {
                    index,
                    element: element.clone(),
                }),
                Some(old) if old != element => self.history.record(Delta::Update {
                    id: element.id().to_string(),
                    before: old.clone(),
                    after: element.clone(),
                }),
                Some(_) => {}
            }
        }
        self.doc.mark_dirty();
    }
}
