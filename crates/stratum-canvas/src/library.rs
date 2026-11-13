//! `.excalidrawlib` library read/write.
//!
//! Two on-disk shapes are supported because both have shipped:
//!
//! * the official v2 object
//!   `{"type":"excalidrawlib","version":2,"libraryItems":[...]}`
//! * the bare array the Stratum backend writes
//!   `[{"id","status","created","elements":[...]}]`
//!
//! A loaded library remembers its shape and original bytes, so an untouched
//! file round-trips byte-for-byte and an edited file is written back in the
//! same shape. Item JSON is preserved verbatim, including unknown fields.

use std::collections::{BTreeMap, HashMap};

use pkm_core::{PkmError, PkmResult};
use serde_json::{Map, Value};

use crate::document::BoardDocument;
use crate::element::{Binding, Element};

/// Shape a library file was loaded in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibraryShape {
    /// Official v2 object with a `libraryItems` array.
    ObjectV2,
    /// Bare JSON array of items.
    BareArray,
}

/// Known top-level keys of the official library object.
const KNOWN_KEYS: &[&str] = &["type", "version", "source", "libraryItems"];

/// One library item, backed by its full original JSON object.
#[derive(Debug, Clone, PartialEq)]
pub struct LibraryItem {
    raw: Map<String, Value>,
}

impl LibraryItem {
    /// Parse one library item.
    ///
    /// # Errors
    ///
    /// Returns [`PkmError::Canvas`] when the value is not an object, `id` is
    /// missing, or `elements` is not an array of valid elements.
    pub fn from_json(value: Value) -> PkmResult<Self> {
        let map = match value {
            Value::Object(map) => map,
            _ => {
                return Err(PkmError::Canvas(
                    "library item is not an object".to_string(),
                ))
            }
        };
        if !map.get("id").map(Value::is_string).unwrap_or(false) {
            return Err(PkmError::Canvas("library item without id".to_string()));
        }
        if !map.get("elements").map(Value::is_array).unwrap_or(false) {
            return Err(PkmError::Canvas(
                "library item without elements".to_string(),
            ));
        }
        Ok(Self { raw: map })
    }

    /// Serialize the item with deterministic key ordering, preserving unknown
    /// fields.
    pub fn to_json(&self) -> Value {
        let sorted: BTreeMap<&str, &Value> =
            self.raw.iter().map(|(k, v)| (k.as_str(), v)).collect();
        let mut map = Map::new();
        for (key, value) in sorted {
            map.insert(key.to_string(), value.clone());
        }
        Value::Object(map)
    }

    /// The item id.
    pub fn id(&self) -> &str {
        self.raw
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or_default()
    }

    /// The publication status, defaulting to `"published"`.
    pub fn status(&self) -> &str {
        self.raw
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("published")
    }

    /// The optional item name.
    pub fn name(&self) -> Option<&str> {
        self.raw.get("name").and_then(Value::as_str)
    }

    /// Parsed copies of the item's elements.
    ///
    /// Malformed elements are skipped so a single bad entry cannot make a
    /// whole library unreadable.
    pub fn elements(&self) -> Vec<Element> {
        let Some(Value::Array(items)) = self.raw.get("elements") else {
            return Vec::new();
        };
        items
            .iter()
            .filter_map(|value| Element::from_json(value.clone()).ok())
            .collect()
    }

    /// Replace the item's elements array.
    pub fn set_elements(&mut self, elements: Vec<Element>) {
        let values = elements.iter().map(Element::to_json).collect();
        self.raw.insert("elements".into(), Value::Array(values));
    }

    /// Clone the item into a document at `(x, y)`.
    ///
    /// Every element gets a fresh uuid, group ids are remapped, bound-element
    /// references (`boundElements`, arrow bindings, `containerId`, `frameId`)
    /// are remapped to the new ids, and positions are offset so the item's
    /// top-left corner lands at `(x, y)`. Returns the new element ids in
    /// insertion order.
    ///
    /// # Errors
    ///
    /// Propagates [`BoardDocument::add_element`] errors.
    pub fn insert_into(&self, doc: &mut BoardDocument, x: f64, y: f64) -> PkmResult<Vec<String>> {
        let mut elements = self.elements();
        if elements.is_empty() {
            return Ok(Vec::new());
        }
        let min_x = elements
            .iter()
            .map(Element::x)
            .fold(f64::INFINITY, f64::min);
        let min_y = elements
            .iter()
            .map(Element::y)
            .fold(f64::INFINITY, f64::min);
        let mut id_map: HashMap<String, String> = HashMap::new();
        let mut group_map: HashMap<String, String> = HashMap::new();
        for element in &elements {
            id_map.insert(element.id().to_string(), uuid::Uuid::new_v4().to_string());
        }
        let mut new_ids = Vec::with_capacity(elements.len());
        for element in &mut elements {
            let Some(new_id) = id_map.get(element.id()).cloned() else {
                continue;
            };
            element.set_id(&new_id);
            element.set_x(x + (element.x() - min_x));
            element.set_y(y + (element.y() - min_y));
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
                binding.element_id = remap(&id_map, &binding.element_id);
                element.set_start_binding(Some(&binding));
            }
            if let Some(mut binding) = element.end_binding() {
                binding.element_id = remap(&id_map, &binding.element_id);
                element.set_end_binding(Some(&binding));
            }
            if let Some(container) = element.container_id() {
                element.set_extra("containerId", Value::String(remap(&id_map, &container)));
            }
            if let Some(frame) = element.frame_id() {
                element.set_frame_id(Some(&remap(&id_map, &frame)));
            }
            new_ids.push(new_id);
        }
        // Push after remapping so add_element's duplicate check sees the final
        // ids. A partial failure leaves earlier inserts in place, matching the
        // document's append-only semantics.
        for element in elements {
            doc.add_element(element)?;
        }
        Ok(new_ids)
    }
}

/// Remap an id when it is part of the cloned item; keep dangling references.
fn remap(map: &HashMap<String, String>, id: &str) -> String {
    map.get(id).cloned().unwrap_or_else(|| id.to_string())
}

/// A `.excalidrawlib` library file.
#[derive(Debug, Clone)]
pub struct Library {
    shape: LibraryShape,
    items: Vec<LibraryItem>,
    doc_type: String,
    version: u64,
    source: String,
    unknown: Map<String, Value>,
    original: Option<Vec<u8>>,
    dirty: bool,
}

impl Library {
    /// Parse `.excalidrawlib` bytes in either supported shape.
    ///
    /// # Errors
    ///
    /// Returns [`PkmError::Canvas`] when the payload is neither a bare array
    /// nor an object with a `libraryItems` array, or when an item is invalid.
    pub fn load(bytes: &[u8]) -> PkmResult<Self> {
        let value: Value = serde_json::from_slice(bytes)?;
        let (shape, items_value, doc_type, version, source, unknown) = match &value {
            Value::Array(items) => (
                LibraryShape::BareArray,
                items.clone(),
                "excalidrawlib".to_string(),
                2,
                "excalidraw".to_string(),
                Map::new(),
            ),
            Value::Object(map) => {
                let items = map
                    .get("libraryItems")
                    .and_then(Value::as_array)
                    .ok_or_else(|| PkmError::Canvas("library without libraryItems".to_string()))?;
                let doc_type = map
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or("excalidrawlib")
                    .to_string();
                let version = map.get("version").and_then(Value::as_u64).unwrap_or(2);
                let source = map
                    .get("source")
                    .and_then(Value::as_str)
                    .unwrap_or("excalidraw")
                    .to_string();
                let mut unknown = Map::new();
                for (key, value) in map {
                    if !KNOWN_KEYS.contains(&key.as_str()) {
                        unknown.insert(key.clone(), value.clone());
                    }
                }
                (
                    LibraryShape::ObjectV2,
                    items.clone(),
                    doc_type,
                    version,
                    source,
                    unknown,
                )
            }
            _ => {
                return Err(PkmError::Canvas(
                    "library is neither an array nor an object".to_string(),
                ))
            }
        };
        let mut items = Vec::with_capacity(items_value.len());
        for item in items_value {
            items.push(LibraryItem::from_json(item)?);
        }
        Ok(Self {
            shape,
            items,
            doc_type,
            version,
            source,
            unknown,
            original: Some(bytes.to_vec()),
            dirty: false,
        })
    }

    /// An empty library in the bare-array shape the backend writes.
    pub fn empty() -> Self {
        Self {
            shape: LibraryShape::BareArray,
            items: Vec::new(),
            doc_type: "excalidrawlib".to_string(),
            version: 2,
            source: "excalidraw".to_string(),
            unknown: Map::new(),
            original: None,
            dirty: false,
        }
    }

    /// The shape this library was loaded in.
    pub fn shape(&self) -> LibraryShape {
        self.shape
    }

    /// Items in file order.
    pub fn items(&self) -> &[LibraryItem] {
        &self.items
    }

    /// Append an item and mark the library dirty.
    pub fn push(&mut self, item: LibraryItem) {
        self.items.push(item);
        self.dirty = true;
    }

    /// Remove an item by id. Returns whether it existed.
    pub fn remove(&mut self, id: &str) -> bool {
        let Some(index) = self.items.iter().position(|item| item.id() == id) else {
            return false;
        };
        self.items.remove(index);
        self.dirty = true;
        true
    }

    /// Save the library in its original shape.
    ///
    /// An untouched library returns its original bytes verbatim.
    ///
    /// # Errors
    ///
    /// Returns [`PkmError::Serialization`] when the JSON encoder fails.
    pub fn save(&mut self) -> PkmResult<Vec<u8>> {
        if !self.dirty {
            if let Some(original) = &self.original {
                return Ok(original.clone());
            }
        }
        let items = self
            .items
            .iter()
            .map(LibraryItem::to_json)
            .collect::<Vec<_>>();
        let value = match self.shape {
            LibraryShape::BareArray => Value::Array(items),
            LibraryShape::ObjectV2 => {
                let mut map = Map::new();
                map.insert("type".into(), Value::String(self.doc_type.clone()));
                map.insert("version".into(), Value::from(self.version));
                map.insert("source".into(), Value::String(self.source.clone()));
                map.insert("libraryItems".into(), Value::Array(items));
                for (key, value) in &self.unknown {
                    map.insert(key.clone(), value.clone());
                }
                Value::Object(map)
            }
        };
        let bytes = serde_json::to_vec(&value)?;
        self.original = Some(bytes.clone());
        self.dirty = false;
        Ok(bytes)
    }

    /// Merge items from another library, skipping ids already present.
    pub fn merge(&mut self, other: &Library) {
        let mut changed = false;
        for item in &other.items {
            if self.items.iter().any(|existing| existing.id() == item.id()) {
                continue;
            }
            self.items.push(item.clone());
            changed = true;
        }
        if changed {
            self.dirty = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element_kind::ElementKind;

    fn item_with_rectangle() -> LibraryItem {
        let mut rectangle = Element::new(ElementKind::Rectangle, "lib-r", 5.0, 5.0, 20.0, 20.0);
        rectangle.set_group_ids(&["g1".to_string()]);
        LibraryItem::from_json(serde_json::json!({
            "id": "item-1",
            "status": "published",
            "created": 0,
            "elements": [rectangle.to_json()],
        }))
        .unwrap()
    }

    #[test]
    fn insert_into_remaps_ids_and_groups() {
        let item = item_with_rectangle();
        let mut doc = BoardDocument::empty();
        let ids = item.insert_into(&mut doc, 100.0, 200.0).unwrap();
        assert_eq!(ids.len(), 1);
        assert_ne!(ids[0], "lib-r");
        let element = doc.element(&ids[0]).unwrap();
        assert_eq!(element.x(), 100.0);
        assert_eq!(element.y(), 200.0);
        assert_ne!(element.group_ids()[0], "g1");
    }

    #[test]
    fn bare_array_round_trips_shape() {
        let mut library = Library::empty();
        library.push(item_with_rectangle());
        let bytes = library.save().unwrap();
        let value: Value = serde_json::from_slice(&bytes).unwrap();
        assert!(value.is_array());
    }
}
