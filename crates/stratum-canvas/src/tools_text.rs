//! Text editing for [`CanvasState`]: live buffer, commit, container binding
//! and cancel.
//!
//! Split out of `tools.rs` to keep every source file under the 500-line limit.

use crate::element::{Binding, Element};
use crate::element_kind::ElementKind;
use crate::geometry::Point;
use crate::history::Delta;
use crate::text::layout;
use crate::tools::{CanvasAction, CanvasState, TextEdit};

impl CanvasState {
    /// The live text buffer while editing.
    pub fn text_editing(&self) -> Option<&str> {
        self.text_edit.as_ref().map(|_| self.text_buffer.as_str())
    }

    /// Update the live text buffer and the element's text.
    pub fn update_text(&mut self, text: &str) -> CanvasAction {
        let Some(edit) = self.text_edit.clone() else {
            return CanvasAction::None;
        };
        self.text_buffer = text.to_string();
        if let Some(element) = self.doc.element_mut(&edit.id) {
            element.set_text(text);
        }
        CanvasAction::TextEditing(edit.id)
    }

    /// Commit the text edit (Enter).
    pub fn commit_text(&mut self) -> CanvasAction {
        let Some(edit) = self.text_edit.clone() else {
            return CanvasAction::None;
        };
        self.text_edit = None;
        let buffer = std::mem::take(&mut self.text_buffer);
        if buffer.trim().is_empty() {
            self.doc.remove_element(&edit.id);
            if !edit.created {
                self.history.record(Delta::Remove {
                    index: edit.index,
                    element: edit.original.clone(),
                });
            }
            return CanvasAction::DocumentChanged;
        }
        let font_size = 20.0;
        let line_height = 1.25;
        let text_layout = layout(&buffer, font_size, 400.0, line_height);
        let width = text_layout.width.max(20.0);
        let height = text_layout.height.max(line_height * font_size);
        let mut container_before = None;
        if let Some(container_id) = &edit.container {
            if let Some(container) = self.doc.element(container_id).cloned() {
                container_before = Some(container.clone());
                let x = container.x() + (container.width() - width) / 2.0;
                let y = container.y() + (container.height() - height) / 2.0;
                if let Some(element) = self.doc.element_mut(&edit.id) {
                    element.set_x(x.max(container.x()));
                    element.set_y(y.max(container.y()));
                }
                let mut bindings = container.bound_elements();
                if !bindings.iter().any(|binding| binding.id == edit.id) {
                    bindings.push(Binding {
                        element_type: "text".to_string(),
                        id: edit.id.clone(),
                    });
                }
                if let Some(element) = self.doc.element_mut(container_id) {
                    element.set_bound_elements(&bindings);
                }
            }
        }
        if let Some(element) = self.doc.element_mut(&edit.id) {
            element.set_text(&buffer);
            element.set_extra("originalText", serde_json::Value::String(buffer.clone()));
            element.set_extra("fontSize", serde_json::Value::from(font_size));
            element.set_extra("lineHeight", serde_json::Value::from(line_height));
            element.set_extra("textAlign", serde_json::Value::String("left".to_string()));
            element.set_extra(
                "verticalAlign",
                serde_json::Value::String("top".to_string()),
            );
            if let Some(container_id) = &edit.container {
                element.set_extra(
                    "containerId",
                    serde_json::Value::String(container_id.clone()),
                );
            }
            element.set_width(width);
            element.set_height(height);
        }
        let Some(after) = self.doc.element(&edit.id).cloned() else {
            return CanvasAction::DocumentChanged;
        };
        if edit.created {
            let index = self.doc.elements().len().saturating_sub(1);
            self.history.record(Delta::Insert {
                index,
                element: after,
            });
        } else {
            self.history.record(Delta::Update {
                id: edit.id.clone(),
                before: edit.original.clone(),
                after,
            });
        }
        if let (Some(container_id), Some(before)) = (&edit.container, container_before) {
            if let Some(container) = self.doc.element(container_id).cloned() {
                self.record_update(container_id, before, container);
            }
        }
        self.selection = vec![edit.id];
        CanvasAction::DocumentChanged
    }

    /// Create a text element and enter editing.
    pub(crate) fn create_text(&mut self, point: Point, container: Option<String>) -> CanvasAction {
        let id = uuid::Uuid::new_v4().to_string();
        let element = Element::new(ElementKind::Text, &id, point.x, point.y, 20.0, 25.0);
        let index = self.doc.elements().len();
        if self.doc.add_element(element.clone()).is_err() {
            return CanvasAction::None;
        }
        self.text_edit = Some(TextEdit {
            id: id.clone(),
            created: true,
            index,
            original: element,
            container,
        });
        self.text_buffer.clear();
        CanvasAction::TextEditing(id)
    }

    /// Enter editing on an existing text element.
    pub(crate) fn edit_existing_text(&mut self, element: &Element) -> CanvasAction {
        let index = self
            .doc
            .elements()
            .iter()
            .position(|candidate| candidate.id() == element.id())
            .unwrap_or(0);
        self.text_edit = Some(TextEdit {
            id: element.id().to_string(),
            created: false,
            index,
            original: element.clone(),
            container: element.container_id(),
        });
        self.text_buffer = element.text().unwrap_or_default();
        CanvasAction::TextEditing(element.id().to_string())
    }

    /// Cancel an in-progress text edit.
    pub(crate) fn cancel_text(&mut self) -> CanvasAction {
        let Some(edit) = self.text_edit.take() else {
            return CanvasAction::None;
        };
        self.text_buffer.clear();
        if edit.created {
            self.doc.remove_element(&edit.id);
        } else if let Some(element) = self.doc.element_mut(&edit.id) {
            *element = edit.original;
        }
        CanvasAction::DocumentChanged
    }
}
