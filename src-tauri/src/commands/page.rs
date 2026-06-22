//! Page management commands.

use crate::commands::vault::AppState;
use serde::{Deserialize, Serialize};
use std::path::Path;

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
    let state = state.lock().map_err(|e| e.to_string())?;
    let store = pkm_block::BlockStore::open(&state.db_path).map_err(|e| e.to_string())?;
    let paths = store.list_pages().map_err(|e| e.to_string())?;

    let mut pages = Vec::new();
    for path in paths {
        let slug = std::path::Path::new(&path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("untitled")
            .to_string();
        let fm = store.get_page(&path).ok().flatten();
        let title = fm.and_then(|f| f.title);
        let blocks = store.get_blocks_by_page(&path).unwrap_or_default();

        pages.push(PageDto {
            path,
            slug,
            title,
            block_count: blocks.len(),
            modified_at: chrono::Utc::now().to_rfc3339(),
        });
    }

    Ok(PageListDto { pages })
}

/// Scan the vault filesystem for .md files and upsert any missing ones into SQLite.
/// Called once at app startup to import pre-existing pages.
pub fn sync_filesystem_to_db(vault_path: &Path, db_path: &Path) -> Result<usize, String> {
    let store = pkm_block::BlockStore::open(db_path).map_err(|e| e.to_string())?;
    let db_paths = store.list_pages().map_err(|e| e.to_string())?;
    let md_files = find_md_files(vault_path, vault_path).map_err(|e| e.to_string())?;

    let mut count = 0;
    for rel in md_files {
        if !db_paths.contains(&rel) {
            let full = vault_path.join(&rel);
            let page = pkm_block::Page::new(full, vault_path);
            store.upsert_page(&page).map_err(|e| e.to_string())?;
            count += 1;
        }
    }
    Ok(count)
}

#[tauri::command]
pub async fn open_page(
    path: String,
    state: tauri::State<'_, AppState>,
) -> Result<PageDto, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let full_path = state.vault_path.join(&path);

    let slug = std::path::Path::new(&path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("untitled")
        .to_string();

    // If the file doesn't exist, return an empty page (new page)
    let (frontmatter, block_count) = if full_path.exists() {
        let content = std::fs::read_to_string(&full_path).map_err(|e| e.to_string())?;
        let (fm, _, _) = pkm_markdown::block_parser::parse_document(&content);
        let store = pkm_block::BlockStore::open(&state.db_path).map_err(|e| e.to_string())?;
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
        modified_at: chrono::Utc::now().to_rfc3339(),
    })
}

#[tauri::command]
pub async fn save_page(
    path: String,
    content: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let full_path = state.vault_path.join(&path);

    // Ensure parent directory exists
    if let Some(parent) = full_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    // Write .md file
    std::fs::write(&full_path, &content).map_err(|e| e.to_string())?;

    // Parse and store blocks in SQLite
    let (frontmatter, _, blocks) = pkm_markdown::block_parser::parse_document(&content);
    let store = pkm_block::BlockStore::open(&state.db_path).map_err(|e| e.to_string())?;

    // Delete existing blocks for this page
    store.delete_blocks_by_page(&path).map_err(|e| e.to_string())?;

    // Insert blocks
    for block in &blocks {
        store.insert_block(block, &path).map_err(|e| e.to_string())?;
    }

    // Upsert page metadata
    let mut page = pkm_block::Page::new(full_path.clone(), &state.vault_path);
    page.frontmatter = pkm_block::PageFrontmatter {
        title: frontmatter.title,
        created: frontmatter.created,
        modified: frontmatter.modified,
        tags: frontmatter.tags,
        aliases: frontmatter.aliases,
        ..Default::default()
    };
    store.upsert_page(&page).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn create_page(
    path: String,
    title: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<PageDto, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let full_path = state.vault_path.join(&path);

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

    // Also upsert in SQLite so the page appears in list_pages
    let store = pkm_block::BlockStore::open(&state.db_path).map_err(|e| e.to_string())?;
    let mut page = pkm_block::Page::new(full_path, &state.vault_path);
    page.frontmatter.title = title.clone();
    store.upsert_page(&page).map_err(|e| e.to_string())?;

    Ok(PageDto {
        path: path.clone(),
        slug: std::path::Path::new(&path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("untitled")
            .to_string(),
        title,
        block_count: 0,
        modified_at: chrono::Utc::now().to_rfc3339(),
    })
}

#[tauri::command]
pub async fn delete_page(
    path: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let full_path = state.vault_path.join(&path);

    if full_path.exists() {
        std::fs::remove_file(&full_path).map_err(|e| e.to_string())?;
    }

    let store = pkm_block::BlockStore::open(&state.db_path).map_err(|e| e.to_string())?;
    store.delete_page(&path).map_err(|e| e.to_string())?;

    Ok(())
}

/// Walk a directory recursively, returning vault-relative paths of .md files.
fn find_md_files(dir: &Path, vault_root: &Path) -> std::io::Result<Vec<String>> {
    let mut result = Vec::new();
    if !dir.exists() {
        return Ok(result);
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.file_name().and_then(|n| n.to_str()) == Some(".pkm")
            || path.file_name().and_then(|n| n.to_str()) == Some("templates")
        {
            continue;
        }
        if path.is_dir() {
            result.extend(find_md_files(&path, vault_root)?);
        } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
            let rel = path
                .strip_prefix(vault_root)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();
            result.push(rel);
        }
    }
    Ok(result)
}
