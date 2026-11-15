//! Plugin and theme DTOs for the [`crate::provider::panels::PanelData`] seam.
//!
//! Split out of `panels.rs` to keep every provider source file under the
//! repository length limit. The catalog entry types are re-exported from
//! `stratum-app` so hosts never maintain a second copy of the catalog schema.

pub use stratum_app::plugins::catalog::{PluginCatalogEntry, ThemeCatalogEntry};

/// One installed plugin row.
#[derive(Clone, Debug, PartialEq)]
pub struct PluginRow {
    /// Plugin id.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Version.
    pub version: String,
    /// Runtime status.
    pub status: String,
    /// Whether the plugin is enabled.
    pub enabled: bool,
    /// Author.
    pub author: String,
    /// Description.
    pub description: String,
    /// Declared permissions.
    pub permissions: Vec<String>,
    /// Declared hook names.
    pub hooks: Vec<String>,
    /// Optional load/runtime error.
    pub error: Option<String>,
}

impl PluginRow {
    /// Build a minimal row for provider defaults.
    pub fn empty(id: &str) -> Self {
        Self {
            id: id.to_string(),
            name: id.to_string(),
            version: String::new(),
            status: "disabled".to_string(),
            enabled: false,
            author: String::new(),
            description: String::new(),
            permissions: Vec::new(),
            hooks: Vec::new(),
            error: None,
        }
    }
}

/// A plugin catalog snapshot.
#[derive(Clone, Debug, PartialEq)]
pub struct PluginCatalogSnapshot {
    /// Catalog entries.
    pub entries: Vec<PluginCatalogEntry>,
    /// True when served from the disk cache.
    pub offline: bool,
    /// Last successful fetch timestamp.
    pub fetched_at: Option<String>,
    /// Stable error message.
    pub error: Option<String>,
}

/// A theme catalog snapshot.
#[derive(Clone, Debug, PartialEq)]
pub struct ThemeCatalogSnapshot {
    /// Catalog entries.
    pub entries: Vec<ThemeCatalogEntry>,
    /// True when served from the disk cache.
    pub offline: bool,
    /// Last successful fetch timestamp.
    pub fetched_at: Option<String>,
    /// Stable error message.
    pub error: Option<String>,
}

/// One installed theme row.
#[derive(Clone, Debug, PartialEq)]
pub struct InstalledThemeRow {
    /// Theme id.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Whether the theme is active.
    pub active: bool,
}

/// Theme values applied to the shell.
#[derive(Clone, Debug, PartialEq)]
pub struct ThemeValuesRow {
    /// Whether the theme is dark.
    pub dark_mode: bool,
    /// Primary color.
    pub primary_color: String,
    /// Secondary color.
    pub secondary_color: String,
    /// Base font size.
    pub font_size: f64,
}

/// Result of a plugin HTTP smoke test.
#[derive(Clone, Debug, PartialEq)]
pub struct PluginTestResult {
    /// Whether the request succeeded.
    pub ok: bool,
    /// Status/body detail.
    pub detail: String,
}
