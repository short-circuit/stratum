//! Style, rounding and binding models for Excalidraw elements.
//!
//! Split out of `element.rs` to keep every source file under the 500-line
//! limit. The types are re-exported from [`crate::element`] so callers keep
//! using the documented `element::Style` / `element::Binding` paths.

use pkm_core::{PkmError, PkmResult};
use serde_json::{Map, Value};

use crate::element_kind::{
    Arrowhead, FillStyle, FontFamily, StrokeSharpness, StrokeStyle, TextAlign, VerticalAlign,
};

/// Canonical stroke colour used by Excalidraw for new elements.
pub const DEFAULT_STROKE_COLOR: &str = "#1e1e1e";
/// Canonical background colour for unfilled elements.
pub const DEFAULT_BACKGROUND_COLOR: &str = "transparent";

/// Corner rounding descriptor stored in the canonical `roundness` object.
#[derive(Debug, Clone, PartialEq)]
pub struct Roundness {
    /// Excalidraw roundness kind (`2` fixed radius, `3` adaptive radius).
    pub kind: u32,
    /// Optional fixed radius in world units (kind `2`).
    pub value: Option<f64>,
}

impl Roundness {
    /// Read a `roundness` JSON object.
    ///
    /// Returns `None` when the value is `null`, missing, or not an object.
    pub fn from_json(value: &Value) -> Option<Self> {
        let obj = value.as_object()?;
        let kind = obj.get("type").and_then(Value::as_u64).unwrap_or(3) as u32;
        let radius = obj.get("value").and_then(Value::as_f64);
        Some(Self {
            kind,
            value: radius,
        })
    }

    /// Serialize the canonical `roundness` object.
    pub fn to_json(&self) -> Value {
        let mut map = Map::new();
        map.insert("type".into(), Value::from(self.kind));
        if let Some(value) = self.value {
            map.insert("value".into(), Value::from(value));
        }
        Value::Object(map)
    }
}

/// Style model for one element, mapping to Excalidraw's known style keys.
///
/// The default values mirror Excalidraw's "draw" defaults. `bowing` is a
/// renderer knob only and is never persisted.
#[derive(Debug, Clone, PartialEq)]
pub struct Style {
    /// Stroke colour (`strokeColor`).
    pub stroke_color: String,
    /// Fill colour (`backgroundColor`).
    pub background_color: String,
    /// Fill pattern (`fillStyle`).
    pub fill_style: FillStyle,
    /// Stroke width in world units (`strokeWidth`).
    pub stroke_width: f64,
    /// Stroke dash style (`strokeStyle`).
    pub stroke_style: StrokeStyle,
    /// Hand-drawn roughness (`roughness`).
    pub roughness: f64,
    /// Opacity percentage (`opacity`).
    pub opacity: f64,
    /// Deterministic roughness seed (`seed`).
    pub seed: u64,
    /// Corner sharpness derived from `roundness` or the legacy field.
    pub stroke_sharpness: StrokeSharpness,
    /// Canonical `roundness` object when present.
    pub roundness: Option<Roundness>,
    /// Font family (`fontFamily`).
    pub font_family: FontFamily,
    /// Font size in world units (`fontSize`).
    pub font_size: f64,
    /// Horizontal text alignment (`textAlign`).
    pub text_align: TextAlign,
    /// Vertical text alignment (`verticalAlign`).
    pub vertical_align: VerticalAlign,
    /// Line height multiplier (`lineHeight`).
    pub line_height: f64,
    /// Renderer-only bowing knob; not persisted.
    pub bowing: f64,
}

impl Default for Style {
    /// Excalidraw draw defaults. The seed is `0` here; `Element::new` seeds
    /// from the element id so fixtures reproduce deterministically.
    fn default() -> Self {
        Self {
            stroke_color: DEFAULT_STROKE_COLOR.to_string(),
            background_color: DEFAULT_BACKGROUND_COLOR.to_string(),
            fill_style: FillStyle::Hachure,
            stroke_width: 2.0,
            stroke_style: StrokeStyle::Solid,
            roughness: 1.0,
            opacity: 100.0,
            seed: 0,
            stroke_sharpness: StrokeSharpness::Sharp,
            roundness: None,
            font_family: FontFamily::Normal,
            font_size: 20.0,
            text_align: TextAlign::Left,
            vertical_align: VerticalAlign::Top,
            line_height: 1.25,
            bowing: 1.0,
        }
    }
}

/// A bound-element reference (`boundElements` entry).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    /// Target element `type`.
    pub element_type: String,
    /// Target element id.
    pub id: String,
}

impl Binding {
    /// Read one `boundElements` entry.
    ///
    /// # Errors
    ///
    /// Returns [`PkmError::Canvas`] when `type` or `id` is missing.
    pub fn from_json(value: &Value) -> PkmResult<Self> {
        let obj = value
            .as_object()
            .ok_or_else(|| PkmError::Canvas("boundElements entry is not an object".to_string()))?;
        let element_type = obj
            .get("type")
            .and_then(Value::as_str)
            .ok_or_else(|| PkmError::Canvas("binding without type".to_string()))?;
        let id = obj
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| PkmError::Canvas("binding without id".to_string()))?;
        Ok(Self {
            element_type: element_type.to_string(),
            id: id.to_string(),
        })
    }

    /// Serialize one `boundElements` entry.
    pub fn to_json(&self) -> Value {
        let mut map = Map::new();
        map.insert("type".into(), Value::String(self.element_type.clone()));
        map.insert("id".into(), Value::String(self.id.clone()));
        Value::Object(map)
    }
}

/// An arrow endpoint binding (`startBinding`/`endBinding`).
#[derive(Debug, Clone, PartialEq)]
pub struct ArrowBinding {
    /// Bound element id (`elementId`).
    pub element_id: String,
    /// Focus ratio along the bound shape (0..=1).
    pub focus: f64,
    /// Gap in world units.
    pub gap: f64,
}

impl ArrowBinding {
    /// Read a binding object.
    ///
    /// # Errors
    ///
    /// Returns [`PkmError::Canvas`] when `elementId` is missing; `focus` and
    /// `gap` default to `0`.
    pub fn from_json(value: &Value) -> PkmResult<Self> {
        let obj = value
            .as_object()
            .ok_or_else(|| PkmError::Canvas("arrow binding is not an object".to_string()))?;
        let element_id = obj
            .get("elementId")
            .and_then(Value::as_str)
            .ok_or_else(|| PkmError::Canvas("arrow binding without elementId".to_string()))?;
        Ok(Self {
            element_id: element_id.to_string(),
            focus: obj.get("focus").and_then(Value::as_f64).unwrap_or(0.0),
            gap: obj.get("gap").and_then(Value::as_f64).unwrap_or(0.0),
        })
    }

    /// Serialize the binding object.
    pub fn to_json(&self) -> Value {
        let mut map = Map::new();
        map.insert("elementId".into(), Value::String(self.element_id.clone()));
        map.insert("focus".into(), Value::from(self.focus));
        map.insert("gap".into(), Value::from(self.gap));
        Value::Object(map)
    }
}

/// Arrowhead helper used by [`crate::element::Element::set_end_arrowhead`].
pub(crate) fn arrowhead_wire(head: Arrowhead) -> &'static str {
    head.as_wire()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binding_requires_type_and_id() {
        assert!(Binding::from_json(&serde_json::json!({"type": "text"})).is_err());
        let binding = Binding::from_json(&serde_json::json!({"type": "text", "id": "t1"})).unwrap();
        assert_eq!(binding.id, "t1");
    }

    #[test]
    fn roundness_round_trips() {
        let roundness = Roundness {
            kind: 3,
            value: None,
        };
        let parsed = Roundness::from_json(&roundness.to_json()).unwrap();
        assert_eq!(parsed, roundness);
    }
}
