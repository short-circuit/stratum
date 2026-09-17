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

/// Reconcile the `links` table for a page so block-level backlinks stay convergent
/// with the wiki-links actually present in the page's blocks: delete every prior link
/// row sourced by this page, then insert one row per extracted `[[Target]]` wiki-link.
/// Must be invoked inside the same transaction that rewrites the page's blocks.
///
/// The table drives block-level backlink queries and was previously populated only by
/// tests, leaving production backlinks permanently empty. Wiring this into the shared
/// write/sync paths lets repair, reindex, and startup sync heal the table.
pub(crate) fn reconcile_page_links(
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
            store
                .insert_link(block.id, "page_ref", Some(&link.target), None)
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

    // Flush index once after all pages are processed
    if let Some(ref mut bi) = block_index {
        let _ = bi.flush();
    }

    for rel in &db_paths {
        if rel.starts_with(".git/") || rel.contains("/.git/") {
            let _ = store.delete_page(rel);
            count += 1;
        }
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
