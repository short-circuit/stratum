//! Database repair commands (repair_db_core, repair_db_inner, repair_db_from_disk).

use super::get_file_mtime;
use crate::commands::vault::{AppState, IndexingGuard};
use pkm_core::fs_util::MdCollector;
use pkm_index::block_search::BlockIndex;
use std::path::Path;
use tauri::Emitter;
use tracing::{info, warn};

/// First-class repair for a stale `blocks.db`: reconcile every .md file on disk
/// into SQLite. A page is re-synced from disk when it is missing from the store,
/// registered with zero blocks (a partial-import or stale shape), or its on-disk
/// mtime is strictly newer than its DB record (true drift). After the file pass,
/// DB page rows with no on-disk file are pruned (orphan sweep).
///
/// Extracted from the `repair_db_from_disk` command so it can be tested without a
/// Tauri runtime. This is the pure, DB-scoped contract; progress emission lives in
/// the command wrapper.
pub fn repair_db_core(
    store: &pkm_block::BlockStore,
    vault_path: &Path,
) -> Result<crate::commands::ReindexResult, String> {
    Ok(repair_db_inner(store, vault_path, None)?.0)
}

/// Underlying repair that also returns the number of orphan rows removed, so the
/// command wrapper can report it in its final progress message. `block_index` is
/// forwarded to [`sync_page_from_disk`] so repaired pages also land in the Tantivy
/// search index; pass `None` for a DB-scoped run (tests).
fn repair_db_inner(
    store: &pkm_block::BlockStore,
    vault_path: &Path,
    mut block_index: Option<&mut BlockIndex>,
) -> Result<(crate::commands::ReindexResult, usize), String> {
    let db_paths = store.list_pages().map_err(|e| e.to_string())?;
    let md_files = MdCollector::new()
        .include_extensionless(true)
        .skip_dirs(vec![".pkm", "templates", ".git"])
        .collect_relative(vault_path, vault_path)
        .map_err(|e| e.to_string())?;

    let mut succeeded: usize = 0;
    let mut failed: usize = 0;
    let mut errors: Vec<String> = Vec::new();

    for rel in &md_files {
        let needs_sync = if !db_paths.iter().any(|p| p == rel) {
            true
        } else {
            let blocks = store.get_blocks_by_page(rel).unwrap_or_default();
            if blocks.is_empty() {
                true
            } else {
                let db_modified = store.get_page_modified_at(rel).unwrap_or(None);
                match db_modified {
                    Some(db_str) => {
                        let file_dt = chrono::DateTime::parse_from_rfc3339(&get_file_mtime(
                            &vault_path.join(rel),
                        ));
                        let db_dt = chrono::DateTime::parse_from_rfc3339(&db_str);
                        match (file_dt, db_dt) {
                            (Ok(file_dt), Ok(db_dt)) => file_dt > db_dt,
                            _ => true,
                        }
                    }
                    None => true,
                }
            }
        };

        if needs_sync {
            match super::sync::sync_page_from_disk(
                store,
                rel,
                vault_path,
                block_index.as_deref_mut(),
            ) {
                Ok(true) => succeeded += 1,
                Ok(false) => {}
                Err(e) => {
                    failed += 1;
                    errors.push(format!("{rel}: {e}"));
                    warn!("repair_db_core: failed for {}: {}", rel, e);
                }
            }
        }
    }

    // Orphan sweep: prune DB page rows with no on-disk .md file.
    let mut orphans_removed = 0usize;
    for rel in &db_paths {
        if rel.starts_with(".git/") || rel.contains("/.git/") {
            continue;
        }
        if !md_files.contains(rel) {
            match store.delete_page(rel) {
                Ok(()) => orphans_removed += 1,
                Err(e) => {
                    failed += 1;
                    errors.push(format!("{rel}: {e}"));
                }
            }
        }
    }

    Ok((
        crate::commands::ReindexResult {
            processed: succeeded + failed,
            succeeded,
            failed,
            errors,
        },
        orphans_removed,
    ))
}

/// Run [`repair_db_core`] as a Tauri command, mirroring `reindex_vault`'s
/// indexing-guard / progress-emission pattern and `reindex_page`'s local-search-index
/// pattern: repaired pages are also indexed into Tantivy so full-text search stays
/// consistent with the healed DB.
#[tauri::command]
pub async fn repair_db_from_disk(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<crate::commands::ReindexResult, String> {
    let mut state = state.lock().map_err(|e| e.to_string())?;
    // Drop cached writers to release Tantivy lockfiles before opening a local writer
    drop(state.block_index.take());
    drop(state.index_engine.take());
    let vault_path = state.vault_path.clone();

    // IndexingGuard clears the flag even when an early `?` returns, so an error
    // cannot leave the watcher permanently disabled.
    let _guard = IndexingGuard::new(&state)?;
    let store = state.get_store().map_err(|e| e.to_string())?;
    let mut local_block_index = BlockIndex::create(&state.vault_path.join(".pkm").join("search"))
        .map_err(|e| e.to_string())?;
    let (result, orphans_removed) =
        repair_db_inner(&store, &vault_path, Some(&mut local_block_index))?;
    local_block_index.flush().map_err(|e| e.to_string())?;
    // Drop the local writer so its Tantivy lockfile is released before any later
    // IndexEngine / BlockIndex::create for the same directory.
    drop(local_block_index);

    let _ = app.emit(
        "reindex-progress",
        crate::commands::ProgressEventPayload {
            message: format!(
                "Repaired {}/{} pages ({} failed, {} orphans removed)",
                result.succeeded, result.processed, result.failed, orphans_removed
            ),
            percent: 1.0,
        },
    );
    info!(
        "Repaired {} pages ({} failed, {} orphans removed)",
        result.processed, result.failed, orphans_removed
    );
    // Invalidate graph cache so fresh data is served
    crate::commands::graph::invalidate_graph_cache();
    Ok(result)
}
