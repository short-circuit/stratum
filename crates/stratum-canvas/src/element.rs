//! Lossless Excalidraw element model.
//!
//! Every [`Element`] keeps its entire original JSON object as the backing
//! store. Typed accessors read and write individual known keys inside that
//! map, so unknown or future fields survive a load/save cycle verbatim. This
//! is what lets a board edited in Stratum be opened again in Excalidraw
//! without losing data.
//!
//! Use [`Element::from_json`] to parse one object, [`Element::new`] to build a
//! canonical element, and [`Element::to_json`] to serialize it back with
//! deterministic key ordering. Errors are reported through
//! [`pkm_core::PkmError`].

use std::collections::BTreeMap;

use pkm_core::{PkmError, PkmResult};
use serde_json::{Map, Value};

use crate::element_kind::{
    ElementKind, FillStyle, FontFamily, StrokeSharpness, StrokeStyle, TextAlign, VerticalAlign,
};

// The style/binding model lives in `element_style.rs` to keep files small;
// re-export it here so the documented `element::Style` paths stay valid.
pub use crate::element_style::{
    ArrowBinding, Binding, Roundness, Style, DEFAULT_BACKGROUND_COLOR, DEFAULT_STROKE_COLOR,
};

/// One Excalidraw element backed by its full original JSON object.
#[derive(Debug, Clone, PartialEq)]
pub struct Element {
    pub(crate) raw: Map<String, Value>,
}

impl Element {
    /// Parse one JSON object into an element.
    ///
    /// # Errors
    ///
    /// Returns [`PkmError::Canvas`] when the value is not an object or when
    /// `type`/`id` are missing or not strings.
    pub fn from_json(value: Value) -> PkmResult<Self> {
        let map = match value {
            Value::Object(map) => map,
            _ => return Err(PkmError::Canvas("element is not an object".to_string())),
        };
        if !map.get("type").map(Value::is_string).unwrap_or(false) {
            return Err(PkmError::Canvas("element without type".to_string()));
        }
        if !map.get("id").map(Value::is_string).unwrap_or(false) {
            return Err(PkmError::Canvas("element without id".to_string()));
        }
        Ok(Self { raw: map })
    }

    /// Serialize the backing object with deterministic (sorted) key order.
    ///
    /// Unknown keys are preserved verbatim.
    pub fn to_json(&self) -> Value {
        let sorted: BTreeMap<&str, &Value> =
            self.raw.iter().map(|(k, v)| (k.as_str(), v)).collect();
        let mut map = Map::new();
        for (key, value) in sorted {
            map.insert(key.to_string(), value.clone());
        }
        Value::Object(map)
    }

    /// Build a canonical element of `kind` with the given geometry and the
    /// Excalidraw default style.
    ///
    /// The seed is derived from `id` so fixtures are reproducible.
    pub fn new(kind: ElementKind, id: &str, x: f64, y: f64, width: f64, height: f64) -> Self {
        let mut raw = Map::new();
        raw.insert("type".into(), Value::String(kind.as_wire().to_string()));
        raw.insert("id".into(), Value::String(id.to_string()));
        raw.insert("x".into(), Value::from(x));
        raw.insert("y".into(), Value::from(y));
        raw.insert("width".into(), Value::from(width));
        raw.insert("height".into(), Value::from(height));
        raw.insert("angle".into(), Value::from(0.0));
        raw.insert("groupIds".into(), Value::Array(Vec::new()));
        raw.insert("frameId".into(), Value::Null);
        raw.insert("boundElements".into(), Value::Array(Vec::new()));
        raw.insert("updated".into(), Value::from(0_u64));
        raw.insert("link".into(), Value::Null);
        raw.insert("locked".into(), Value::from(false));
        raw.insert("isDeleted".into(), Value::from(false));
        raw.insert("version".into(), Value::from(1_u64));
        raw.insert("versionNonce".into(), Value::from(seed_for(id, 0, 1)));
        match kind {
            ElementKind::Line | ElementKind::Arrow | ElementKind::Freedraw => {
                raw.insert("points".into(), Value::Array(Vec::new()));
                raw.insert("startBinding".into(), Value::Null);
                raw.insert("endBinding".into(), Value::Null);
                raw.insert("startArrowhead".into(), Value::Null);
                raw.insert("endArrowhead".into(), Value::Null);
                raw.insert("lastCommittedPoint".into(), Value::Null);
            }
            ElementKind::Text => {
                raw.insert("text".into(), Value::String(String::new()));
                raw.insert("originalText".into(), Value::String(String::new()));
                raw.insert("containerId".into(), Value::Null);
                raw.insert("autoResize".into(), Value::from(true));
            }
            ElementKind::Frame => {
                raw.insert("name".into(), Value::String("Frame".to_string()));
            }
            ElementKind::Image => {
                raw.insert("fileId".into(), Value::Null);
                raw.insert("status".into(), Value::String("pending".to_string()));
            }
            _ => {}
        }
        let mut element = Self { raw };
        let style = Style {
            seed: seed_for(id, 0, 1),
            ..Style::default()
        };
        element.set_style(&style);
        element
    }

    /// The element kind parsed from `type`. Unknown kinds are preserved.
    pub fn kind(&self) -> ElementKind {
        ElementKind::from_wire(self.str_field("type").unwrap_or_default())
    }

    /// The element id.
    pub fn id(&self) -> &str {
        self.str_field("id").unwrap_or_default()
    }

    /// Set the element id.
    pub fn set_id(&mut self, id: &str) {
        self.raw.insert("id".into(), Value::String(id.to_string()));
    }

    /// The element origin `x`.
    pub fn x(&self) -> f64 {
        self.f64_field("x", 0.0)
    }

    /// Set the element origin `x`.
    pub fn set_x(&mut self, x: f64) {
        self.raw.insert("x".into(), Value::from(x));
    }

    /// The element origin `y`.
    pub fn y(&self) -> f64 {
        self.f64_field("y", 0.0)
    }

    /// Set the element origin `y`.
    pub fn set_y(&mut self, y: f64) {
        self.raw.insert("y".into(), Value::from(y));
    }

    /// The element width.
    pub fn width(&self) -> f64 {
        self.f64_field("width", 0.0)
    }

    /// Set the element width.
    pub fn set_width(&mut self, w: f64) {
        self.raw.insert("width".into(), Value::from(w));
    }

    /// The element height.
    pub fn height(&self) -> f64 {
        self.f64_field("height", 0.0)
    }

    /// Set the element height.
    pub fn set_height(&mut self, h: f64) {
        self.raw.insert("height".into(), Value::from(h));
    }

    /// The rotation angle in radians.
    pub fn angle(&self) -> f64 {
        self.f64_field("angle", 0.0)
    }

    /// Set the rotation angle in radians.
    pub fn set_angle(&mut self, a: f64) {
        self.raw.insert("angle".into(), Value::from(a));
    }

    /// Read the style model from the element.
    ///
    /// The legacy `strokeSharpness` string is honoured when `roundness` is
    /// absent, so pre-v2 boards keep their rounded corners. Excalidraw's
    /// `restore()` maps `strokeSharpness: "round"` to a `roundness` object —
    /// `LEGACY` (1) for linear elements and `PROPORTIONAL_RADIUS` (2) for
    /// shapes — so a legacy board renders with the same corner radius the web
    /// app gives it.
    pub fn style(&self) -> Style {
        let roundness = self
            .raw
            .get("roundness")
            .filter(|v| !v.is_null())
            .and_then(Roundness::from_json);
        let kind = self.kind();
        let legacy_round = roundness.is_none()
            && StrokeSharpness::from_wire(self.str_field("strokeSharpness").unwrap_or(""))
                == StrokeSharpness::Round;
        let roundness = roundness.or_else(|| {
            legacy_round.then(|| Roundness {
                kind: if kind.is_linear() { 1 } else { 2 },
                value: None,
            })
        });
        let sharpness = match &roundness {
            Some(r) if r.kind != 0 => StrokeSharpness::Round,
            Some(_) => StrokeSharpness::Sharp,
            None => StrokeSharpness::from_wire(self.str_field("strokeSharpness").unwrap_or("")),
        };
        Style {
            stroke_color: self
                .str_field("strokeColor")
                .unwrap_or(DEFAULT_STROKE_COLOR)
                .to_string(),
            background_color: self
                .str_field("backgroundColor")
                .unwrap_or(DEFAULT_BACKGROUND_COLOR)
                .to_string(),
            fill_style: FillStyle::from_wire(self.str_field("fillStyle").unwrap_or("")),
            stroke_width: self.f64_field("strokeWidth", 2.0),
            stroke_style: StrokeStyle::from_wire(self.str_field("strokeStyle").unwrap_or("")),
            roughness: self.f64_field("roughness", 1.0),
            opacity: self.f64_field("opacity", 100.0),
            seed: self.raw.get("seed").and_then(Value::as_u64).unwrap_or(0),
            stroke_sharpness: sharpness,
            roundness,
            font_family: FontFamily::from_wire(
                self.raw
                    .get("fontFamily")
                    .and_then(Value::as_u64)
                    .unwrap_or(2),
            ),
            font_size: self.f64_field("fontSize", 20.0),
            text_align: TextAlign::from_wire(self.str_field("textAlign").unwrap_or("")),
            vertical_align: VerticalAlign::from_wire(self.str_field("verticalAlign").unwrap_or("")),
            line_height: self.f64_field("lineHeight", 1.25),
            bowing: 1.0,
        }
    }

    /// Write the style model into the element.
    ///
    /// The canonical `roundness` object is written; a legacy
    /// `strokeSharpness` key, when present, is replaced by it.
    pub fn set_style(&mut self, style: &Style) {
        self.set_str("strokeColor", &style.stroke_color);
        self.set_str("backgroundColor", &style.background_color);
        self.set_str("fillStyle", style.fill_style.as_wire());
        self.raw
            .insert("strokeWidth".into(), Value::from(style.stroke_width));
        self.set_str("strokeStyle", style.stroke_style.as_wire());
        self.raw
            .insert("roughness".into(), Value::from(style.roughness));
        self.raw
            .insert("opacity".into(), Value::from(style.opacity));
        self.raw.insert("seed".into(), Value::from(style.seed));
        self.raw.insert(
            "fontFamily".into(),
            Value::from(style.font_family.as_wire()),
        );
        self.raw
            .insert("fontSize".into(), Value::from(style.font_size));
        self.set_str("textAlign", style.text_align.as_wire());
        self.set_str("verticalAlign", style.vertical_align.as_wire());
        self.raw
            .insert("lineHeight".into(), Value::from(style.line_height));
        // The legacy field is superseded by `roundness`; drop it so reads and
        // writes stay consistent after a style edit.
        self.raw.remove("strokeSharpness");
        match (&style.roundness, style.stroke_sharpness) {
            (Some(roundness), _) => {
                self.raw.insert("roundness".into(), roundness.to_json());
            }
            (None, StrokeSharpness::Round) => {
                self.raw.insert(
                    "roundness".into(),
                    Roundness {
                        kind: 3,
                        value: None,
                    }
                    .to_json(),
                );
            }
            (None, StrokeSharpness::Sharp) => {
                self.raw.remove("roundness");
            }
        }
    }

    /// Group ids the element belongs to.
    pub fn group_ids(&self) -> Vec<String> {
        self.string_array("groupIds")
    }

    /// Replace the group ids.
    pub fn set_group_ids(&mut self, ids: &[String]) {
        self.set_string_array("groupIds", ids);
    }

    /// The owning frame id, when any.
    pub fn frame_id(&self) -> Option<String> {
        self.str_field("frameId").map(str::to_string)
    }

    /// Set or clear the owning frame id.
    pub fn set_frame_id(&mut self, id: Option<&str>) {
        self.set_optional_str("frameId", id);
    }

    /// Whether the element is locked against dragging.
    pub fn locked(&self) -> bool {
        self.raw
            .get("locked")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    }

    /// Set the locked flag.
    pub fn set_locked(&mut self, locked: bool) {
        self.raw.insert("locked".into(), Value::from(locked));
    }

    /// Whether the element is soft-deleted (`isDeleted`).
    pub fn is_deleted(&self) -> bool {
        self.raw
            .get("isDeleted")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    }

    /// Set the soft-deleted flag.
    pub fn set_deleted(&mut self, deleted: bool) {
        self.raw.insert("isDeleted".into(), Value::from(deleted));
    }

    /// The hyperlink attached to the element, when non-empty.
    pub fn link(&self) -> Option<String> {
        self.str_field("link")
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    }
}

/// Deterministic 64-bit seed derived from an element id and edit metadata.
///
/// FNV-1a is used because it is stable across platforms and Rust versions;
/// Excalidraw only requires the seed to be stable for a given element.
pub fn seed_for(id: &str, now_ms: u64, version: u64) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in id.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    for byte in now_ms
        .to_le_bytes()
        .iter()
        .chain(version.to_le_bytes().iter())
    {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Point;

    #[test]
    fn unknown_fields_survive_typed_writes() {
        let mut value = serde_json::json!({
            "type": "rectangle",
            "id": "r1",
            "x": 0.0,
            "y": 0.0,
            "width": 10.0,
            "height": 10.0,
            "customFlag": {"nested": [1, 2, 3]},
        });
        value["customFlag"]["extra"] = serde_json::json!(true);
        let mut element = Element::from_json(value).unwrap();
        element.set_x(42.0);
        let out = element.to_json();
        assert_eq!(out["customFlag"]["nested"], serde_json::json!([1, 2, 3]));
        assert_eq!(out["customFlag"]["extra"], serde_json::json!(true));
        assert_eq!(out["x"], serde_json::json!(42.0));
    }

    #[test]
    fn legacy_stroke_sharpness_is_read() {
        let element = Element::from_json(serde_json::json!({
            "type": "rectangle",
            "id": "r1",
            "strokeSharpness": "round",
        }))
        .unwrap();
        assert_eq!(element.style().stroke_sharpness, StrokeSharpness::Round);
    }

    #[test]
    fn points_round_trip_as_tuples() {
        let mut element = Element::new(ElementKind::Arrow, "a1", 0.0, 0.0, 10.0, 10.0);
        element.set_points(&[Point { x: 0.0, y: 0.0 }, Point { x: 5.0, y: 5.0 }]);
        assert_eq!(
            element.to_json()["points"][1],
            serde_json::json!([5.0, 5.0])
        );
        assert_eq!(element.points().len(), 2);
    }

    #[test]
    fn missing_type_is_an_error() {
        let err = Element::from_json(serde_json::json!({"id": "a"})).unwrap_err();
        assert!(matches!(err, PkmError::Canvas(_)));
    }
}
