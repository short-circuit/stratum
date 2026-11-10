//! Deterministic fixture provider.
//!
//! [`FixtureProvider`] backs tests, `STRATUM_FIXTURE=1` and the capture
//! harness. Pages are stored behind a mutex so the editor can save and reload
//! them. The block generator lives in [`blocks`]; this module owns the snapshot,
//! journal/voice overrides and asset bookkeeping.
//!
//! The fixture journal data is fixed: ten past entries running backwards day by
//! day from 2026-11-09 to 2026-10-31, with 2026-11-10 treated as today. Voice
//! capture reports a configured STT endpoint, a `assets/voice/fixture.flac`
//! recording and two inserted blocks so the journal voice flow is testable
//! without a microphone.

use std::collections::HashMap;
use std::sync::Mutex;

use stratum_editor::menus::{AiCall, AiCallKind};
use stratum_editor::{
    asset_refs_in_document, dto_to_blocks, AttachmentRow, BlockDto, EditorDocument,
};
use uuid::Uuid;

use super::journal::{journal_dates_from_paths, journal_path};
use super::{
    attachment_row_with_mime, derive_recents, file_extension, file_stem, mime_for_extension,
    BacklinkRow, BacklinkSnippet, ConnectionRow, PageInfo, RecentPage, ReindexSummary, ShellData,
    ShellSnapshot, VaultStats,
};
use panels::PanelState;

mod blocks;
mod page;
mod panels;
#[cfg(test)]
mod tests;

pub use blocks::{fixture_blocks, fixture_journal_blocks};

/// The fixture's notion of today, matching the capture harness.
pub const FIXTURE_TODAY: &str = "2026-11-10";
/// Past fixture journal dates, newest first, continuing backwards day by day.
///
/// Ten past days keep the stacked journal taller than the 1280x720 capture
/// window, so centring a selected past day produces a real scroll. The first
/// entry stays the newest past day (`FIXTURE_TODAY` is the day after it).
pub const FIXTURE_JOURNAL_DATES: [&str; 10] = [
    "2026-11-09",
    "2026-11-08",
    "2026-11-07",
    "2026-11-06",
    "2026-11-05",
    "2026-11-04",
    "2026-11-03",
    "2026-11-02",
    "2026-11-01",
    "2026-10-31",
];
/// Recording path reported by the fixture voice capture.
pub const FIXTURE_RECORDING: &str = "assets/voice/fixture.flac";
/// The fixture page used by the page-panel captures.
pub const FIXTURE_PAGE: &str = "pages/fixture.md";
/// Deterministic fixture block revealed by `STRATUM_HIGHLIGHT`.
pub const FIXTURE_HIGHLIGHT_BLOCK: &str = "413cf12c-969f-54ef-bf75-40ffd89e8618";
/// Source page of the fixture's linked backlinks.
pub const FIXTURE_BACKLINK_SOURCE: &str = "pages/Project Notes.md";
/// Anchor block id used by the fixture backlink snippet.
pub const FIXTURE_BACKLINK_ANCHOR: &str = "11111111-1111-5111-8111-111111111111";

/// Deterministic provider used by tests and `STRATUM_FIXTURE=1`.
pub struct FixtureProvider {
    snapshot: ShellSnapshot,
    export_pages: u64,
    pages: Mutex<HashMap<String, Vec<BlockDto>>>,
    assets: Mutex<HashMap<String, AttachmentRow>>,
    panels: Mutex<PanelState>,
}

impl FixtureProvider {
    /// Create the deterministic fixture snapshot.
    ///
    /// `STRATUM_FIXTURE_BLOCKS` controls the Welcome page size: 10 000 selects
    /// the perf-capture document, anything else uses the small page.
    pub fn new() -> Self {
        let welcome_blocks = std::env::var("STRATUM_FIXTURE_BLOCKS")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .filter(|count| *count > 0)
            .unwrap_or(40);
        Self::with_welcome_blocks(welcome_blocks)
    }

    /// Create a fixture whose Welcome page has exactly `welcome_blocks` rows.
    pub fn with_welcome_blocks(welcome_blocks: usize) -> Self {
        let welcome_blocks = welcome_blocks.max(1);
        let mut pages = HashMap::new();
        pages.insert(
            "pages/Welcome.md".to_string(),
            fixture_blocks("pages/Welcome.md", welcome_blocks),
        );
        pages.insert(
            "pages/Project Notes.md".to_string(),
            fixture_blocks("pages/Project Notes.md", 34),
        );
        pages.insert(
            "pages/Reading List.md".to_string(),
            fixture_blocks("pages/Reading List.md", 7),
        );
        pages.insert(FIXTURE_PAGE.to_string(), fixture_blocks(FIXTURE_PAGE, 12));
        for date in FIXTURE_JOURNAL_DATES {
            let path = journal_path(date);
            pages.insert(path.clone(), fixture_journal_blocks(date));
        }
        let raw = vec![
            RecentPage {
                path: FIXTURE_PAGE.to_string(),
                title: "Fixture Note".to_string(),
                block_count: 12,
                modified_at: "2026-11-10T10:00:00Z".to_string(),
            },
            RecentPage {
                path: journal_path(FIXTURE_JOURNAL_DATES[0]),
                title: FIXTURE_JOURNAL_DATES[0].to_string(),
                block_count: 5,
                modified_at: "2026-11-09T09:00:00Z".to_string(),
            },
            RecentPage {
                path: "pages/Welcome.md".to_string(),
                title: "Welcome".to_string(),
                block_count: welcome_blocks as u64,
                modified_at: "2026-11-09T08:00:00Z".to_string(),
            },
            RecentPage {
                path: "pages/Project Notes.md".to_string(),
                title: "Project Notes".to_string(),
                block_count: 34,
                modified_at: "2026-11-08T18:00:00Z".to_string(),
            },
            RecentPage {
                path: "pages/Reading List.md".to_string(),
                title: "Reading List".to_string(),
                block_count: 7,
                modified_at: "2026-11-07T12:00:00Z".to_string(),
            },
        ];
        Self {
            snapshot: ShellSnapshot {
                vault: Some(VaultStats {
                    block_count: 1240,
                    page_count: 87,
                }),
                recents: derive_recents(raw),
            },
            export_pages: 87,
            pages: Mutex::new(pages),
            assets: Mutex::new(HashMap::new()),
            panels: Mutex::new(PanelState::new()),
        }
    }

    /// Insert one imported attachment row and return it.
    ///
    /// The vault path is deterministic: `assets/<stem>-<hash>.<ext>` where the
    /// hash is 12 hex characters of a v5 UUID over the bytes, mirroring the
    /// service's `sha256` naming without a crypto dependency in the shell.
    fn import_row(&self, bytes: &[u8], name: &str, mime: Option<&str>) -> AttachmentRow {
        let stem = sanitize_stem(file_stem(name));
        let ext = file_extension(name);
        let digest = Uuid::new_v5(&Uuid::NAMESPACE_OID, bytes)
            .simple()
            .to_string();
        let ref_path = format!("assets/{stem}-{}.{ext}", &digest[..12]);
        let mime = mime.unwrap_or_else(|| mime_for_extension(ext));
        let row = attachment_row_with_mime(&ref_path, bytes.len() as u64, mime);
        if let Ok(mut assets) = self.assets.lock() {
            assets.insert(ref_path, row.clone());
        }
        row
    }

    /// Create a journal page if it does not exist yet.
    fn ensure_journal_page(&self, date: &str) -> Result<String, String> {
        let path = journal_path(date);
        let mut pages = self
            .pages
            .lock()
            .map_err(|_| "fixture provider lock poisoned".to_string())?;
        pages
            .entry(path.clone())
            .or_insert_with(|| fixture_journal_blocks(date));
        Ok(path)
    }

    /// Append `count` deterministic memo blocks to `page_path`.
    fn append_memo_blocks(
        &self,
        page_path: &str,
        count: usize,
        label: &str,
    ) -> Result<usize, String> {
        let mut pages = self
            .pages
            .lock()
            .map_err(|_| "fixture provider lock poisoned".to_string())?;
        let entry = pages.entry(page_path.to_string()).or_insert_with(Vec::new);
        let base = entry.len();
        for offset in 0..count {
            entry.push(BlockDto {
                id: Uuid::new_v5(
                    &Uuid::NAMESPACE_OID,
                    format!("{page_path}#voice-{base}-{offset}").as_bytes(),
                )
                .to_string(),
                content: format!("{label} {}", base + offset + 1),
                parent_id: None,
                left_id: entry.last().map(|block| block.id.clone()),
                properties: Vec::new(),
                marker: None,
                priority: None,
                collapsed: false,
                heading_level: None,
            });
        }
        Ok(count)
    }
}

impl Default for FixtureProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ShellData for FixtureProvider {
    fn snapshot(&self) -> ShellSnapshot {
        self.snapshot.clone()
    }

    fn refresh(&self) -> ShellSnapshot {
        self.snapshot.clone()
    }

    fn export_html(&self) -> Result<u64, String> {
        Ok(self.export_pages)
    }

    fn load_blocks(&self, path: &str) -> Option<Vec<BlockDto>> {
        self.pages
            .lock()
            .ok()
            .and_then(|pages| pages.get(path).cloned())
    }

    fn save_blocks(&self, path: &str, blocks: Vec<BlockDto>) -> Result<(), String> {
        self.pages
            .lock()
            .map_err(|_| "fixture provider lock poisoned".to_string())?
            .insert(path.to_string(), blocks);
        Ok(())
    }

    fn create_page(&self, path: &str, _title: Option<&str>) -> Result<String, String> {
        // Mirror `StratumService::create_page`: append the extension and reject
        // a path that already exists.
        let path = if path.ends_with(".md") {
            path.to_string()
        } else {
            format!("{path}.md")
        };
        let mut pages = self
            .pages
            .lock()
            .map_err(|_| "fixture provider lock poisoned".to_string())?;
        if pages.contains_key(&path) {
            return Err(format!("Page already exists: {path}"));
        }
        pages.insert(path.clone(), Vec::new());
        Ok(path)
    }

    fn journal_dates(&self) -> Vec<String> {
        let paths = self
            .pages
            .lock()
            .map(|pages| pages.keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        journal_dates_from_paths(paths)
    }

    fn ensure_today_journal(&self) -> Result<String, String> {
        self.ensure_journal_page(FIXTURE_TODAY)
    }

    fn create_journal_day(&self, date: &str) -> Result<String, String> {
        self.ensure_journal_page(date)
    }

    fn repair_db(&self) -> Result<(), String> {
        Ok(())
    }

    fn stt_configured(&self) -> bool {
        true
    }

    fn voice_start(&self, _page_path: &str) -> Result<String, String> {
        Ok(FIXTURE_RECORDING.to_string())
    }

    fn voice_stop(&self) -> Result<(String, f64), String> {
        Ok((FIXTURE_RECORDING.to_string(), 3.0))
    }

    fn voice_transcribe(&self, _recording_path: &str, page_path: &str) -> Result<usize, String> {
        self.append_memo_blocks(page_path, 2, "🎙️ Fixture memo")
    }

    fn voice_insert_clip(
        &self,
        recording_path: &str,
        page_path: &str,
        seconds: f64,
    ) -> Result<(), String> {
        let label = format!("🎙️ Voice memo {recording_path} ({seconds:.1}s)");
        self.append_memo_blocks(page_path, 1, &label).map(|_| ())
    }

    fn ai_run(&self, call: AiCall) -> Result<String, String> {
        Ok(match call.kind {
            AiCallKind::Mermaid => "graph TD\n  A[Start] --> B[End]".to_string(),
            AiCallKind::Transform(_) | AiCallKind::Research | AiCallKind::Interlink => call.text,
        })
    }

    fn page_attachments(&self, page_path: &str) -> Vec<AttachmentRow> {
        let blocks = self.load_blocks(page_path).unwrap_or_default();
        let Ok(document) = EditorDocument::from_blocks(page_path, dto_to_blocks(blocks)) else {
            return Vec::new();
        };
        let Ok(assets) = self.assets.lock() else {
            return Vec::new();
        };
        asset_refs_in_document(&document)
            .into_iter()
            .filter_map(|ref_path| assets.get(&ref_path).cloned())
            .collect()
    }

    fn import_asset_path(&self, path: &str) -> Result<AttachmentRow, String> {
        let bytes = std::fs::read(path).map_err(|error| format!("read asset file: {error}"))?;
        let name = std::path::Path::new(path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("file");
        Ok(self.import_row(&bytes, name, None))
    }

    fn import_asset_bytes(
        &self,
        bytes: Vec<u8>,
        name: &str,
        mime: Option<&str>,
    ) -> Result<AttachmentRow, String> {
        Ok(self.import_row(&bytes, name, mime))
    }

    fn delete_asset(&self, ref_path: &str) -> Result<(), String> {
        self.assets
            .lock()
            .map_err(|_| "fixture provider lock poisoned".to_string())?
            .remove(ref_path);
        Ok(())
    }

    fn page_info(&self, path: &str) -> Option<PageInfo> {
        self.fixture_page_info(path)
    }

    fn reindex_page(&self, path: &str) -> Result<ReindexSummary, String> {
        self.fixture_reindex(path)
    }

    fn normalize_page(&self, path: &str) -> Result<(), String> {
        self.fixture_normalize(path)
    }

    fn move_page(&self, old_path: &str, new_path: &str) -> Result<String, String> {
        self.fixture_move(old_path, new_path)
    }

    fn delete_page(&self, path: &str) -> Result<(), String> {
        self.fixture_delete(path)
    }

    fn backlinks(&self, path: &str) -> Vec<BacklinkRow> {
        self.fixture_backlinks(path)
    }

    fn backlink_snippet(&self, note_id: &str, anchor_id: &str) -> Option<BacklinkSnippet> {
        self.fixture_snippet(note_id, anchor_id)
    }

    fn suggest_connections(&self, path: &str) -> Result<Vec<ConnectionRow>, String> {
        self.fixture_suggestions(path)
    }

    fn insert_block(&self, path: &str, markdown: &str) -> Result<(), String> {
        self.fixture_insert(path, markdown)
    }
}

/// Sanitize a file stem for a deterministic fixture asset name.
///
/// Mirrors `pkm_block::assets::sanitize_stem`: lowercase, keep `[a-z0-9._-]`,
/// replace other characters with `-`, collapse dashes, cap at 64 characters and
/// fall back to `asset` when nothing usable remains.
fn sanitize_stem(name: &str) -> String {
    let mut replaced = String::new();
    for c in name.to_lowercase().chars() {
        if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
            replaced.push(c);
        } else {
            replaced.push('-');
        }
    }
    let mut collapsed = String::new();
    let mut previous_dash = false;
    for c in replaced.chars() {
        if c == '-' {
            if !previous_dash {
                collapsed.push(c);
            }
            previous_dash = true;
        } else {
            collapsed.push(c);
            previous_dash = false;
        }
    }
    let trimmed = collapsed.trim_matches('-');
    if trimmed.is_empty() {
        return "asset".to_string();
    }
    trimmed.chars().take(64).collect()
}
