//! Page management commands.
//!
//! Submodule re-exports keep the full command API reachable as
//! `commands::page::<name>` (lib.rs registrations, cross-module callers).

mod journal;
mod normalize;
mod reindex;
mod repair;
mod sync;

pub use journal::*;
pub use normalize::*;
pub use reindex::*;
pub use repair::*;
pub use sync::*;

use crate::commands::vault::AppState;
use serde::{Deserialize, Serialize};

/// Resolve a user-provided path safely within the vault for read operations.
/// Canonicalizes both the vault path and the resulting path to ensure
/// the result is contained within the vault. The file must exist.
pub(crate) fn resolve_safe_path(
    vault_path: &std::path::Path,
    user_path: &str,
) -> Result<std::path::PathBuf, String> {
    let canonical_vault = vault_path
        .canonicalize()
        .map_err(|e| format!("Invalid vault path: {e}"))?;
    let full = canonical_vault.join(user_path);
    let canonical_full = full
        .canonicalize()
        .map_err(|_| format!("Path does not exist: {user_path}"))?;
    if canonical_full.starts_with(&canonical_vault) {
        Ok(canonical_full)
    } else {
        Err("Path traversal detected".to_string())
    }
}

/// Resolve a user-provided path safely within the vault for write/create operations
/// where the file may not yet exist. Validates that every existing ancestor
/// directory is within the vault. Walks up the path tree until it finds an
/// existing path component (at minimum the vault_path itself).
pub(crate) fn resolve_safe_write_path(
    vault_path: &std::path::Path,
    user_path: &str,
) -> Result<std::path::PathBuf, String> {
    let canonical_vault = vault_path
        .canonicalize()
        .map_err(|e| format!("Invalid vault path: {e}"))?;
    let full = canonical_vault.join(user_path);

    // Walk up from the full path to find an existing ancestor directory
    // to verify it's within the vault. At minimum vault_path exists.
    let mut check_path = full.as_path();
    loop {
        if check_path.exists() {
            let canonical = check_path
                .canonicalize()
                .map_err(|e| format!("Path resolution failed: {e}"))?;
            if !canonical.starts_with(&canonical_vault) {
                return Err("Path traversal detected".to_string());
            }
            break;
        }
        match check_path.parent() {
            Some(parent) => check_path = parent,
            None => return Err("Path is outside vault".to_string()),
        }
    }

    Ok(full)
}

/// Read a file's modification time from filesystem metadata and return it as an
/// RFC 3339 string. Falls back to `Utc::now()` if the file doesn't exist or
/// metadata can't be read (e.g. the file was just created and hasn't been
/// flushed to disk yet).
fn get_file_mtime(full_path: &std::path::Path) -> String {
    std::fs::metadata(full_path)
        .and_then(|m| m.modified())
        .map(|t| {
            let dt: chrono::DateTime<chrono::Utc> = t.into();
            dt.to_rfc3339()
        })
        .unwrap_or_else(|_| chrono::Utc::now().to_rfc3339())
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PageDto {
    pub path: String,
    pub slug: String,
    pub title: Option<String>,
    pub block_count: usize,
    pub modified_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PageListDto {
    pub pages: Vec<PageDto>,
}

#[tauri::command]
pub async fn list_pages(state: tauri::State<'_, AppState>) -> Result<PageListDto, String> {
    let vault_path = {
        let s = state.lock().map_err(|e| e.to_string())?;
        s.vault_path.clone()
    };
    let db_path = {
        let s = state.lock().map_err(|e| e.to_string())?;
        s.db_path.clone()
    };

    // Run DB query on blocking pool to avoid freezing the async event loop
    let paths = tokio::task::spawn_blocking(move || {
        let store = pkm_block::BlockStore::open(&db_path)?;
        store.list_pages()
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    let mut pages = Vec::new();
    for path in paths {
        if path.starts_with(".git/") || path.contains("/.git/") {
            continue;
        }
        let slug = std::path::Path::new(&path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("untitled")
            .to_string();
        let full_path = vault_path.join(&path);
        pages.push(PageDto {
            path,
            slug,
            title: None,
            block_count: 0,
            modified_at: get_file_mtime(&full_path),
        });
    }

    Ok(PageListDto { pages })
}

#[tauri::command]
pub async fn open_page(path: String, state: tauri::State<'_, AppState>) -> Result<PageDto, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let full_path = resolve_safe_path(&state.vault_path, &path)?;

    let slug = std::path::Path::new(&path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("untitled")
        .to_string();

    // If the file doesn't exist, return an empty page (new page)
    let (frontmatter, block_count) = if full_path.exists() {
        let content = std::fs::read_to_string(&full_path).map_err(|e| e.to_string())?;
        let (fm, _, _) = pkm_markdown::block_parser::parse_document(&content);
        let store = state.get_store().map_err(|e| e.to_string())?;
        let blocks = store.get_blocks_by_page(&path).map_err(|e| e.to_string())?;
        (fm, blocks.len())
    } else {
        (pkm_core::Frontmatter::default(), 0)
    };

    Ok(PageDto {
        path: path.clone(),
        slug,
        title: frontmatter.title,
        block_count,
        modified_at: get_file_mtime(&full_path),
    })
}

#[tauri::command]
pub async fn save_page(
    path: String,
    content: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let mut state = state.lock().map_err(|e| e.to_string())?;
    let full_path = resolve_safe_write_path(&state.vault_path, &path)?;
    let vault_path = state.vault_path.clone();

    // Ensure parent directory exists
    if let Some(parent) = full_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    // Parse content
    let (frontmatter, _, blocks) = pkm_markdown::block_parser::parse_document(&content);
    let store = state.get_store().map_err(|e| e.to_string())?;

    // Build page metadata
    let mut page = pkm_block::Page::new(full_path.clone(), &vault_path);
    page.frontmatter = pkm_block::PageFrontmatter {
        title: frontmatter.title,
        created: frontmatter.created,
        modified: frontmatter.modified,
        tags: frontmatter.tags,
        aliases: frontmatter.aliases,
        ..Default::default()
    };
    page.set_blocks(&blocks);

    // SQLite first (inside a transaction), then write .md file
    // This ensures the database is the source of truth.
    store.execute_batch("BEGIN").map_err(|e| e.to_string())?;
    let result = (|| -> Result<(), String> {
        store
            .delete_blocks_by_page(&path)
            .map_err(|e| e.to_string())?;
        for block in &blocks {
            store
                .insert_block(block, &path)
                .map_err(|e| e.to_string())?;
        }
        store.upsert_page(&page).map_err(|e| e.to_string())?;
        Ok(())
    })();
    match result {
        Ok(()) => store.execute_batch("COMMIT").map_err(|e| e.to_string())?,
        Err(e) => {
            store.execute_batch("ROLLBACK").ok();
            return Err(e);
        }
    }

    // Write .md file after SQLite succeeds
    std::fs::write(&full_path, &content).map_err(|e| e.to_string())?;
    // Mark this as our own save so the file watcher can skip it
    state.watcher_last_save = std::time::SystemTime::now();

    // Notify auto-commit engine
    state.record_change(&path);

    // Drop BlockIndex writer before IndexEngine acquires its own (same Tantivy dir)
    drop(state.block_index.take());

    // Keep IndexEngine in sync with the written file
    state
        .ensure_index()?
        .refresh_page(&path, &vault_path)
        .map_err(|e| format!("Index refresh failed: {}", e))?;

    // Invalidate graph cache since page data changed
    crate::commands::graph::invalidate_graph_cache();

    Ok(())
}

#[tauri::command]
pub async fn create_page(
    path: String,
    title: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<PageDto, String> {
    let mut state = state.lock().map_err(|e| e.to_string())?;

    // Ensure path has .md extension
    let path = if path.ends_with(".md") {
        path
    } else {
        format!("{}.md", path)
    };

    let full_path = resolve_safe_write_path(&state.vault_path, &path)?;

    if full_path.exists() {
        return Err(format!("Page already exists: {}", path));
    }

    // Create with default frontmatter
    let content = if let Some(ref t) = title {
        format!("---\ntitle: {}\n---\n", t)
    } else {
        String::new()
    };

    if let Some(parent) = full_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&full_path, &content).map_err(|e| e.to_string())?;
    // Mark this as our own save so the file watcher can skip it
    state.watcher_last_save = std::time::SystemTime::now();

    // Notify auto-commit engine
    state.record_change(&path);

    // Parse content into blocks and upsert in SQLite so the page appears in list_pages
    let (_fm, _, blocks) = pkm_markdown::block_parser::parse_document(&content);
    let store = state.get_store().map_err(|e| e.to_string())?;
    let mut page = pkm_block::Page::new(full_path.clone(), &state.vault_path);
    page.frontmatter.title = title.clone();
    page.set_blocks(&blocks);
    for block in &blocks {
        store
            .insert_block(block, &path)
            .map_err(|e| e.to_string())?;
    }
    store.upsert_page(&page).map_err(|e| e.to_string())?;

    // Index blocks in Tantivy for full-text search
    let block_index = state.ensure_block_index()?;
    for block in &blocks {
        block_index
            .index_block(block, &path)
            .map_err(|e| e.to_string())?;
    }
    block_index.flush().map_err(|e| e.to_string())?;

    // Drop BlockIndex writer to release the Tantivy directory lock before
    // the file watcher fires on the new .md file — otherwise the watcher's
    // own BlockIndex::create hits a lock contention error.
    drop(state.block_index.take());

    Ok(PageDto {
        path: path.clone(),
        slug: std::path::Path::new(&path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("untitled")
            .to_string(),
        title,
        block_count: blocks.len(),
        modified_at: get_file_mtime(&full_path),
    })
}

#[tauri::command]
pub async fn delete_page(path: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut state = state.lock().map_err(|e| e.to_string())?;
    let full_path = resolve_safe_write_path(&state.vault_path, &path)?;

    if full_path.exists() {
        std::fs::remove_file(&full_path).map_err(|e| e.to_string())?;
    }

    // Notify auto-commit engine
    state.record_change(&path);

    let store = state.get_store().map_err(|e| e.to_string())?;
    store.delete_page(&path).map_err(|e| e.to_string())?;

    // Drop BlockIndex writer before IndexEngine acquires its own (same Tantivy dir)
    drop(state.block_index.take());

    // Remove from IndexEngine so graph/search don't reference a deleted note
    state
        .ensure_index()?
        .remove_note(&path)
        .map_err(|e| format!("Failed to remove note from index: {}", e))?;

    // Invalidate graph cache since a page was deleted
    crate::commands::graph::invalidate_graph_cache();

    Ok(())
}
