//! Daily journal creation and sync (ensure_today_journal_core / ensure_today_journal).

use super::{get_file_mtime, resolve_safe_write_path, PageDto};
use crate::commands::vault::AppState;
use std::path::Path;

/// Ensure the journal page for `today` (`YYYY-MM-DD`) is registered in the store
/// and return its `PageDto`. If the file already exists on disk but is not in
/// SQLite (e.g. a stale `blocks.db`), it is synced from disk so the page and blocks
/// become queryable. Idempotent — safe to call repeatedly.
///
/// Extracted from the `ensure_today_journal` command so it can be tested without a
/// Tauri runtime.
pub fn ensure_today_journal_core(
    store: &pkm_block::BlockStore,
    vault_path: &Path,
    today: &str,
) -> Result<PageDto, String> {
    let path = format!("journals/{}.md", today);
    let full_path = resolve_safe_write_path(vault_path, &path)?;
    let content = std::fs::read_to_string(&full_path).map_err(|e| e.to_string())?;
    let (fm, _, _) = pkm_markdown::block_parser::parse_document(&content);
    // Register an on-disk journal into SQLite (page + blocks). This recovers a stale
    // DB where the file exists on disk but is not indexed, and converges idempotently.
    super::sync::sync_page_from_disk(store, &path, vault_path, None)?;
    let blocks = store.get_blocks_by_page(&path).map_err(|e| e.to_string())?;
    Ok(PageDto {
        path,
        slug: today.to_string(),
        title: fm.title.or_else(|| Some(today.to_string())),
        block_count: blocks.len(),
        modified_at: get_file_mtime(&full_path),
    })
}

/// Atomically ensure today's journal page exists.
///
/// Computes the current local date (`YYYY-MM-DD`), checks if `journals/YYYY-MM-DD.md`
/// already exists on disk, and creates it with a frontmatter title if not.
/// Returns the `PageDto` for the journal page in either case.
///
/// This is idempotent — safe to call repeatedly. It replaces the previous pattern of
/// `createPage` + `loadPages` which was prone to race conditions and infinite spinners.
#[tauri::command]
pub async fn ensure_today_journal(state: tauri::State<'_, AppState>) -> Result<PageDto, String> {
    let mut state = state.lock().map_err(|e| e.to_string())?;
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let path = format!("journals/{}.md", today);
    let full_path = resolve_safe_write_path(&state.vault_path, &path)?;

    if full_path.exists() {
        let store = state.get_store().map_err(|e| e.to_string())?;
        let page = ensure_today_journal_core(&store, &state.vault_path, &today)?;
        // Index the synced blocks into Tantivy, then release the directory lock
        // before the file watcher can fire on the just-written page.
        let block_index = state.ensure_block_index()?;
        let blocks = store.get_blocks_by_page(&path).map_err(|e| e.to_string())?;
        for block in &blocks {
            block_index
                .index_block(block, &path)
                .map_err(|e| e.to_string())?;
        }
        block_index.flush().map_err(|e| e.to_string())?;
        drop(state.block_index.take());

        return Ok(page);
    }

    let title = today.clone();
    let content = format!("---\ntitle: {}\n---\n", title);

    if let Some(parent) = full_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&full_path, &content).map_err(|e| e.to_string())?;
    // Mark this as our own save so the file watcher can skip it
    state.watcher_last_save = std::time::SystemTime::now();

    let (_fm, _, blocks) = pkm_markdown::block_parser::parse_document(&content);
    let store = state.get_store().map_err(|e| e.to_string())?;
    let mut page = pkm_block::Page::new(full_path.clone(), &state.vault_path);
    page.frontmatter.title = Some(title.clone());
    page.set_blocks(&blocks);
    for block in &blocks {
        store
            .insert_block(block, &path)
            .map_err(|e| e.to_string())?;
    }
    store.upsert_page(&page).map_err(|e| e.to_string())?;

    let block_index = state.ensure_block_index()?;
    for block in &blocks {
        block_index
            .index_block(block, &path)
            .map_err(|e| e.to_string())?;
    }
    block_index.flush().map_err(|e| e.to_string())?;

    // Release Tantivy directory lock before the file watcher fires.
    drop(state.block_index.take());

    Ok(PageDto {
        path,
        slug: today,
        title: Some(title),
        block_count: blocks.len(),
        modified_at: get_file_mtime(&full_path),
    })
}
