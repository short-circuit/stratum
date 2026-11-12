//! Top-level `.excalidraw` document metadata.
//!
//! A board file is a JSON object with `type`, `version`, `source`, `elements`,
//! `appState`, optional `files`, `preview` and `previewTheme`, plus any
//! unknown keys written by Excalidraw or plugins. [`DocumentMeta`] keeps the
//! known fields typed and preserves every unknown top-level key so a rewritten
//! board does not lose data.

use pkm_core::{PkmError, PkmResult};
use serde_json::{json, Map, Value};

use crate::theme::PreviewTheme;

/// Default `source` value for boards created by Stratum.
pub const DEFAULT_SOURCE: &str = "stratum";
/// Current Excalidraw document version.
pub const DEFAULT_VERSION: u64 = 2;

/// Known top-level keys that [`DocumentMeta`] models explicitly.
const KNOWN_KEYS: &[&str] = &[
    "type",
    "version",
    "source",
    "elements",
    "appState",
    "files",
    "preview",
    "previewTheme",
];

/// Typed document header plus preserved unknown top-level keys.
#[derive(Debug, Clone, PartialEq)]
pub struct DocumentMeta {
    /// Document discriminator (`type`).
    pub doc_type: String,
    /// Format version (`version`).
    pub version: u64,
    /// Producer (`source`).
    pub source: String,
    /// Canvas state (`appState`).
    pub app_state: Value,
    /// Embedded files map (`files`).
    pub files: Value,
    /// Thumbnail data URL (`preview`), when present.
    pub preview: Option<String>,
    /// Thumbnail theme (`previewTheme`), when present.
    pub preview_theme: Option<PreviewTheme>,
    /// Every top-level key that is not modelled above, preserved verbatim.
    pub unknown: Map<String, Value>,
}

impl DocumentMeta {
    /// Header for a brand new empty board.
    pub fn new_empty() -> Self {
        Self {
            doc_type: "excalidraw".to_string(),
            version: DEFAULT_VERSION,
            source: DEFAULT_SOURCE.to_string(),
            app_state: json!({
                "viewBackgroundColor": "#ffffff",
                "gridSize": null,
            }),
            files: json!({}),
            preview: None,
            preview_theme: None,
            unknown: Map::new(),
        }
    }

    /// Parse the header from a top-level JSON object.
    ///
    /// `elements` is ignored here and parsed by the document loader.
    ///
    /// # Errors
    ///
    /// Returns [`PkmError::Canvas`] when `type` is missing or not a string.
    pub fn from_json(map: &Map<String, Value>) -> PkmResult<Self> {
        let doc_type = map
            .get("type")
            .and_then(Value::as_str)
            .ok_or_else(|| PkmError::Canvas("document without type".to_string()))?;
        let version = map
            .get("version")
            .and_then(Value::as_u64)
            .unwrap_or(DEFAULT_VERSION);
        // Keep the producer the file shipped with. Only boards that do not
        // declare one get the Stratum source.
        let source = map
            .get("source")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .unwrap_or(DEFAULT_SOURCE)
            .to_string();
        let app_state = map.get("appState").cloned().unwrap_or_else(|| json!({}));
        let files = map.get("files").cloned().unwrap_or_else(|| json!({}));
        let preview = map
            .get("preview")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        let preview_theme = map
            .get("previewTheme")
            .and_then(Value::as_str)
            .and_then(PreviewTheme::from_wire);
        let mut unknown = Map::new();
        for (key, value) in map {
            if !KNOWN_KEYS.contains(&key.as_str()) {
                unknown.insert(key.clone(), value.clone());
            }
        }
        Ok(Self {
            doc_type: doc_type.to_string(),
            version,
            source,
            app_state,
            files,
            preview,
            preview_theme,
            unknown,
        })
    }

    /// Build the top-level JSON map for serialization.
    ///
    /// Unknown keys are appended after the known fields; the caller supplies
    /// the serialized element array.
    pub fn to_map(&self, elements: Vec<Value>) -> Map<String, Value> {
        let mut map = Map::new();
        map.insert("type".into(), Value::String(self.doc_type.clone()));
        map.insert("version".into(), Value::from(self.version));
        map.insert("source".into(), Value::String(self.source.clone()));
        map.insert("elements".into(), Value::Array(elements));
        map.insert("appState".into(), self.app_state.clone());
        map.insert("files".into(), self.files.clone());
        if let Some(preview) = &self.preview {
            map.insert("preview".into(), Value::String(preview.clone()));
        }
        if let Some(theme) = self.preview_theme {
            map.insert(
                "previewTheme".into(),
                Value::String(theme.as_wire().to_string()),
            );
        }
        for (key, value) in &self.unknown {
            map.insert(key.clone(), value.clone());
        }
        map
    }
}
