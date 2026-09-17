//! Vault / page reindexing commands (reindex_vault, reindex_page).
//!
//! Both re-index pages from disk into SQLite (and Tantivy) and follow the
//! indexing-guard / progress-emission pattern established across commands.

use crate::commands::vault::{AppState, IndexingGuard};
use pkm_core::ProgressCallback;
use pkm_index::block_search::BlockIndex;
use tauri::Emitter;
use tracing::{info, warn};

/// Re-sync every .md file from disk into SQLite. Useful after importing a new dataset
/// or recovering from a corrupted/inconsistent blocks.db.
#[tauri::command]
pub async fn reindex_vault(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<crate::commands::ReindexResult, String> {
    let mut state = state.lock().map_err(|e| e.to_string())?;
    // Drop cached writers to release Tantivy lockfiles
    drop(state.block_index.take());
    drop(state.index_engine.take());
    let vault_path = state.vault_path.clone();

    // Signal a bulk operation — the file watcher skips events while the indexing
    // flag is held. IndexingGuard clears the flag even when an early `?` returns,
    // so an error can never leak the flag and permanently disable the watcher.
    let _guard = IndexingGuard::new(&state)?;

    // Single pass: rebuild_all handles Tantivy indexing + graph + tags + notes
    let app2 = app.clone();
    let progress_cb: Option<ProgressCallback> = Some(Box::new(move |msg: String, pct: f32| {
        let _ = app2.emit(
            "reindex-progress",
            crate::commands::ProgressEventPayload {
                message: msg,
                percent: pct,
            },
        );
    }));

    let notes = state
        .ensure_index()
        .map_err(|e| e.to_string())?
        .rebuild_all(progress_cb)
        .map_err(|e| e.to_string())?;

    // Sync into SQLite (no Tantivy re-indexing)
    let store = state.get_store().map_err(|e| e.to_string())?;
    let mut succeeded: u64 = 0;
    let mut failed: u64 = 0;
    for note in &notes {
        let rel = note.rel_path.to_string_lossy().to_string();
        let full = vault_path.join(&note.rel_path);
        let content = match std::fs::read_to_string(&full) {
            Ok(c) => c,
            Err(_) => {
                failed += 1;
                continue;
            }
        };
        let (_fm, _body, blocks) = pkm_markdown::block_parser::parse_document(&content);
        let mut page = pkm_block::Page::new(full, &vault_path);
        page.frontmatter = pkm_block::PageFrontmatter {
            title: note.frontmatter.title.clone(),
            created: note.frontmatter.created.clone(),
            modified: note.frontmatter.modified.clone(),
            tags: note.tags.iter().map(|t| t.name.clone()).collect(),
            aliases: note.frontmatter.aliases.clone(),
            ..Default::default()
        };
        page.set_blocks(&blocks);

        store.execute_batch("BEGIN").ok();
        let result = (|| -> Result<(), String> {
            store.upsert_page(&page).map_err(|e| e.to_string())?;
            store
                .delete_blocks_by_page(&rel)
                .map_err(|e| e.to_string())?;
            for block in &blocks {
                store.insert_block(block, &rel).map_err(|e| e.to_string())?;
            }
            super::sync::reconcile_page_links(&store, &rel, &blocks)?;
            Ok(())
        })();
        match result {
            Ok(()) => {
                store.execute_batch("COMMIT").ok();
                succeeded += 1;
            }
            Err(_e) => {
                store.execute_batch("ROLLBACK").ok();
                failed += 1;
            }
        }
    }

    let processed = succeeded + failed;
    let _ = app.emit(
        "reindex-progress",
        crate::commands::ProgressEventPayload {
            message: format!(
                "Reindexed {}/{} pages ({} failed)",
                succeeded, processed, failed
            ),
            percent: 1.0,
        },
    );
    info!("Reindexed {} pages ({} failed)", processed, failed);
    // Invalidate graph cache so fresh data is served
    crate::commands::graph::invalidate_graph_cache();
    Ok(crate::commands::ReindexResult {
        processed: processed as usize,
        succeeded: succeeded as usize,
        failed: failed as usize,
        errors: vec![],
    })
}

/// Re-sync a single page from disk into SQLite, always using the plain-text converter.
/// Useful for reindexing externally-created or previously-saved notes.
#[tauri::command]
pub async fn reindex_page(
    path: String,
    state: tauri::State<'_, AppState>,
) -> Result<crate::commands::ReindexResult, String> {
    let mut state = state.lock().map_err(|e| e.to_string())?;
    // Drop cached writers to release Tantivy lockfiles before creating a local one
    drop(state.block_index.take());
    drop(state.index_engine.take());
    let _guard = IndexingGuard::new(&state)?;
    let store = state.get_store().map_err(|e| e.to_string())?;
    let mut local_block_index = BlockIndex::create(&state.vault_path.join(".pkm").join("search"))
        .map_err(|e| e.to_string())?;

    let mut succeeded = 0usize;
    let mut errors = Vec::new();

    match super::sync::reparse_page_from_disk(
        &store,
        &path,
        &state.vault_path,
        Some(&mut local_block_index),
    ) {
        Ok(true) => {
            local_block_index.flush().map_err(|e| e.to_string())?;
            succeeded = 1;
        }
        Ok(false) => {}
        Err(e) => {
            errors.push(format!("{}: {}", path, e));
            warn!("reindex_page: failed for {}: {}", path, e);
        }
    }

    // Drop local index writer so its Tantivy lockfile is released before
    // IndexEngine tries to open its own writer on the same directory.
    drop(local_block_index);

    // Drop indexing guard before accessing IndexEngine (&mut self)
    drop(_guard);

    // Rebuild IndexEngine so graph data stays current
    state
        .ensure_index()?
        .rebuild_all(None)
        .map_err(|e| e.to_string())?;

    let processed = if succeeded > 0 || !errors.is_empty() {
        1
    } else {
        0
    };
    Ok(crate::commands::ReindexResult {
        processed,
        succeeded,
        failed: errors.len(),
        errors,
    })
}
