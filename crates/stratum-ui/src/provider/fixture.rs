//! Deterministic fixture provider and page generator.
//!
//! [`FixtureProvider`] backs tests, `STRATUM_FIXTURE=1` and the 10k perf
//! capture. Pages are stored behind a mutex so the editor can save and reload
//! them, and [`fixture_blocks`] produces the same block list for the same path
//! and count every time.
//!
//! `STRATUM_FIXTURE_BLOCKS` controls the Welcome page size; the perf binary
//! selects the size explicitly through [`FixtureProvider::with_welcome_blocks`].

use std::collections::HashMap;
use std::sync::Mutex;

use stratum_editor::menus::{AiCall, AiCallKind};
use stratum_editor::{
    asset_refs_in_document, dto_to_blocks, AttachmentRow, BlockDto, EditorDocument,
};
use uuid::Uuid;

use super::{
    attachment_row_with_mime, derive_recents, file_extension, file_stem, mime_for_extension,
    RecentPage, ShellData, ShellSnapshot, VaultStats,
};

/// Deterministic provider used by tests and `STRATUM_FIXTURE=1`.
pub struct FixtureProvider {
    snapshot: ShellSnapshot,
    export_pages: u64,
    pages: Mutex<HashMap<String, Vec<BlockDto>>>,
    assets: Mutex<HashMap<String, AttachmentRow>>,
}

impl FixtureProvider {
    /// Create the deterministic fixture snapshot.
    ///
    /// The raw list intentionally contains a journal page so the fixture
    /// exercises the same filtering and ordering as the real provider.
    /// `STRATUM_FIXTURE_BLOCKS` controls the Welcome page size: 10 000 selects
    /// the perf-capture document, anything else uses the small ~40-block page.
    pub fn new() -> Self {
        let welcome_blocks = std::env::var("STRATUM_FIXTURE_BLOCKS")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .filter(|count| *count > 0)
            .unwrap_or(40);
        Self::with_welcome_blocks(welcome_blocks)
    }

    /// Create a fixture whose Welcome page has exactly `welcome_blocks` rows.
    ///
    /// The perf capture uses this to build a 10 000-block document without
    /// depending on process environment variables.
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
        let raw = vec![
            RecentPage {
                path: "journals/2026-11-09.md".to_string(),
                title: "2026-11-09".to_string(),
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

/// Build a deterministic page of `count` fixture blocks.
///
/// The generator is shared by [`FixtureProvider`] and the window-free contract
/// tests. Blocks cycle through headings, tasks, wiki-links, tags, fenced code,
/// Mermaid, math, CJK/emoji, quotes and dividers, with a small nested subtree so
/// depth and collapse behaviour are exercised. UUIDs are v5 values derived from
/// the page path and row index, so the same call always returns the same blocks.
pub fn fixture_blocks(path: &str, count: usize) -> Vec<BlockDto> {
    let mut blocks = Vec::with_capacity(count);
    let mut last_at_depth: Vec<Option<String>> = Vec::new();
    for index in 0..count {
        let depth = match index % 9 {
            0 | 6 | 8 => 0,
            1 | 2 | 4 => 1,
            3 => 2,
            _ => 0,
        };
        let id = fixture_uuid(path, index);
        let parent_id = if depth == 0 {
            None
        } else {
            last_at_depth.get(depth - 1).cloned().flatten()
        };
        let left_id = last_at_depth.get(depth).cloned().flatten();
        let (content, marker, priority, heading_level) = fixture_content(index);
        blocks.push(BlockDto {
            id,
            content,
            parent_id,
            left_id,
            properties: Vec::new(),
            marker,
            priority,
            collapsed: false,
            heading_level,
        });
        if last_at_depth.len() <= depth {
            last_at_depth.resize(depth + 1, None);
        }
        last_at_depth[depth] = blocks.last().map(|block| block.id.clone());
        last_at_depth.truncate(depth + 1);
    }
    blocks
}

/// Deterministic v5 UUID for one fixture row.
fn fixture_uuid(path: &str, index: usize) -> String {
    Uuid::new_v5(&Uuid::NAMESPACE_OID, format!("{path}#{index}").as_bytes()).to_string()
}

/// Content and metadata for one fixture row.
fn fixture_content(index: usize) -> (String, Option<String>, Option<String>, Option<u8>) {
    match index % 10 {
        0 => ("Welcome to Stratum".to_string(), None, None, Some(1)),
        1 => (
            "Explore [[Project Notes]] and the reading list.".to_string(),
            None,
            None,
            None,
        ),
        2 => (
            "TODO Review the reading list #research".to_string(),
            Some("TODO".to_string()),
            Some("A".to_string()),
            None,
        ),
        3 => (
            "```rust\nfn main() {\n    println!(\"stratum\");\n}\n```".to_string(),
            None,
            None,
            None,
        ),
        4 => (
            "```mermaid\ngraph TD; A-->B;\n```".to_string(),
            None,
            None,
            None,
        ),
        5 => ("$$\\frac{1}{2}$$".to_string(), None, None, None),
        6 => (
            "你好，世界 🌍 — deterministic CJK and emoji.".to_string(),
            None,
            None,
            None,
        ),
        7 => (
            "> A quoted line from the fixture.".to_string(),
            None,
            None,
            None,
        ),
        8 => ("---".to_string(), None, None, None),
        _ => (
            format!("Row {index} links [[Note {index}]] and tag #inbox"),
            None,
            None,
            None,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stratum_editor::menus::{AiCallScope, AiTransformAction};

    #[test]
    fn fixture_blocks_are_deterministic_and_nested() {
        let first = fixture_blocks("pages/Welcome.md", 40);
        let second = fixture_blocks("pages/Welcome.md", 40);
        assert_eq!(first, second);
        assert_eq!(first.len(), 40);
        assert!(first.iter().any(|block| block.parent_id.is_some()));
        assert!(first
            .iter()
            .any(|block| block.content.starts_with("```mermaid")));
        assert!(first.iter().any(|block| block.content.contains("你好")));
        assert!(first
            .iter()
            .any(|block| block.marker.as_deref() == Some("TODO")));
    }

    #[test]
    fn welcome_size_is_selectable() {
        let provider = FixtureProvider::with_welcome_blocks(10_000);
        let blocks = provider.load_blocks("pages/Welcome.md").expect("page");
        assert_eq!(blocks.len(), 10_000);
    }

    #[test]
    fn fixture_asset_round_trip_is_deterministic() {
        let provider = FixtureProvider::new();
        let bytes = b"\x89PNG\r\n\x1a\nfixture".to_vec();
        let first = provider
            .import_asset_bytes(bytes.clone(), "photo.png", Some("image/png"))
            .expect("import");
        let second = provider
            .import_asset_bytes(bytes, "photo.png", None)
            .expect("import");
        assert_eq!(first, second, "same bytes and name must dedupe");
        assert!(first.ref_path.starts_with("assets/photo-"));
        assert!(first.ref_path.ends_with(".png"));
        assert_eq!(
            first.ref_path.len(),
            "assets/photo-".len() + 12 + ".png".len()
        );
        assert_eq!(first.mime, "image/png");
        assert_eq!(first.size, 15);
        assert_eq!(first.size_label, "15 B");

        // The attachment is only listed once the page content references it.
        assert!(provider.page_attachments("pages/Welcome.md").is_empty());
        let blocks = vec![BlockDto {
            id: Uuid::new_v4().to_string(),
            content: format!("![[{}]]", first.ref_path),
            parent_id: None,
            left_id: None,
            properties: Vec::new(),
            marker: None,
            priority: None,
            collapsed: false,
            heading_level: None,
        }];
        provider
            .save_blocks("pages/Photo.md", blocks)
            .expect("save page");
        assert_eq!(
            provider.page_attachments("pages/Photo.md"),
            vec![first.clone()]
        );

        provider.delete_asset(&first.ref_path).expect("delete");
        assert!(provider.page_attachments("pages/Photo.md").is_empty());
        assert_eq!(provider.delete_asset(&first.ref_path), Ok(()));
    }

    #[test]
    fn fixture_import_path_reads_the_file() {
        let provider = FixtureProvider::new();
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("Report.pdf");
        std::fs::write(&path, b"%PDF-1.7 fixture").expect("write file");
        let row = provider
            .import_asset_path(path.to_str().expect("utf-8 path"))
            .expect("import");
        assert_eq!(row.mime, "application/pdf");
        assert!(row.ref_path.starts_with("assets/report-"));
        assert!(row.ref_path.ends_with(".pdf"));
    }

    #[test]
    fn fixture_ai_run_is_deterministic() {
        let provider = FixtureProvider::new();
        let transform = AiCall {
            kind: AiCallKind::Transform(AiTransformAction::Rewrite),
            scope: AiCallScope::InsertAtCaret,
            text: "hello".into(),
            page_path: None,
        };
        assert_eq!(provider.ai_run(transform), Ok("hello".to_string()));
        let research = AiCall {
            kind: AiCallKind::Research,
            scope: AiCallScope::ReplacePage,
            text: "topic".into(),
            page_path: None,
        };
        assert_eq!(provider.ai_run(research), Ok("topic".to_string()));
        let interlink = AiCall {
            kind: AiCallKind::Interlink,
            scope: AiCallScope::ReplacePage,
            text: "body".into(),
            page_path: Some("pages/x.md".into()),
        };
        assert_eq!(provider.ai_run(interlink), Ok("body".to_string()));
        let mermaid = AiCall {
            kind: AiCallKind::Mermaid,
            scope: AiCallScope::InsertBlockAfterCaret,
            text: "prompt".into(),
            page_path: None,
        };
        assert_eq!(
            provider.ai_run(mermaid),
            Ok("graph TD\n  A[Start] --> B[End]".to_string())
        );
    }
}
