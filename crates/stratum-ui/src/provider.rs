//! Shell data provider.
//!
//! [`ShellData`] is the seam between the gpui shell and the headless service
//! layer. [`AppProvider`] drives the real [`StratumService`] on a Tokio
//! runtime; [`FixtureProvider`] returns a deterministic snapshot for tests
//! and for `STRATUM_FIXTURE=1`, which lets the capture harness render the
//! shell without a vault.
//!
//! Recent-page derivation mirrors `src/stores/recentsStore.ts`: journal pages
//! are excluded and the list is ordered by `modified_at` descending. The
//! ordering is a plain string comparison because the backend already emits
//! RFC 3339 timestamps.

use std::path::PathBuf;
use std::sync::Arc;

use stratum_app::StratumService;

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
}

/// Deterministic provider used by tests and `STRATUM_FIXTURE=1`.
pub struct FixtureProvider {
    snapshot: ShellSnapshot,
    export_pages: u64,
}

impl FixtureProvider {
    /// Create the deterministic fixture snapshot.
    ///
    /// The raw list intentionally contains a journal page so the fixture
    /// exercises the same filtering and ordering as the real provider.
    pub fn new() -> Self {
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
                block_count: 12,
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
        }
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
}
