//! Filesystem ⇄ SQLite sync helpers for pages.
//!
//! Shared write/sync primitives used by startup import, reindex, journal, and
//! repair paths. `sync_page_from_disk` and `reconcile_page_links` are part of
//! the `commands::page::*` API surface used from `lib.rs` and `commands::block`.

use super::get_file_mtime;
use pkm_core::fs_util::MdCollector;
use pkm_index::block_search::BlockIndex;
use std::path::Path;
use tracing::warn;

/// Read a single .md file from disk, parse it, and sync its page metadata + blocks
/// into SQLite. Returns true if the page was synced, false if the file couldn't be read.
pub(crate) fn sync_page_from_disk(
    store: &pkm_block::BlockStore,
    rel: &str,
    vault_path: &Path,
    block_index: Option<&mut BlockIndex>,
) -> Result<bool, String> {
    let full = vault_path.join(rel);
    let content = std::fs::read_to_string(&full).map_err(|e| e.to_string())?;
    let file_mtime = get_file_mtime(&full);
    let (fm, _, blocks) = pkm_markdown::block_parser::parse_document(&content);

    let mut page = pkm_block::Page::new(full, vault_path);
    page.frontmatter = pkm_block::PageFrontmatter {
        title: fm.title,
        created: fm.created,
        modified: fm.modified,
        tags: fm.tags,
        aliases: fm.aliases,
        ..Default::default()
    };
    page.set_blocks(&blocks);
    // Persist the on-disk modification time so the DB `modified_at` tracks disk;
    // this makes subsequent drift detection meaningful.
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&file_mtime) {
        page.modified_at = dt.with_timezone(&chrono::Utc);
    }

    // Wrap SQLite operations in an explicit transaction for atomicity
    store.execute_batch("BEGIN").map_err(|e| e.to_string())?;
    let result = (|| -> Result<(), String> {
        store.upsert_page(&page).map_err(|e| e.to_string())?;
        store
            .delete_blocks_by_page(rel)
            .map_err(|e| e.to_string())?;
        for block in &blocks {
            store.insert_block(block, rel).map_err(|e| e.to_string())?;
        }
        reconcile_page_links(store, rel, &blocks)?;
        Ok(())
    })();
    match result {
        Ok(()) => store.execute_batch("COMMIT").map_err(|e| e.to_string())?,
        Err(e) => {
            store.execute_batch("ROLLBACK").ok();
            return Err(e);
        }
    }

    // Rebuild Tantivy search index for this page
    if let Some(block_index) = block_index {
        for block in &blocks {
            let _ = block_index.index_block(block, rel);
        }
    }

    Ok(true)
}

/// Handle a watcher Created/Modified/Renamed event for `rel`.
///
/// If the file exists on disk, delegate to [`sync_page_from_disk`] (page +
/// blocks synced, Ok(true)). If the file does NOT exist — the old-path half of
/// an external rename, or an external delete that arrived as Modify — delete
/// the stale SQLite rows (page + blocks; links cascade via FK) and return
/// Ok(false) so the caller can purge the matching Tantivy docs.
///
/// Only reachable from the file watcher, which is not built on Android; the
/// gate mirrors `#[cfg(not(target_os = "android"))]` in `lib.rs` to keep the
/// Android lib build free of dead-code warnings.
#[cfg(not(target_os = "android"))]
pub(crate) fn handle_renamed_or_created(
    store: &pkm_block::BlockStore,
    rel: &str,
    vault_path: &Path,
    block_index: Option<&mut BlockIndex>,
) -> Result<bool, String> {
    if !vault_path.join(rel).exists() {
        // Old-path half of a rename (or a delete surfaced as Modify): remove
        // the stale rows instead of failing on the missing file. `delete_page`
        // internally deletes the page's blocks too; calling
        // `delete_blocks_by_page` first mirrors the watcher Deleted arm.
        store
            .delete_blocks_by_page(rel)
            .map_err(|e| e.to_string())?;
        store.delete_page(rel).map_err(|e| e.to_string())?;
        return Ok(false);
    }
    sync_page_from_disk(store, rel, vault_path, block_index)
}

/// Reconcile the `links` table for a page so block-level backlinks stay convergent
/// with the wiki-links actually present in the page's blocks: delete every prior link
/// row sourced by this page, then insert one row per extracted `[[Target]]` wiki-link.
/// Must be invoked inside the same transaction that rewrites the page's blocks.
///
/// The table drives block-level backlink queries and was previously populated only by
/// tests, leaving production backlinks permanently empty. Wiring this into the shared
/// write/sync paths lets repair, reindex, and startup sync heal the table.
pub fn reconcile_page_links(
    store: &pkm_block::BlockStore,
    page_path: &str,
    blocks: &[pkm_block::Block],
) -> Result<(), String> {
    store
        .delete_links_for_page(page_path)
        .map_err(|e| e.to_string())?;
    for block in blocks {
        let links = pkm_markdown::linker::extract_links(&block.content);
        for link in links {
            // Asset targets (`assets/...`) become `asset_ref` rows so the graph
            // (which filters on `link_type = 'page_ref'`) never treats them as notes,
            // while everything else stays a `page_ref` wiki-link.
            let link_type = if pkm_markdown::linker::is_asset_link_target(&link.target) {
                "asset_ref"
            } else {
                "page_ref"
            };
            store
                .insert_link(block.id, link_type, Some(&link.target), None)
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// Scan the vault filesystem for .md files and upsert any missing or empty ones into SQLite.
/// Called once at app startup to import pre-existing pages.
/// Pages already in SQLite with zero blocks are also re-synced from disk to recover from
/// partial imports (e.g. the initial bug where blocks weren't parsed).
pub fn sync_filesystem_to_db(vault_path: &Path, db_path: &Path) -> Result<usize, String> {
    let store = pkm_block::BlockStore::open(db_path).map_err(|e| e.to_string())?;
    let db_paths = store.list_pages().map_err(|e| e.to_string())?;
    let md_files = MdCollector::new()
        .include_extensionless(true)
        .skip_dirs(vec![".pkm", "templates", ".git"])
        .collect_relative(vault_path, vault_path)
        .map_err(|e| e.to_string())?;

    // Create a shared BlockIndex for the full sync pass
    let mut block_index = BlockIndex::create(&vault_path.join(".pkm").join("search")).ok();

    let mut count = 0;
    for rel in md_files {
        let needs_sync = if db_paths.iter().any(|p| p == &rel) {
            let blocks = store.get_blocks_by_page(&rel).unwrap_or_default();
            blocks.is_empty()
        } else {
            true
        };

        if needs_sync {
            match sync_page_from_disk(&store, &rel, vault_path, block_index.as_mut()) {
                Ok(true) => count += 1,
                Ok(false) => {}
                Err(e) => {
                    warn!("sync_filesystem_to_db: failed for {}: {}", rel, e);
                }
            }
        }
    }

    // Prune stale ("ghost") rows: any DB path whose on-disk file no longer
    // exists (external delete / rename old-path) plus legacy `.git/` rows.
    // Direct existence checks are intentional — rows for `templates/*.md`
    // (registered via watcher events but skipped by MdCollector) must survive,
    // and only truly-missing files are removed.
    for rel in &db_paths {
        let is_git = rel.starts_with(".git/") || rel.contains("/.git/");
        if is_git || !vault_path.join(rel).exists() {
            let _ = store.delete_page(rel);
            // Purge matching Tantivy docs so search never returns deleted
            // pages. Applied through the shared BlockIndex; committed by the
            // flush below.
            if let Some(ref mut bi) = block_index {
                let _ = bi.delete_blocks_by_page(rel);
            }
            count += 1;
        }
    }

    // Flush index once after all pages (+ prunes) are processed so pending
    // Tantivy deletions are committed.
    if let Some(ref mut bi) = block_index {
        let _ = bi.flush();
    }

    Ok(count)
}

/// Re-read a single .md file from disk and re-parse it, preserving block syntax,
/// UUIDs, markers, priorities, and heading levels. This is the "Reindex Note" operation.
/// Delegates to [`sync_page_from_disk`] since the two are identical.
pub(super) fn reparse_page_from_disk(
    store: &pkm_block::BlockStore,
    rel: &str,
    vault_path: &Path,
    block_index: Option<&mut BlockIndex>,
) -> Result<bool, String> {
    sync_page_from_disk(store, rel, vault_path, block_index)
}

#[cfg(test)]
mod tests {
    //! Unit tests for the stale-ghost-page sync fix: `handle_renamed_or_created`
    //! deletes stale rows when the file is gone, and `sync_filesystem_to_db`
    //! prunes ghost rows (any DB path with no on-disk file) while keeping real
    //! pages — including `templates/*.md` rows that MdCollector skips.

    use super::*;
    use pkm_block::Page;
    use uuid::Uuid;

    fn test_vault() -> (tempfile::TempDir, std::path::PathBuf, pkm_block::BlockStore) {
        let dir = tempfile::TempDir::new().unwrap();
        let vault = dir.path().to_path_buf();
        std::fs::create_dir_all(vault.join(".pkm")).unwrap();
        std::fs::create_dir_all(vault.join("pages")).unwrap();
        let db = vault.join(".pkm").join("blocks.db");
        let store = pkm_block::BlockStore::open(&db).unwrap();
        (dir, vault, store)
    }

    /// A page row whose on-disk file is gone must be removed (page + blocks)
    /// rather than leaving a ghost behind.
    #[test]
    fn handle_renamed_or_created_removes_stale_rows_when_file_gone() {
        let (_dir, vault, store) = test_vault();
        let rel = "pages/ghost.md";

        // Upsert a page + one block for a path that has NO file on disk.
        let page = Page::new(vault.join(rel), &vault);
        store.upsert_page(&page).unwrap();
        let block = pkm_block::Block::new(Uuid::new_v4(), "orphan block".to_string());
        store.insert_block(&block, rel).unwrap();

        let result = handle_renamed_or_created(&store, rel, &vault, None);
        assert_eq!(result, Ok(false), "missing file must report Ok(false)");
        assert!(
            store.get_page(rel).unwrap().is_none(),
            "page row must be deleted"
        );
        assert!(
            store.get_blocks_by_page(rel).unwrap().is_empty(),
            "blocks for the page must be deleted"
        );
    }

    /// When the file does exist, the helper must behave like a normal sync and
    /// persist the page + parsed blocks.
    #[test]
    fn handle_renamed_or_created_syncs_when_file_exists() {
        let (_dir, vault, store) = test_vault();
        let rel = "pages/real.md";
        std::fs::write(vault.join(rel), "- on-disk hello\n").unwrap();

        let result = handle_renamed_or_created(&store, rel, &vault, None);
        assert_eq!(result, Ok(true), "existing file must sync successfully");
        assert!(
            store.get_page(rel).unwrap().is_some(),
            "page row must exist"
        );
        let blocks = store.get_blocks_by_page(rel).unwrap();
        assert_eq!(blocks.len(), 1, "exactly one block expected");
        assert!(
            blocks[0].content.contains("on-disk hello"),
            "block content must come from the on-disk file"
        );
    }

    /// `sync_filesystem_to_db` must prune ghost rows (DB path with no file) while
    /// preserving real pages, counting each pruned row in the return value.
    #[test]
    fn sync_filesystem_to_db_prunes_ghost_rows_keeps_real_rows() {
        let (_dir, vault, store) = test_vault();
        std::fs::write(vault.join("pages/real.md"), "- stable block\n").unwrap();

        // Real page with non-empty blocks (main sync loop will skip it).
        let real_rel = "pages/real.md";
        let page = Page::new(vault.join(real_rel), &vault);
        store.upsert_page(&page).unwrap();
        let block = pkm_block::Block::new(Uuid::new_v4(), "stable block".to_string());
        store.insert_block(&block, real_rel).unwrap();

        // Ghost page with NO blocks and NO file on disk.
        let ghost_rel = "pages/ghost.md";
        let ghost = Page::new(vault.join(ghost_rel), &vault);
        store.upsert_page(&ghost).unwrap();

        let db = vault.join(".pkm").join("blocks.db");
        let count = sync_filesystem_to_db(&vault, &db).unwrap();
        assert!(count >= 1, "at least the ghost row must be pruned");

        assert!(
            store.get_page(ghost_rel).unwrap().is_none(),
            "ghost page row must be pruned"
        );
        assert!(
            store.get_blocks_by_page(ghost_rel).unwrap().is_empty(),
            "ghost page blocks must be pruned/empty"
        );
        assert!(
            store.get_page(real_rel).unwrap().is_some(),
            "real page row must survive"
        );
        assert_eq!(
            store.get_blocks_by_page(real_rel).unwrap().len(),
            1,
            "real page blocks must survive"
        );
    }

    /// Manual-smoke equivalent for an external rename: after moving a page on
    /// disk (`mv pages/a.md pages/b.md`), a subsequent `sync_filesystem_to_db`
    /// must remove the old path's row and register the new one, so `list_pages`
    /// never shows the stale (ghost) path again.
    #[test]
    fn sync_filesystem_to_db_after_external_rename_removes_old_path() {
        let (_dir, vault, store) = test_vault();
        std::fs::write(vault.join("pages/a.md"), "- original content\n").unwrap();

        let db = vault.join(".pkm").join("blocks.db");
        // First boot: import the page.
        sync_filesystem_to_db(&vault, &db).unwrap();
        assert!(store.get_page("pages/a.md").unwrap().is_some());

        // External rename: old file gone, new file present.
        std::fs::rename(vault.join("pages/a.md"), vault.join("pages/b.md")).unwrap();

        // Second boot: the stale row must be pruned, new path registered.
        sync_filesystem_to_db(&vault, &db).unwrap();
        let pages = store.list_pages().unwrap();
        assert!(
            !pages.iter().any(|p| p == "pages/a.md"),
            "old (renamed-away) path must not remain a ghost row; got {pages:?}"
        );
        assert!(
            pages.contains(&"pages/b.md".to_string()),
            "new path must be registered after rename; got {pages:?}"
        );
    }
}
