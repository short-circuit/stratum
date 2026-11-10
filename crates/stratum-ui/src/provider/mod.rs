//! Shell data provider.
//!
//! [`ShellData`] is the seam between the gpui shell and the headless service
//! layer. [`AppProvider`] drives the real [`StratumService`] on a Tokio
//! runtime; [`FixtureProvider`] returns a deterministic snapshot for tests and
//! for `STRATUM_FIXTURE=1`, which lets the capture harness render the shell
//! without a vault.
//!
//! Recent-page derivation mirrors `src/stores/recentsStore.ts`: journal pages
//! are excluded and the list is ordered by `modified_at` descending. The
//! ordering is a plain string comparison because the backend already emits
//! RFC 3339 timestamps.
//!
//! The fixture generator lives in [`fixture`] so this module stays under the
//! repository file-length limit.

use std::path::PathBuf;
use std::sync::Arc;

use stratum_app::StratumService;
use stratum_editor::{AssetThumbnail, AttachmentRow, BlockDto};

use convert::{app_block_to_editor, editor_block_to_app};

mod app;
mod app_panels;
mod assets;
mod audio;
mod convert;
mod fixture;
mod journal;
mod page;
pub mod panels;
mod voice;

pub use assets::attachment_row;
pub(crate) use assets::{
    attachment_row_for_bytes, attachment_row_with_mime, file_extension, file_stem,
    mime_for_extension,
};
pub use journal::{journal_dates_from_paths, journal_path, parse_journal_date};
pub use page::{
    clamp_relevance, group_backlinks, normalize_move_path, BacklinkRow, BacklinkSnippet,
    ConnectionRow, PageInfo, ReindexSummary,
};
pub use panels::{
    AskAnswer, AskSource, FlashcardRow, InstalledThemeRow, PanelData, PluginCatalogEntry,
    PluginCatalogSnapshot, PluginRow, PluginTestResult, QueryTable, ReminderRow, SavedQueryRow,
    SearchHit, TemplateRow, ThemeCatalogEntry, ThemeCatalogSnapshot, ThemeValuesRow,
    PANEL_DATA_NOT_AVAILABLE,
};

pub use fixture::{
    fixture_blocks, fixture_journal_blocks, FixtureProvider, FIXTURE_BACKLINK_ANCHOR,
    FIXTURE_BACKLINK_SOURCE, FIXTURE_HIGHLIGHT_BLOCK, FIXTURE_JOURNAL_DATES, FIXTURE_PAGE,
    FIXTURE_RECORDING, FIXTURE_TODAY,
};

/// Vault counters shown in the sidebar header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VaultStats {
    /// Number of blocks in the vault.
    pub block_count: u64,
    /// Number of pages in the vault.
    pub page_count: u64,
}

/// A recent page row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecentPage {
    /// Vault-relative page path.
    pub path: String,
    /// Display title (falls back to the slug in the provider).
    pub title: String,
    /// Number of blocks on the page.
    pub block_count: u64,
    /// RFC 3339 modification timestamp.
    pub modified_at: String,
}

/// The data the shell needs to render one frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShellSnapshot {
    /// Vault counters, absent when the vault cannot be read.
    pub vault: Option<VaultStats>,
    /// Journal-filtered, newest-first recent pages.
    pub recents: Vec<RecentPage>,
}

/// Shell data operations.
///
/// Panel data is a supertrait: every shell provider also answers the
/// [`PanelData`] contract, whose methods carry safe defaults so existing
/// providers and window-free tests keep compiling.
pub trait ShellData: PanelData {
    /// Read the current snapshot.
    fn snapshot(&self) -> ShellSnapshot;
    /// Refresh and return a new snapshot.
    fn refresh(&self) -> ShellSnapshot;
    /// Export the vault as static HTML, returning the number of pages written.
    fn export_html(&self) -> Result<u64, String>;
    /// Load the blocks of one page. `None` means the page has no blocks (or the
    /// provider cannot read it), and the editor starts empty.
    fn load_blocks(&self, _path: &str) -> Option<Vec<BlockDto>> {
        None
    }
    /// Persist every block of one page. Providers that cannot persist pages
    /// accept the write without error.
    fn save_blocks(&self, _path: &str, _blocks: Vec<BlockDto>) -> Result<(), String> {
        Ok(())
    }
    /// Create a page at `path` and return its stored path.
    ///
    /// The provider appends `.md` when the path has no extension, mirroring
    /// `StratumService::create_page`. Providers that cannot create pages return
    /// the path unchanged.
    fn create_page(&self, _path: &str, _title: Option<&str>) -> Result<String, String> {
        Ok(_path.to_string())
    }
    /// Speak `text` aloud. Providers without audio support return `Ok(())`.
    fn speak(&self, _text: String) -> Result<(), String> {
        Ok(())
    }
    /// Run one AI slash request. Providers without AI support echo the input.
    fn ai_run(&self, call: stratum_editor::menus::AiCall) -> Result<String, String> {
        Ok(call.text)
    }
    /// Read a decoded thumbnail for an image asset.
    ///
    /// Providers without asset support return `None`, and the view keeps the
    /// shipped chip fallback instead of an image preview.
    fn read_asset_thumbnail(&self, _ref_path: &str) -> Option<AssetThumbnail> {
        None
    }
    /// The attachment rows referenced by one page.
    ///
    /// Providers without asset support return an empty list; the panel then
    /// shows its empty state.
    fn page_attachments(&self, _page_path: &str) -> Vec<AttachmentRow> {
        Vec::new()
    }
    /// Import a vault-relative file path as an attachment.
    ///
    /// Providers that cannot persist assets return a row for `path` unchanged.
    fn import_asset_path(&self, path: &str) -> Result<AttachmentRow, String> {
        Ok(attachment_row(path))
    }
    /// Import raw bytes (a clipboard paste or drag-and-drop payload).
    ///
    /// Providers that cannot persist assets return a row under `assets/` with
    /// the byte length and the provided or inferred MIME.
    fn import_asset_bytes(
        &self,
        bytes: Vec<u8>,
        name: &str,
        mime: Option<&str>,
    ) -> Result<AttachmentRow, String> {
        Ok(attachment_row_for_bytes(name, bytes.len() as u64, mime))
    }
    /// Delete an attachment by vault-relative ref path.
    ///
    /// Providers without asset support accept the delete without error.
    fn delete_asset(&self, _ref_path: &str) -> Result<(), String> {
        Ok(())
    }
    /// Valid journal dates present in the vault, newest first.
    ///
    /// Providers without journal support return an empty list, so the panel
    /// renders only today's entry.
    fn journal_dates(&self) -> Vec<String> {
        Vec::new()
    }
    /// Ensure the local date's journal page exists and return its path.
    ///
    /// The default derives today's path without touching storage; providers
    /// that can persist pages override it.
    fn ensure_today_journal(&self) -> Result<String, String> {
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        Ok(journal_path(&today))
    }
    /// Create the journal page for `date` and return its path.
    fn create_journal_day(&self, date: &str) -> Result<String, String> {
        Ok(journal_path(date))
    }
    /// Reconcile the block database with the files on disk.
    ///
    /// Providers without a database accept the repair without error.
    fn repair_db(&self) -> Result<(), String> {
        Ok(())
    }
    /// Whether a speech-to-text endpoint is configured.
    fn stt_configured(&self) -> bool {
        false
    }
    /// Start a voice recording destined for `page_path`.
    ///
    /// Returns the recording path; providers without audio return an empty
    /// path so the capture flow still completes.
    fn voice_start(&self, _page_path: &str) -> Result<String, String> {
        Ok(String::new())
    }
    /// Stop the active recording, returning its path and duration in seconds.
    fn voice_stop(&self) -> Result<(String, f64), String> {
        Ok((String::new(), 0.0))
    }
    /// Transcribe a saved clip into `page_path`, returning the inserted count.
    fn voice_transcribe(&self, _recording_path: &str, _page_path: &str) -> Result<usize, String> {
        Ok(0)
    }
    /// Insert a clip-reference placeholder into `page_path`.
    fn voice_insert_clip(
        &self,
        _recording_path: &str,
        _page_path: &str,
        _seconds: f64,
    ) -> Result<(), String> {
        Ok(())
    }
    /// Read page metadata (title, slug, block count, modification time).
    ///
    /// Providers without page metadata return `None`; the header then falls
    /// back to the path's file stem.
    fn page_info(&self, _path: &str) -> Option<PageInfo> {
        None
    }
    /// Reindex one page from disk into SQLite and the search index.
    ///
    /// Providers without an index accept the request and report zero work.
    fn reindex_page(&self, _path: &str) -> Result<ReindexSummary, String> {
        Ok(ReindexSummary::default())
    }
    /// Normalize one page file through the block parser.
    fn normalize_page(&self, _path: &str) -> Result<(), String> {
        Ok(())
    }
    /// Move/rename a page, returning the stored new path.
    ///
    /// Providers without storage return the normalized target path.
    fn move_page(&self, _old_path: &str, new_path: &str) -> Result<String, String> {
        Ok(normalize_move_path(new_path))
    }
    /// Delete a page.
    fn delete_page(&self, _path: &str) -> Result<(), String> {
        Ok(())
    }
    /// Linked references and unlinked mentions for one page.
    fn backlinks(&self, _path: &str) -> Vec<BacklinkRow> {
        Vec::new()
    }
    /// The bounded snippet around a backlink anchor.
    fn backlink_snippet(&self, _note_id: &str, _anchor_id: &str) -> Option<BacklinkSnippet> {
        None
    }
    /// Related-note suggestions for one page.
    fn suggest_connections(&self, _path: &str) -> Result<Vec<ConnectionRow>, String> {
        Ok(Vec::new())
    }
    /// Append one markdown block to a page.
    fn insert_block(&self, _path: &str, _markdown: &str) -> Result<(), String> {
        Ok(())
    }
}

/// Real provider backed by the headless service layer.
pub struct AppProvider {
    service: StratumService,
    runtime: tokio::runtime::Runtime,
}

impl AppProvider {
    /// Create a provider for `vault_path`.
    ///
    /// The runtime is multi-threaded because the service uses
    /// `spawn_blocking` for SQLite and filesystem work.
    pub fn new(vault_path: PathBuf) -> Self {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("failed to build the shell Tokio runtime");
        Self {
            service: StratumService::new(vault_path),
            runtime,
        }
    }

    /// Read vault counters, mapping an unreadable vault to `None`.
    fn read_vault(&self) -> Option<VaultStats> {
        self.runtime
            .block_on(self.service.get_vault_info())
            .ok()
            .map(|info| VaultStats {
                block_count: info.block_count as u64,
                page_count: info.page_count as u64,
            })
    }

    /// Read recent pages, mapping an unreadable page list to an empty list.
    fn read_recents(&self) -> Vec<RecentPage> {
        match self.runtime.block_on(self.service.list_pages()) {
            Ok(list) => derive_recents(
                list.pages
                    .into_iter()
                    .map(|page| RecentPage {
                        path: page.path,
                        title: page.title.unwrap_or(page.slug),
                        block_count: page.block_count as u64,
                        modified_at: page.modified_at,
                    })
                    .collect(),
            ),
            Err(_) => Vec::new(),
        }
    }

    /// Load the editor blocks of one page through the service layer.
    fn read_blocks(&self, path: &str) -> Option<Vec<BlockDto>> {
        self.runtime
            .block_on(self.service.get_blocks(path.to_string()))
            .ok()
            .map(|list| {
                list.blocks
                    .into_iter()
                    .map(app_block_to_editor)
                    .collect::<Vec<_>>()
            })
    }

    /// Persist editor blocks through the service layer.
    fn write_blocks(&self, path: &str, blocks: Vec<BlockDto>) -> Result<(), String> {
        let app_blocks = blocks.into_iter().map(editor_block_to_app).collect();
        self.runtime
            .block_on(self.service.save_blocks(path.to_string(), app_blocks, None))
            .map_err(|error| error.to_string())
    }
}

/// Derive the recent-page list: exclude the journal namespace and sort by
/// `modified_at` descending. The input is consumed; the returned vector keeps
/// the original block counts.
pub fn derive_recents(pages: Vec<RecentPage>) -> Vec<RecentPage> {
    let mut recents: Vec<RecentPage> = pages
        .into_iter()
        .filter(|page| !page.path.starts_with("journals/"))
        .collect();
    recents.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
    recents
}

/// Convenience constructor for the provider selected by the environment.
pub fn provider_from_env(vault_path: PathBuf, fixture: bool) -> Arc<dyn ShellData> {
    if fixture {
        Arc::new(FixtureProvider::new())
    } else {
        Arc::new(AppProvider::new(vault_path))
    }
}

#[cfg(test)]
mod tests;
