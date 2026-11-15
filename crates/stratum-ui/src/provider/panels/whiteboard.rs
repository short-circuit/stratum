//! Whiteboard DTOs for the [`crate::provider::panels::PanelData`] seam.
//!
//! The native Whiteboards panel loads boards as raw `.excalidraw` JSON so the
//! canvas crate can parse, edit and serialize them with its byte-stable
//! round-trip. Library rows carry the raw item JSON for the same reason.

/// One whiteboard file.
#[derive(Clone, Debug, PartialEq)]
pub struct WhiteboardRow {
    /// Board name (file stem).
    pub name: String,
    /// Vault-relative `.excalidraw` path.
    pub path: String,
    /// Raw board JSON.
    pub content: String,
}

impl WhiteboardRow {
    /// Build a row from a board name and its raw JSON.
    pub fn new(name: &str, content: String) -> Self {
        Self {
            name: name.to_string(),
            path: format!("whiteboards/{name}.excalidraw"),
            content,
        }
    }
}

/// One library item row.
#[derive(Clone, Debug, PartialEq)]
pub struct LibraryItemRow {
    /// Item id.
    pub id: String,
    /// Optional display name.
    pub name: Option<String>,
    /// Raw library item JSON (including its `elements` array).
    pub json: String,
}

impl LibraryItemRow {
    /// Build a row from a parsed library item.
    pub fn from_item(item: &stratum_canvas::LibraryItem) -> Self {
        Self {
            id: item.id().to_string(),
            name: item.name().map(str::to_string),
            json: item.to_json().to_string(),
        }
    }

    /// Parse the raw JSON back into a library item.
    ///
    /// Returns `None` when the stored JSON is malformed; callers treat such
    /// rows as unavailable rather than failing the whole library load.
    pub fn to_item(&self) -> Option<stratum_canvas::LibraryItem> {
        serde_json::from_str(&self.json)
            .ok()
            .and_then(|value| stratum_canvas::LibraryItem::from_json(value).ok())
    }
}
