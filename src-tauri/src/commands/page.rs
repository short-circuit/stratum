//! Page management commands.

use crate::commands::vault::{AppState, IndexingGuard};
use pkm_core::fs_util::MdCollector;
use pkm_index::block_search::BlockIndex;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tauri::Emitter;
use tracing::info;

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
        if path.starts_with(".git/") || path.contains("/.git/") {
            continue;
        }
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

/// Read a single .md file from disk, parse it, and sync its page metadata + blocks
/// into SQLite. Returns true if the page was synced, false if the file couldn't be read.
fn sync_page_from_disk(
    store: &pkm_block::BlockStore,
    rel: &str,
    vault_path: &Path,
    block_index: Option<&mut BlockIndex>,
) -> Result<bool, String> {
    let full = vault_path.join(rel);
    let content = std::fs::read_to_string(&full).map_err(|e| e.to_string())?;
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
    store.upsert_page(&page).map_err(|e| e.to_string())?;

    store
        .delete_blocks_by_page(rel)
        .map_err(|e| e.to_string())?;
    for block in &blocks {
        store.insert_block(block, rel).map_err(|e| e.to_string())?;
    }

    // Rebuild Tantivy search index for this page
    if let Some(block_index) = block_index {
        for block in &blocks {
            let _ = block_index.index_block(block, rel);
        }
    }

    Ok(true)
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

        if needs_sync && sync_page_from_disk(&store, &rel, vault_path, block_index.as_mut())? {
            count += 1;
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

/// Re-sync every .md file from disk into SQLite. Useful after importing a new dataset
/// or recovering from a corrupted/inconsistent blocks.db.
#[tauri::command]
pub async fn reindex_vault(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<usize, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let _guard = IndexingGuard::new(&state)?;
    let store = pkm_block::BlockStore::open(&state.db_path).map_err(|e| e.to_string())?;
    let md_files = MdCollector::new()
        .include_extensionless(true)
        .skip_dirs(vec![".pkm", "templates", ".git"])
        .collect_relative(&state.vault_path, &state.vault_path)
        .map_err(|e| e.to_string())?;

    let vault_path = state.vault_path.clone();
    let total = md_files.len();
    // Create a local BlockIndex for this reindex pass (IndexingGuard prevents state mutation)
    let mut local_block_index =
        BlockIndex::create(&vault_path.join(".pkm").join("search")).map_err(|e| e.to_string())?;
    let mut count = 0;
    for (i, rel) in md_files.iter().enumerate() {
        let _ = app.emit(
            "reindex-progress",
            super::ProgressEventPayload {
                message: format!("Reindexing {}/{}", i + 1, total),
                percent: if total > 0 {
                    (i as f32 + 1.0) / total as f32
                } else {
                    1.0
                },
            },
        );
        if sync_page_from_disk(&store, rel, &vault_path, Some(&mut local_block_index))? {
            count += 1;
        }
    }
    local_block_index.flush().map_err(|e| e.to_string())?;

    let _ = app.emit(
        "reindex-progress",
        super::ProgressEventPayload {
            message: format!("Reindexed {} pages from filesystem", count),
            percent: 1.0,
        },
    );
    info!("Reindexed {} pages from filesystem", count);
    Ok(count)
}

/// Strip Stratum block property lines from content that was previously saved in block format.
/// Removes `.id:`, `.marker:`, `.priority:` lines. For `.heading-level: N` lines, rewrites
/// the preceding block line with ATX heading markers (e.g., `## `) to preserve heading level.
fn strip_block_properties(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut out: Vec<String> = Vec::new();
    let mut last_content_idx: Option<usize> = None;

    for line in &lines {
        let trimmed = line.trim_start();

        if trimmed.starts_with(".id: ")
            || trimmed.starts_with(".marker: ")
            || trimmed.starts_with(".priority: ")
        {
            continue;
        }

        if let Some(hl_val) = trimmed.strip_prefix(".heading-level: ") {
            if let Ok(hl @ 1..=6) = hl_val.trim().parse::<u8>() {
                if let Some(idx) = last_content_idx {
                    let marker = "#".repeat(hl as usize);
                    let prev = out[idx]
                        .trim_start()
                        .strip_prefix("- ")
                        .unwrap_or(&out[idx]);
                    out[idx] = format!("{} {}", marker, prev);
                }
            }
            continue;
        }

        last_content_idx = Some(out.len());
        out.push(line.to_string());
    }

    out.join("\n")
}

/// Re-read a single .md file from disk and re-parse it using the plain-text converter,
/// ignoring any `- ` block syntax. This is the "Reindex Note" operation.
fn reparse_page_from_disk(
    store: &pkm_block::BlockStore,
    rel: &str,
    vault_path: &Path,
    block_index: Option<&mut BlockIndex>,
) -> Result<bool, String> {
    let full = vault_path.join(rel);
    let content = std::fs::read_to_string(&full).map_err(|e| e.to_string())?;
    // Strip block property lines before conversion to avoid .id: uuid appearing as paragraph blocks
    let cleaned = strip_block_properties(&content);
    let (fm, _, blocks) = pkm_markdown::block_parser::parse_document_as_plain_markdown(&cleaned);

    let mut page = pkm_block::Page::new(full, vault_path);
    page.frontmatter = pkm_block::PageFrontmatter {
        title: fm.title,
        created: fm.created,
        modified: fm.modified,
        tags: fm.tags,
        aliases: fm.aliases,
        ..Default::default()
    };
    store.upsert_page(&page).map_err(|e| e.to_string())?;
    store
        .delete_blocks_by_page(rel)
        .map_err(|e| e.to_string())?;
    for block in &blocks {
        store.insert_block(block, rel).map_err(|e| e.to_string())?;
    }

    // Rebuild Tantivy search index for this page
    if let Some(block_index) = block_index {
        for block in &blocks {
            let _ = block_index.index_block(block, rel);
        }
    }

    Ok(true)
}

/// Re-sync a single page from disk into SQLite, always using the plain-text converter.
/// Useful for reindexing externally-created or previously-saved notes.
/// Returns the number of blocks parsed from the page.
#[tauri::command]
pub async fn reindex_page(
    path: String,
    state: tauri::State<'_, AppState>,
) -> Result<usize, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let _guard = IndexingGuard::new(&state)?;
    let store = pkm_block::BlockStore::open(&state.db_path).map_err(|e| e.to_string())?;
    // Create a local BlockIndex for this operation (IndexingGuard prevents state mutation)
    let mut local_block_index = BlockIndex::create(&state.vault_path.join(".pkm").join("search"))
        .map_err(|e| e.to_string())?;
    if reparse_page_from_disk(
        &store,
        &path,
        &state.vault_path,
        Some(&mut local_block_index),
    )? {
        local_block_index.flush().map_err(|e| e.to_string())?;
        let blocks = store.get_blocks_by_page(&path).map_err(|e| e.to_string())?;
        Ok(blocks.len())
    } else {
        Ok(0)
    }
}

#[tauri::command]
pub async fn open_page(path: String, state: tauri::State<'_, AppState>) -> Result<PageDto, String> {
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
    store
        .delete_blocks_by_page(&path)
        .map_err(|e| e.to_string())?;

    // Insert blocks
    for block in &blocks {
        store
            .insert_block(block, &path)
            .map_err(|e| e.to_string())?;
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
    let mut state = state.lock().map_err(|e| e.to_string())?;

    // Ensure path has .md extension
    let path = if path.ends_with(".md") {
        path
    } else {
        format!("{}.md", path)
    };

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

    // Parse content into blocks and upsert in SQLite so the page appears in list_pages
    let (_fm, _, blocks) = pkm_markdown::block_parser::parse_document(&content);
    let store = pkm_block::BlockStore::open(&state.db_path).map_err(|e| e.to_string())?;
    let mut page = pkm_block::Page::new(full_path, &state.vault_path);
    page.frontmatter.title = title.clone();
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

    Ok(PageDto {
        path: path.clone(),
        slug: std::path::Path::new(&path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("untitled")
            .to_string(),
        title,
        block_count: blocks.len(),
        modified_at: chrono::Utc::now().to_rfc3339(),
    })
}

#[tauri::command]
pub async fn delete_page(path: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let full_path = state.vault_path.join(&path);

    if full_path.exists() {
        std::fs::remove_file(&full_path).map_err(|e| e.to_string())?;
    }

    let store = pkm_block::BlockStore::open(&state.db_path).map_err(|e| e.to_string())?;
    store.delete_page(&path).map_err(|e| e.to_string())?;

    Ok(())
}
