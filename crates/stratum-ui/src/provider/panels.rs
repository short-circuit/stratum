//! Panel data DTOs and the [`PanelData`] provider seam.
//!
//! This module is the data contract for the native panels (Search, Query, Ask
//! Notes, Templates, Flashcards, Reminders, Plugins, Graph and Whiteboards).
//! Every method has a safe default so existing providers and window-free tests
//! keep compiling; the real implementation lives in [`super::app_panels`] and
//! the deterministic fixture in [`super::fixture::panels`].
//!
//! The catalog entry types are re-exported from `stratum-app` so hosts never
//! maintain a second copy of the catalog schema.

pub mod graph;
pub mod plugins;
pub mod whiteboard;
pub use graph::{
    GraphComponentRow, GraphEdgeRow, GraphNodeRow, GraphOrphanRow, GraphPanelData, GraphSettingsRow,
};
pub use plugins::{
    InstalledThemeRow, PluginCatalogEntry, PluginCatalogSnapshot, PluginRow, PluginTestResult,
    ThemeCatalogEntry, ThemeCatalogSnapshot, ThemeValuesRow,
};
pub use whiteboard::{LibraryItemRow, WhiteboardRow};

/// One full-text search hit.
#[derive(Clone, Debug, PartialEq)]
pub struct SearchHit {
    /// Block id.
    pub block_id: String,
    /// Vault-relative page path.
    pub page_path: String,
    /// Highlighted snippet.
    pub snippet: String,
    /// Full block content.
    pub content: String,
    /// Relevance score.
    pub score: f32,
}

/// A Datalog result table.
#[derive(Clone, Debug, PartialEq)]
pub struct QueryTable {
    /// Column labels.
    pub columns: Vec<String>,
    /// Row values.
    pub rows: Vec<Vec<String>>,
}

/// One saved query row.
#[derive(Clone, Debug, PartialEq)]
pub struct SavedQueryRow {
    /// Query name.
    pub name: String,
    /// Datalog source.
    pub query: String,
    /// Last update timestamp.
    pub updated_at: String,
}

impl SavedQueryRow {
    /// Build a row with no timestamp (used by provider defaults).
    pub fn new(name: &str, query: &str) -> Self {
        Self {
            name: name.to_string(),
            query: query.to_string(),
            updated_at: String::new(),
        }
    }
}

/// One template row.
#[derive(Clone, Debug, PartialEq)]
pub struct TemplateRow {
    /// Template name.
    pub name: String,
    /// Vault-relative template path.
    pub path: String,
    /// Optional description.
    pub description: Option<String>,
    /// Raw template content (used to derive the placeholder chips).
    pub content: String,
    /// Variables referenced by the template (`{{name}}` without braces).
    pub variables: Vec<String>,
}

/// One flashcard row.
#[derive(Clone, Debug, PartialEq)]
pub struct FlashcardRow {
    /// Card/block id.
    pub id: String,
    /// Question side.
    pub front: String,
    /// Answer side.
    pub back: String,
    /// Source page path.
    pub page_path: String,
    /// SM-2 ease factor.
    pub ease_factor: f64,
    /// SM-2 interval in days.
    pub interval_days: u32,
    /// SM-2 repetition count.
    pub repetitions: u32,
    /// Next review date.
    pub next_review: String,
}

/// One block reminder.
#[derive(Clone, Debug, PartialEq)]
pub struct ReminderRow {
    /// Block id.
    pub block_id: String,
    /// Vault-relative page path.
    pub page_path: String,
    /// Optional page title.
    pub page_title: Option<String>,
    /// Block content.
    pub content: String,
    /// RFC 3339 due timestamp.
    pub due: String,
}

/// One citation returned by Ask Notes.
#[derive(Clone, Debug, PartialEq)]
pub struct AskSource {
    /// Cited page path.
    pub path: String,
    /// Cited snippet.
    pub snippet: String,
    /// Relevance score.
    pub score: f32,
}

/// An Ask Notes answer.
#[derive(Clone, Debug, PartialEq)]
pub struct AskAnswer {
    /// Answer text.
    pub answer: String,
    /// Citations.
    pub sources: Vec<AskSource>,
    /// Whether any citation was produced.
    pub had_sources: bool,
}

/// Stable error for providers that cannot answer a panel request.
pub const PANEL_DATA_NOT_AVAILABLE: &str = "panel_data_not_available";

/// Panel data operations.
///
/// Defaults keep the trait non-breaking: a provider that only implements the
/// shell surface still compiles. Methods whose empty result would mislead the
/// UI (`run_query`, `ask_notes`, `plugin_catalog`, `theme_catalog`,
/// `apply_theme`) return [`PANEL_DATA_NOT_AVAILABLE`] instead.
pub trait PanelData: Send + Sync {
    /// Full-text search with a result limit.
    fn search_blocks(&self, _query: &str, _limit: usize) -> Result<Vec<SearchHit>, String> {
        Ok(Vec::new())
    }

    /// Tag search.
    fn search_by_tag(&self, _tag: &str) -> Result<Vec<SearchHit>, String> {
        Ok(Vec::new())
    }

    /// Rebuild the search index, returning a human-readable summary.
    fn rebuild_search_index(&self) -> Result<String, String> {
        Ok(String::new())
    }

    /// Execute a Datalog query.
    fn run_query(&self, _datalog: &str) -> Result<QueryTable, String> {
        Err(PANEL_DATA_NOT_AVAILABLE.to_string())
    }

    /// List saved queries.
    fn saved_queries(&self) -> Result<Vec<SavedQueryRow>, String> {
        Ok(Vec::new())
    }

    /// Save (create or overwrite) a named query.
    fn save_query(&self, name: &str, query: &str) -> Result<SavedQueryRow, String> {
        Ok(SavedQueryRow::new(name, query))
    }

    /// Rename a saved query.
    fn rename_query(&self, _old_name: &str, new_name: &str) -> Result<SavedQueryRow, String> {
        Ok(SavedQueryRow::new(new_name, ""))
    }

    /// Delete a saved query.
    fn delete_query(&self, _name: &str) -> Result<(), String> {
        Ok(())
    }

    /// Ask the knowledge base a question.
    fn ask_notes(&self, _question: &str) -> Result<AskAnswer, String> {
        Err(PANEL_DATA_NOT_AVAILABLE.to_string())
    }

    /// List templates.
    fn templates(&self) -> Result<Vec<TemplateRow>, String> {
        Ok(Vec::new())
    }

    /// Apply a template to `target`, returning the rendered content.
    fn apply_template(
        &self,
        _name: &str,
        _target: &str,
        _variables: Vec<(String, String)>,
    ) -> Result<String, String> {
        Ok(String::new())
    }

    /// Install the bundled starter templates.
    fn install_starter_templates(&self) -> Result<Vec<String>, String> {
        Ok(Vec::new())
    }

    /// List flashcards.
    fn flashcards(&self) -> Result<Vec<FlashcardRow>, String> {
        Ok(Vec::new())
    }

    /// Record an SM-2 review.
    fn review_card(&self, _id: &str, _quality: u8, _page_path: &str) -> Result<(), String> {
        Ok(())
    }

    /// List reminders.
    fn reminders(&self) -> Result<Vec<ReminderRow>, String> {
        Ok(Vec::new())
    }

    /// Reminders that are due now (the shell decides whether to toast).
    fn due_reminders(&self) -> Vec<ReminderRow> {
        Vec::new()
    }

    /// Snooze a reminder by `minutes`.
    fn snooze_reminder(
        &self,
        _page_path: &str,
        _block_id: &str,
        _minutes: u32,
    ) -> Result<(), String> {
        Ok(())
    }

    /// Clear a reminder.
    fn clear_reminder(&self, _page_path: &str, _block_id: &str) -> Result<(), String> {
        Ok(())
    }

    /// Set an arbitrary reminder due time (`None` clears it).
    fn set_reminder(
        &self,
        _page_path: &str,
        _block_id: &str,
        _due: Option<String>,
    ) -> Result<(), String> {
        Ok(())
    }

    /// List installed plugins.
    fn plugins(&self) -> Result<Vec<PluginRow>, String> {
        Ok(Vec::new())
    }

    /// Enable or disable a plugin.
    fn set_plugin_enabled(&self, id: &str, enabled: bool) -> Result<PluginRow, String> {
        let mut row = PluginRow::empty(id);
        row.enabled = enabled;
        row.status = if enabled { "ready" } else { "disabled" }.to_string();
        Ok(row)
    }

    /// Reload a plugin.
    fn reload_plugin(&self, id: &str) -> Result<PluginRow, String> {
        Ok(PluginRow::empty(id))
    }

    /// Uninstall a plugin.
    fn uninstall_plugin(&self, _id: &str) -> Result<(), String> {
        Ok(())
    }

    /// Install a plugin from a local WASM path.
    fn install_plugin_path(&self, path: &str) -> Result<PluginRow, String> {
        Ok(PluginRow::empty(path))
    }

    /// Install a plugin from a catalog URL.
    fn install_plugin_url(
        &self,
        _url: &str,
        _sha256: Option<&str>,
        id: &str,
    ) -> Result<PluginRow, String> {
        Ok(PluginRow::empty(id))
    }

    /// Read a note through the plugin host API.
    fn plugin_note_read(&self, _path: &str) -> Result<String, String> {
        Ok(String::new())
    }

    /// Run a plugin-style HTTP request, returning `status + body`.
    fn plugin_http_request(&self, _url: &str, _method: &str) -> Result<String, String> {
        Ok(String::new())
    }

    /// Fetch the plugin catalog.
    fn plugin_catalog(&self) -> Result<PluginCatalogSnapshot, String> {
        Err(PANEL_DATA_NOT_AVAILABLE.to_string())
    }

    /// Fetch the theme catalog.
    fn theme_catalog(&self) -> Result<ThemeCatalogSnapshot, String> {
        Err(PANEL_DATA_NOT_AVAILABLE.to_string())
    }

    /// List installed themes.
    fn installed_themes(&self) -> Vec<InstalledThemeRow> {
        Vec::new()
    }

    /// Install a theme by catalog id.
    fn install_theme(&self, id: &str) -> Result<InstalledThemeRow, String> {
        Ok(InstalledThemeRow {
            id: id.to_string(),
            name: id.to_string(),
            active: true,
        })
    }

    /// Apply an installed theme, returning its values.
    fn apply_theme(&self, _id: &str) -> Result<ThemeValuesRow, String> {
        Err(PANEL_DATA_NOT_AVAILABLE.to_string())
    }

    /// Uninstall a theme.
    fn uninstall_theme(&self, _id: &str) -> Result<(), String> {
        Ok(())
    }

    /// Read the graph panel data (nodes, edges, components and orphans).
    fn graph_panel_data(&self) -> Result<GraphPanelData, String> {
        Ok(GraphPanelData::empty())
    }

    /// Read the persisted graph settings from the vault configuration.
    ///
    /// Returns `Ok(None)` when the provider has no persisted graph settings, in
    /// which case the panel falls back to its canonical defaults.
    fn graph_settings(&self) -> Result<Option<GraphSettingsRow>, String> {
        Ok(None)
    }

    /// Persist the graph settings to the vault configuration.
    ///
    /// `node_cap` is not part of [`GraphSettingsRow`]; it stays client-side.
    fn save_graph_settings(&self, _settings: GraphSettingsRow) -> Result<(), String> {
        Ok(())
    }

    /// List whiteboards.
    fn whiteboards(&self) -> Result<Vec<WhiteboardRow>, String> {
        Ok(Vec::new())
    }

    /// Load a whiteboard's raw `.excalidraw` JSON.
    fn load_whiteboard(&self, _name: &str) -> Result<String, String> {
        Ok(String::new())
    }

    /// Save a whiteboard's raw `.excalidraw` JSON.
    fn save_whiteboard(&self, _name: &str, _content: &str) -> Result<(), String> {
        Ok(())
    }

    /// Create an empty whiteboard and return its row.
    fn create_whiteboard(&self, _name: &str) -> Result<WhiteboardRow, String> {
        Ok(WhiteboardRow::new(_name, String::new()))
    }

    /// Rename a whiteboard.
    fn rename_whiteboard(&self, _old_name: &str, _new_name: &str) -> Result<(), String> {
        Ok(())
    }

    /// Delete one or more whiteboards.
    fn delete_whiteboards(&self, _names: Vec<String>) -> Result<(), String> {
        Ok(())
    }

    /// Read the personal whiteboard library.
    fn whiteboard_library(&self) -> Result<Vec<LibraryItemRow>, String> {
        Ok(Vec::new())
    }

    /// Persist the personal whiteboard library from raw `.excalidrawlib` JSON.
    fn save_whiteboard_library(&self, _json: &str) -> Result<(), String> {
        Ok(())
    }

    /// Write an exported whiteboard image, returning its vault-relative path.
    fn write_canvas_export(
        &self,
        _name: &str,
        _extension: &str,
        _bytes: Vec<u8>,
    ) -> Result<String, String> {
        Ok(String::new())
    }
}
