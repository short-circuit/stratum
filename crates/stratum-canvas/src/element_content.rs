//! Content accessors for [`Element`]: points, text, bindings, arrowheads,
//! files, frame names and the shared key helpers.
//!
//! Split out of `element.rs` to keep every source file under the 500-line
//! limit. The methods are re-exported through the documented `element::Element`
//! type, so callers never import this module directly.

use serde_json::{Map, Value};

use crate::element::{seed_for, Element};
use crate::element_kind::Arrowhead;
use crate::element_style::{arrowhead_wire, ArrowBinding, Binding};
use crate::geometry::Point;

impl Element {
    /// Read the polyline points.
    ///
    /// Both the canonical `[x, y]` tuples and `{x, y}` objects are accepted.
    pub fn points(&self) -> Vec<Point> {
        let Some(Value::Array(items)) = self.raw.get("points") else {
            return Vec::new();
        };
        items
            .iter()
            .filter_map(|item| {
                if let Some(pair) = item.as_array() {
                    let x = pair.first().and_then(Value::as_f64)?;
                    let y = pair.get(1).and_then(Value::as_f64)?;
                    Some(Point { x, y })
                } else {
                    let obj = item.as_object()?;
                    Some(Point {
                        x: obj.get("x").and_then(Value::as_f64)?,
                        y: obj.get("y").and_then(Value::as_f64)?,
                    })
                }
            })
            .collect()
    }

    /// Write the polyline points using Excalidraw's `[x, y]` tuples.
    pub fn set_points(&mut self, points: &[Point]) {
        let values = points
            .iter()
            .map(|p| Value::Array(vec![Value::from(p.x), Value::from(p.y)]))
            .collect();
        self.raw.insert("points".into(), Value::Array(values));
    }

    /// The text content, when the element carries one.
    pub fn text(&self) -> Option<String> {
        self.str_field("text").map(str::to_string)
    }

    /// Set the text content.
    pub fn set_text(&mut self, text: &str) {
        self.set_str("text", text);
    }

    /// The original (unwrapped) text, when present.
    pub fn original_text(&self) -> Option<String> {
        self.str_field("originalText").map(str::to_string)
    }

    /// The bound container id, when this text lives inside a container.
    pub fn container_id(&self) -> Option<String> {
        self.str_field("containerId").map(str::to_string)
    }

    /// Read the `boundElements` references.
    pub fn bound_elements(&self) -> Vec<Binding> {
        let Some(Value::Array(items)) = self.raw.get("boundElements") else {
            return Vec::new();
        };
        items
            .iter()
            .filter_map(|item| Binding::from_json(item).ok())
            .collect()
    }

    /// Replace the `boundElements` references.
    pub fn set_bound_elements(&mut self, bindings: &[Binding]) {
        let values = bindings.iter().map(Binding::to_json).collect();
        self.raw
            .insert("boundElements".into(), Value::Array(values));
    }

    /// The arrow start binding, when present.
    pub fn start_binding(&self) -> Option<ArrowBinding> {
        self.raw
            .get("startBinding")
            .filter(|v| !v.is_null())
            .and_then(|v| ArrowBinding::from_json(v).ok())
    }

    /// The arrow end binding, when present.
    pub fn end_binding(&self) -> Option<ArrowBinding> {
        self.raw
            .get("endBinding")
            .filter(|v| !v.is_null())
            .and_then(|v| ArrowBinding::from_json(v).ok())
    }

    /// Set or clear the arrow start binding.
    pub fn set_start_binding(&mut self, binding: Option<&ArrowBinding>) {
        match binding {
            Some(binding) => {
                self.raw.insert("startBinding".into(), binding.to_json());
            }
            None => {
                self.raw.insert("startBinding".into(), Value::Null);
            }
        }
    }

    /// Set or clear the arrow end binding.
    pub fn set_end_binding(&mut self, binding: Option<&ArrowBinding>) {
        match binding {
            Some(binding) => {
                self.raw.insert("endBinding".into(), binding.to_json());
            }
            None => {
                self.raw.insert("endBinding".into(), Value::Null);
            }
        }
    }

    /// The start arrowhead, when present.
    pub fn start_arrowhead(&self) -> Option<Arrowhead> {
        self.str_field("startArrowhead")
            .and_then(Arrowhead::from_wire)
    }

    /// The end arrowhead, when present.
    pub fn end_arrowhead(&self) -> Option<Arrowhead> {
        self.str_field("endArrowhead")
            .and_then(Arrowhead::from_wire)
    }

    /// Set or clear the start arrowhead.
    pub fn set_start_arrowhead(&mut self, head: Option<Arrowhead>) {
        self.set_optional_str("startArrowhead", head.map(arrowhead_wire));
    }

    /// Set or clear the end arrowhead.
    pub fn set_end_arrowhead(&mut self, head: Option<Arrowhead>) {
        self.set_optional_str("endArrowhead", head.map(arrowhead_wire));
    }

    /// The embedded image file id, when present.
    pub fn file_id(&self) -> Option<String> {
        self.str_field("fileId").map(str::to_string)
    }

    /// Set or clear the embedded image file id.
    pub fn set_file_id(&mut self, id: Option<&str>) {
        self.set_optional_str("fileId", id);
    }

    /// The frame name, when present.
    pub fn frame_name(&self) -> Option<String> {
        self.str_field("name").map(str::to_string)
    }

    /// Set or clear the frame name.
    pub fn set_frame_name(&mut self, name: Option<&str>) {
        self.set_optional_str("name", name);
    }

    /// Raw read-only view of the backing object. Renderers use it to pass
    /// unknown fields through without losing them.
    pub fn raw(&self) -> &Map<String, Value> {
        &self.raw
    }

    /// Insert or overwrite an arbitrary key (used by fixture builders and
    /// callers that want to attach custom metadata).
    pub fn set_extra(&mut self, key: &str, value: Value) {
        self.raw.insert(key.to_string(), value);
    }

    /// Bump `version`, `versionNonce` and `updated` exactly like Excalidraw
    /// does on edit. The nonce is deterministic for a given id/timestamp/
    /// version so tests stay reproducible.
    pub fn touch(&mut self, now_ms: u64) {
        let version = self
            .raw
            .get("version")
            .and_then(Value::as_u64)
            .unwrap_or(1)
            .saturating_add(1);
        self.raw.insert("version".into(), Value::from(version));
        self.raw.insert(
            "versionNonce".into(),
            Value::from(seed_for(self.id(), now_ms, version)),
        );
        self.raw.insert("updated".into(), Value::from(now_ms));
    }

    // The helpers below are shared with `element.rs`, so they are crate
    // visible rather than private to this module.
    pub(crate) fn str_field(&self, key: &str) -> Option<&str> {
        self.raw.get(key).and_then(Value::as_str)
    }

    pub(crate) fn f64_field(&self, key: &str, default: f64) -> f64 {
        self.raw.get(key).and_then(Value::as_f64).unwrap_or(default)
    }

    pub(crate) fn set_str(&mut self, key: &str, value: &str) {
        self.raw
            .insert(key.to_string(), Value::String(value.to_string()));
    }

    pub(crate) fn set_optional_str(&mut self, key: &str, value: Option<&str>) {
        match value {
            Some(value) => {
                self.raw
                    .insert(key.to_string(), Value::String(value.to_string()));
            }
            None => {
                self.raw.insert(key.to_string(), Value::Null);
            }
        }
    }

    pub(crate) fn string_array(&self, key: &str) -> Vec<String> {
        match self.raw.get(key) {
            Some(Value::Array(items)) => items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect(),
            _ => Vec::new(),
        }
    }

    pub(crate) fn set_string_array(&mut self, key: &str, values: &[String]) {
        let items = values
            .iter()
            .map(|value| Value::String(value.clone()))
            .collect();
        self.raw.insert(key.to_string(), Value::Array(items));
    }
}
