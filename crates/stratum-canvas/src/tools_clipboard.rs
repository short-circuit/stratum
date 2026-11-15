//! Clipboard operations for [`CanvasState`]: copy the selection as Excalidraw
//! clipboard JSON and paste it back with fresh ids.
//!
//! Split out of `tools_edit.rs` to keep every source file under the 500-line
//! limit. The clipboard payload mirrors the shipped Excalidraw format
//! (`{"type":"excalidraw/clipboard","elements":[...]}`), but a bare element
//! array is accepted on paste so callers can feed either shape.
//!
//! Pasted elements are remapped the same way a duplicate is: every element
//! gets a fresh uuid, group ids and internal references are remapped, and
//! positions are offset so the paste lands beside the source selection. A
//! malformed payload is reported through [`pkm_core::PkmError::Canvas`].

use std::collections::HashMap;

use pkm_core::{PkmError, PkmResult};
use serde_json::{Map, Value};

use crate::element::{Binding, Element};
use crate::history::Delta;
use crate::tools::{CanvasAction, CanvasState};

/// Clipboard payload discriminator written by Excalidraw.
pub const CLIPBOARD_TYPE: &str = "excalidraw/clipboard";

/// World-unit offset applied to pasted elements.
const PASTE_OFFSET: f64 = 10.0;

impl CanvasState {
    /// Serialize the selected, non-deleted elements as Excalidraw clipboard
    /// JSON.
    ///
    /// Elements are emitted in document (z-order) order. The document's
    /// embedded `files` map is included so pasted images keep their payloads.
    /// An empty selection yields a valid payload with an empty element array.
    pub fn copy_selection(&self) -> String {
        let elements = self
            .doc
            .elements()
            .iter()
            .filter(|element| {
                !element.is_deleted() && self.selection.iter().any(|id| id.as_str() == element.id())
            })
            .map(Element::to_json)
            .collect::<Vec<_>>();
        let mut payload = Map::new();
        payload.insert("type".into(), Value::String(CLIPBOARD_TYPE.to_string()));
        payload.insert("elements".into(), Value::Array(elements));
        if let Value::Object(files) = self.doc.files() {
            payload.insert("files".into(), Value::Object(files.clone()));
        }
        Value::Object(payload).to_string()
    }

    /// Paste Excalidraw clipboard JSON into the document.
    ///
    /// Accepts either the canonical clipboard object or a bare element array.
    /// New elements are placed at the top of the z-order, offset by
    /// `(10, 10)`, and become the selection.
    ///
    /// # Errors
    ///
    /// Returns [`PkmError::Canvas`] when the payload is not JSON, is neither an
    /// object with an `elements` array nor a bare array, or contains an invalid
    /// element.
    pub fn paste(&mut self, payload: &str) -> PkmResult<CanvasAction> {
        let value: Value = serde_json::from_str(payload)
            .map_err(|error| PkmError::Canvas(format!("clipboard payload is not JSON: {error}")))?;
        let (element_values, files) = match value {
            Value::Array(items) => (items, None),
            Value::Object(map) => {
                let items = map
                    .get("elements")
                    .and_then(Value::as_array)
                    .cloned()
                    .ok_or_else(|| {
                        PkmError::Canvas("clipboard payload without elements".to_string())
                    })?;
                (items, map.get("files").cloned())
            }
            _ => {
                return Err(PkmError::Canvas(
                    "clipboard payload is neither an array nor an object".to_string(),
                ))
            }
        };
        let mut elements = Vec::with_capacity(element_values.len());
        for value in element_values {
            elements.push(Element::from_json(value)?);
        }
        if elements.is_empty() {
            return Ok(CanvasAction::None);
        }
        // Merge embedded files before inserting elements so image references
        // resolve as soon as the paste completes.
        if let Some(Value::Object(files)) = files {
            for (file_id, entry) in files {
                let data_url = entry.get("dataURL").and_then(Value::as_str);
                let mime = entry.get("mimeType").and_then(Value::as_str);
                if let (Some(data_url), Some(mime)) = (data_url, mime) {
                    self.doc.set_file(&file_id, data_url, mime);
                }
            }
        }
        let new_ids = self.insert_clipboard_elements(elements, PASTE_OFFSET, PASTE_OFFSET);
        if new_ids.is_empty() {
            return Ok(CanvasAction::None);
        }
        self.selection = new_ids;
        Ok(CanvasAction::DocumentChanged)
    }

    /// Clone clipboard elements into the document with remapped references.
    fn insert_clipboard_elements(
        &mut self,
        originals: Vec<Element>,
        dx: f64,
        dy: f64,
    ) -> Vec<String> {
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
                    element.set_extra("containerId", Value::String(mapped.clone()));
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::BoardDocument;
    use crate::element_kind::ElementKind;

    fn canvas_with_rectangle() -> CanvasState {
        let mut doc = BoardDocument::empty();
        doc.add_element(Element::new(
            ElementKind::Rectangle,
            "r1",
            10.0,
            20.0,
            40.0,
            30.0,
        ))
        .unwrap();
        let mut canvas = CanvasState::new(doc);
        canvas.select_all();
        canvas
    }

    #[test]
    fn copy_serializes_selected_elements() {
        let canvas = canvas_with_rectangle();
        let payload = canvas.copy_selection();
        let value: Value = serde_json::from_str(&payload).unwrap();
        assert_eq!(value["type"], CLIPBOARD_TYPE);
        assert_eq!(value["elements"].as_array().unwrap().len(), 1);
        assert_eq!(value["elements"][0]["id"], "r1");
    }

    #[test]
    fn paste_remaps_ids_and_offsets() {
        let mut canvas = canvas_with_rectangle();
        let payload = canvas.copy_selection();
        let action = canvas.paste(&payload).unwrap();
        assert_eq!(action, CanvasAction::DocumentChanged);
        assert_eq!(canvas.selection().len(), 1);
        let pasted_id = canvas.selection()[0].clone();
        assert_ne!(pasted_id, "r1");
        let pasted = canvas.document().element(&pasted_id).unwrap();
        assert_eq!(pasted.x(), 20.0);
        assert_eq!(pasted.y(), 30.0);
        assert_eq!(canvas.document().visible_elements().count(), 2);
    }

    #[test]
    fn paste_accepts_bare_array() {
        let mut canvas = CanvasState::new(BoardDocument::empty());
        let element = Element::new(ElementKind::Ellipse, "e1", 0.0, 0.0, 10.0, 10.0);
        let payload = Value::Array(vec![element.to_json()]).to_string();
        let action = canvas.paste(&payload).unwrap();
        assert_eq!(action, CanvasAction::DocumentChanged);
        assert_eq!(canvas.selection().len(), 1);
    }

    #[test]
    fn empty_payload_is_a_no_op() {
        let mut canvas = CanvasState::new(BoardDocument::empty());
        let payload = Value::Object(Map::new()).to_string();
        assert!(canvas.paste(&payload).is_err());
        let empty = Value::Array(Vec::new()).to_string();
        assert_eq!(canvas.paste(&empty).unwrap(), CanvasAction::None);
    }

    #[test]
    fn malformed_paste_reports_canvas_error() {
        let mut canvas = CanvasState::new(BoardDocument::empty());
        let error = canvas.paste("not json").unwrap_err();
        assert!(matches!(error, PkmError::Canvas(_)));
        let error = canvas.paste(r#"{"elements":[{"id":"x"}]}"#).unwrap_err();
        assert!(matches!(error, PkmError::Canvas(_)));
    }
}
