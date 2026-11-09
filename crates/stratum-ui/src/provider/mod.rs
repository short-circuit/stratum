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
use stratum_editor::BlockDto;

mod fixture;

pub use fixture::{fixture_blocks, FixtureProvider};

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
pub trait ShellData: Send + Sync {
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

impl ShellData for AppProvider {
    fn snapshot(&self) -> ShellSnapshot {
        ShellSnapshot {
            vault: self.read_vault(),
            recents: self.read_recents(),
        }
    }

    fn refresh(&self) -> ShellSnapshot {
        self.snapshot()
    }

    fn export_html(&self) -> Result<u64, String> {
        self.runtime
            .block_on(self.service.export_html("/tmp/stratum-export".to_string()))
            .map(|result| result.pages_exported as u64)
    }

    fn load_blocks(&self, path: &str) -> Option<Vec<BlockDto>> {
        self.read_blocks(path)
    }

    fn save_blocks(&self, path: &str, blocks: Vec<BlockDto>) -> Result<(), String> {
        self.write_blocks(path, blocks)
    }
}

/// Convert an `stratum-app` block DTO into the editor DTO (field-identical).
fn app_block_to_editor(dto: stratum_app::block::BlockDto) -> BlockDto {
    BlockDto {
        id: dto.id,
        content: dto.content,
        parent_id: dto.parent_id,
        left_id: dto.left_id,
        properties: dto.properties,
        marker: dto.marker,
        priority: dto.priority,
        collapsed: dto.collapsed,
        heading_level: dto.heading_level,
    }
}

/// Convert an editor DTO into the `stratum-app` block DTO (field-identical).
fn editor_block_to_app(dto: BlockDto) -> stratum_app::block::BlockDto {
    stratum_app::block::BlockDto {
        id: dto.id,
        content: dto.content,
        parent_id: dto.parent_id,
        left_id: dto.left_id,
        properties: dto.properties,
        marker: dto.marker,
        priority: dto.priority,
        collapsed: dto.collapsed,
        heading_level: dto.heading_level,
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
mod tests {
    use super::*;

    fn page(path: &str, modified_at: &str, blocks: u64) -> RecentPage {
        RecentPage {
            path: path.to_string(),
            title: path.to_string(),
            block_count: blocks,
            modified_at: modified_at.to_string(),
        }
    }

    #[test]
    fn derive_recents_filters_journals_and_sorts_descending() {
        let pages = vec![
            page("journals/2026-11-09.md", "2026-11-09T09:00:00Z", 5),
            page("pages/A.md", "2026-11-07T12:00:00Z", 7),
            page("pages/B.md", "2026-11-09T08:00:00Z", 12),
            page("pages/C.md", "2026-11-08T18:00:00Z", 34),
        ];
        let recents = derive_recents(pages);
        let paths: Vec<_> = recents.iter().map(|page| page.path.as_str()).collect();
        assert_eq!(paths, ["pages/B.md", "pages/C.md", "pages/A.md"]);
        assert_eq!(recents[0].block_count, 12);
        assert_eq!(recents[1].block_count, 34);
        assert_eq!(recents[2].block_count, 7);
    }

    #[test]
    fn fixture_snapshot_is_deterministic_and_journal_free() {
        let provider = FixtureProvider::new();
        let snapshot = provider.snapshot();
        assert_eq!(
            snapshot.vault,
            Some(VaultStats {
                block_count: 1240,
                page_count: 87
            })
        );
        assert!(!snapshot.recents.is_empty());
        assert!(snapshot
            .recents
            .iter()
            .all(|page| !page.path.starts_with("journals/")));
        assert_eq!(snapshot, provider.refresh());
    }

    #[test]
    fn fixture_export_reports_the_page_count() {
        assert_eq!(FixtureProvider::new().export_html(), Ok(87));
    }

    #[test]
    fn fixture_provider_round_trips_saved_blocks() {
        let provider = FixtureProvider::new();
        let mut blocks = provider
            .load_blocks("pages/Welcome.md")
            .expect("page exists");
        blocks[0].content = "Changed".to_string();
        assert_eq!(
            provider.save_blocks("pages/Welcome.md", blocks.clone()),
            Ok(())
        );
        assert_eq!(provider.load_blocks("pages/Welcome.md"), Some(blocks));
        assert_eq!(provider.load_blocks("pages/Missing.md"), None);
    }
}
