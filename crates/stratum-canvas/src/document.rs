//! `.excalidraw` board document: loading, editing and byte-stable saving.
//!
//! [`BoardDocument`] keeps the original bytes of a loaded board together with
//! a dirty flag. An untouched board serializes back to the exact input bytes,
//! which is the "never rewrite untouched boards" guarantee. Once mutated, the
//! document is rewritten with every unknown element field and unknown
//! top-level key preserved, `type`/`version`/`source` carried over from the
//! input, and the current element order.
//!
//! All failures use [`pkm_core::PkmError`].

use pkm_core::{PkmError, PkmResult};
use serde_json::Value;

use crate::document_meta::DocumentMeta;
use crate::element::Element;
use crate::element_kind::ElementKind;
use crate::files;
use crate::geometry::Rect;
use crate::theme::PreviewTheme;

/// A whiteboard document backed by Excalidraw JSON.
#[derive(Debug, Clone)]
pub struct BoardDocument {
    meta: DocumentMeta,
    elements: Vec<Element>,
    original: Option<Vec<u8>>,
    dirty: bool,
}

impl BoardDocument {
    /// Parse `.excalidraw` bytes.
    ///
    /// # Errors
    ///
    /// Returns [`PkmError::Canvas`] when the payload is not a JSON object, when
    /// `type` or `elements` is missing, or when any element is malformed.
    pub fn load(bytes: &[u8]) -> PkmResult<Self> {
        let value: Value = serde_json::from_slice(bytes)?;
        let map = match &value {
            Value::Object(map) => map,
            _ => {
                return Err(PkmError::Canvas(
                    "document is not a JSON object".to_string(),
                ))
            }
        };
        let meta = DocumentMeta::from_json(map)?;
        let element_values = map
            .get("elements")
            .and_then(Value::as_array)
            .ok_or_else(|| PkmError::Canvas("document without elements".to_string()))?;
        let mut elements = Vec::with_capacity(element_values.len());
        for element in element_values {
            elements.push(Element::from_json(element.clone())?);
        }
        Ok(Self {
            meta,
            elements,
            original: Some(bytes.to_vec()),
            dirty: false,
        })
    }

    /// A brand new empty board with `source: "stratum"`.
    pub fn empty() -> Self {
        Self {
            meta: DocumentMeta::new_empty(),
            elements: Vec::new(),
            original: None,
            dirty: false,
        }
    }

    /// The board producer (`source`).
    pub fn source(&self) -> &str {
        &self.meta.source
    }

    /// The board format version (`version`).
    pub fn version(&self) -> u64 {
        self.meta.version
    }

    /// Elements in z-order (file order). Soft-deleted elements are retained.
    pub fn elements(&self) -> &[Element] {
        &self.elements
    }

    /// Visible elements only, in z-order.
    pub fn visible_elements(&self) -> impl Iterator<Item = &Element> {
        self.elements.iter().filter(|element| !element.is_deleted())
    }

    /// Find an element by id, including soft-deleted entries.
    pub fn element(&self, id: &str) -> Option<&Element> {
        self.elements.iter().find(|element| element.id() == id)
    }

    /// Mutable access to an element; marks the document dirty.
    pub fn element_mut(&mut self, id: &str) -> Option<&mut Element> {
        let index = self
            .elements
            .iter()
            .position(|element| element.id() == id)?;
        self.dirty = true;
        self.elements.get_mut(index)
    }

    /// Add an element at the top of the z-order.
    ///
    /// # Errors
    ///
    /// Returns [`PkmError::AlreadyExists`] when the id is already used.
    pub fn add_element(&mut self, element: Element) -> PkmResult<()> {
        if self
            .elements
            .iter()
            .any(|existing| existing.id() == element.id())
        {
            return Err(PkmError::AlreadyExists(format!(
                "element id {}",
                element.id()
            )));
        }
        self.elements.push(element);
        self.dirty = true;
        Ok(())
    }

    /// Remove an element entry entirely. Returns whether it existed.
    pub fn remove_element(&mut self, id: &str) -> bool {
        let Some(index) = self.elements.iter().position(|element| element.id() == id) else {
            return false;
        };
        self.elements.remove(index);
        self.dirty = true;
        true
    }

    /// Move an element to a z-order index.
    ///
    /// # Errors
    ///
    /// Returns [`PkmError::NotFound`] when the id is unknown. Out-of-range
    /// indices are clamped to the last slot.
    pub fn reorder(&mut self, id: &str, index: usize) -> PkmResult<()> {
        let Some(from) = self.elements.iter().position(|element| element.id() == id) else {
            return Err(PkmError::NotFound(format!("element {id}")));
        };
        let element = self.elements.remove(from);
        let target = index.min(self.elements.len());
        self.elements.insert(target, element);
        self.dirty = true;
        Ok(())
    }

    /// Replace the canvas state; marks the document dirty.
    pub fn set_app_state(&mut self, app_state: Value) {
        self.meta.app_state = app_state;
        self.dirty = true;
    }

    /// The current canvas state.
    pub fn app_state(&self) -> &Value {
        &self.meta.app_state
    }

    /// The view background colour from `appState`, defaulting to white.
    pub fn view_background_color(&self) -> String {
        self.meta
            .app_state
            .get("viewBackgroundColor")
            .and_then(Value::as_str)
            .unwrap_or("#ffffff")
            .to_string()
    }

    /// The embedded files map.
    pub fn files(&self) -> &Value {
        &self.meta.files
    }

    /// Insert or replace one embedded file entry; marks the document dirty.
    pub fn set_file(&mut self, file_id: &str, data_url: &str, mime: &str) {
        if files::set_file(&mut self.meta.files, file_id, data_url, mime).is_ok() {
            self.dirty = true;
        }
    }

    /// Insert a data-URL image under a fresh uuid and return the file id.
    ///
    /// # Errors
    ///
    /// Returns [`PkmError::Canvas`] when the URL is not a base64 data URL.
    pub fn add_file_from_data_url(&mut self, data_url: &str) -> PkmResult<String> {
        let mime = files::data_url_mime(data_url)
            .ok_or_else(|| PkmError::Canvas("not a base64 data URL".to_string()))?;
        let file_id = uuid::Uuid::new_v4().to_string();
        self.set_file(&file_id, data_url, mime);
        Ok(file_id)
    }

    /// The stored thumbnail data URL, when present.
    pub fn preview(&self) -> Option<&str> {
        self.meta.preview.as_deref()
    }

    /// The stored thumbnail theme, when present.
    pub fn preview_theme(&self) -> Option<PreviewTheme> {
        self.meta.preview_theme
    }

    /// Store a thumbnail and its theme; marks the document dirty.
    pub fn set_preview(&mut self, data_url: &str, theme: PreviewTheme) {
        self.meta.preview = Some(data_url.to_string());
        self.meta.preview_theme = Some(theme);
        self.dirty = true;
    }

    /// Whether the document has unsaved mutations.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Serialize the document.
    ///
    /// An untouched loaded board returns its original bytes verbatim. A
    /// rewritten board keeps every unknown element field and unknown top-level
    /// key, preserves `type`/`version`/`source`, and emits `elements` in the
    /// current order.
    ///
    /// # Errors
    ///
    /// Returns [`PkmError::Serialization`] if the JSON encoder fails.
    pub fn save(&mut self) -> PkmResult<Vec<u8>> {
        if !self.dirty {
            if let Some(original) = &self.original {
                return Ok(original.clone());
            }
        }
        let elements = self
            .elements
            .iter()
            .map(Element::to_json)
            .collect::<Vec<_>>();
        let map = self.meta.to_map(elements);
        let bytes = serde_json::to_vec(&Value::Object(map))?;
        self.original = Some(bytes.clone());
        self.dirty = false;
        Ok(bytes)
    }

    /// Serialize the document as UTF-8 text.
    ///
    /// # Errors
    ///
    /// Propagates [`BoardDocument::save`] errors and returns
    /// [`PkmError::Canvas`] when the bytes are not valid UTF-8.
    pub fn save_string(&mut self) -> PkmResult<String> {
        String::from_utf8(self.save()?).map_err(|e| PkmError::Canvas(e.to_string()))
    }

    /// Non-deleted frames in z-order.
    pub fn frames(&self) -> Vec<&Element> {
        self.elements
            .iter()
            .filter(|element| element.kind() == ElementKind::Frame && !element.is_deleted())
            .collect()
    }

    /// Find an element by exact id (deep-link resolution helper).
    pub fn find_element_by_id(&self, id: &str) -> Option<&Element> {
        self.element(id)
    }

    /// Find a frame by name, case-insensitively and trimmed.
    pub fn find_frame_by_name(&self, name: &str) -> Option<&Element> {
        let needle = name.trim().to_lowercase();
        if needle.is_empty() {
            return None;
        }
        self.frames().into_iter().find(|frame| {
            frame
                .frame_name()
                .map(|value| value.trim().to_lowercase() == needle)
                .unwrap_or(false)
        })
    }

    /// Non-deleted elements belonging to a frame.
    pub fn frame_children(&self, frame_id: &str) -> Vec<&Element> {
        self.visible_elements()
            .filter(|element| element.frame_id().as_deref() == Some(frame_id))
            .collect()
    }

    /// Bounding box of all visible elements, padded by stroke width.
    pub fn content_bounds(&self) -> Option<Rect> {
        self.visible_elements()
            .map(crate::geometry::element_bounds)
            .reduce(|acc, rect| acc.union(&rect))
    }

    /// Replace the element list (used by the history journal). Does not touch
    /// the dirty flag; callers mark the document themselves.
    pub(crate) fn replace_elements(&mut self, elements: Vec<Element>) {
        self.elements = elements;
    }

    /// Clone the current element list (history snapshots).
    pub(crate) fn elements_snapshot(&self) -> Vec<Element> {
        self.elements.clone()
    }

    /// Mark the document dirty (history/journal mutations).
    pub(crate) fn mark_dirty(&mut self) {
        self.dirty = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_document_saves_a_parseable_board() {
        let mut doc = BoardDocument::empty();
        let bytes = doc.save().unwrap();
        let reloaded = BoardDocument::load(&bytes).unwrap();
        assert_eq!(reloaded.source(), "stratum");
        assert_eq!(reloaded.version(), 2);
        assert!(reloaded.elements().is_empty());
    }

    #[test]
    fn duplicate_ids_are_rejected() {
        let mut doc = BoardDocument::empty();
        let element = Element::new(ElementKind::Rectangle, "r1", 0.0, 0.0, 10.0, 10.0);
        doc.add_element(element.clone()).unwrap();
        assert!(doc.add_element(element).is_err());
    }

    #[test]
    fn content_bounds_union() {
        let mut doc = BoardDocument::empty();
        doc.add_element(Element::new(
            ElementKind::Rectangle,
            "r1",
            0.0,
            0.0,
            10.0,
            10.0,
        ))
        .unwrap();
        doc.add_element(Element::new(
            ElementKind::Rectangle,
            "r2",
            50.0,
            50.0,
            10.0,
            10.0,
        ))
        .unwrap();
        let bounds = doc.content_bounds().unwrap();
        assert!(bounds.right() >= 60.0);
        assert!(bounds.bottom() >= 60.0);
    }
}
