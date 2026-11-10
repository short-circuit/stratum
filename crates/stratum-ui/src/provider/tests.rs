//! Provider tests: recent-page derivation and the fixture/default asset flows.
//!
//! Kept in a child module so `provider/mod.rs` stays under the repository
//! file-length limit. The tests exercise [`super::derive_recents`], the
//! deterministic fixture provider and the window-free [`super::ShellData`]
//! defaults.

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

/// Provider with no persistence, used to exercise the trait defaults.
struct BareProvider;

impl ShellData for BareProvider {
    fn snapshot(&self) -> ShellSnapshot {
        ShellSnapshot {
            vault: None,
            recents: Vec::new(),
        }
    }

    fn refresh(&self) -> ShellSnapshot {
        self.snapshot()
    }

    fn export_html(&self) -> Result<u64, String> {
        Ok(0)
    }
}

#[test]
fn default_asset_methods_keep_window_free_providers_compiling() {
    let provider = BareProvider;
    assert_eq!(provider.read_asset_thumbnail("assets/photo.png"), None);
    assert!(provider.page_attachments("pages/Any.md").is_empty());
    assert_eq!(provider.delete_asset("assets/photo.png"), Ok(()));

    let path_row = provider
        .import_asset_path("photos/a.png")
        .expect("default path row");
    assert_eq!(path_row.ref_path, "photos/a.png");
    assert_eq!(path_row.mime, "image/png");
    assert_eq!(path_row.size, 0);
    assert_eq!(path_row.kind, stratum_editor::AssetKind::Image);

    let bytes_row = provider
        .import_asset_bytes(vec![1, 2, 3], "paste.jpg", None)
        .expect("default bytes row");
    assert_eq!(bytes_row.ref_path, "assets/paste.jpg");
    assert_eq!(bytes_row.mime, "image/jpeg");
    assert_eq!(bytes_row.size, 3);
    assert_eq!(bytes_row.size_label, "3 B");

    let typed = provider
        .import_asset_bytes(vec![0; 4], "blob", Some("image/png"))
        .expect("default bytes row");
    assert_eq!(typed.mime, "image/png");
}
