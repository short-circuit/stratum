//! Search and query commands.

use crate::commands::vault::AppState;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchResultDto {
    pub block_id: String,
    pub content: String,
    pub page_path: String,
    pub snippet: String,
    pub score: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchResultsDto {
    pub results: Vec<SearchResultDto>,
}

#[tauri::command]
pub async fn rebuild_search_index(
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let store = pkm_block::BlockStore::open(&state.db_path).map_err(|e| e.to_string())?;
    let pages = store.list_pages().map_err(|e| e.to_string())?;

    let index_path = state.vault_path.join(".pkm").join("search");
    let mut block_index =
        pkm_index::block_search::BlockIndex::create(&index_path).map_err(|e| e.to_string())?;

    let mut count = 0usize;
    for page_path in &pages {
        let blocks = store.get_blocks_by_page(page_path).map_err(|e| e.to_string())?;
        for block in &blocks {
            block_index.index_block(block, page_path).ok();
            count += 1;
        }
    }

    block_index.flush().map_err(|e| e.to_string())?;
    Ok(format!("Indexed {} blocks from {} pages", count, pages.len()))
}

#[tauri::command]
pub async fn search_blocks(
    query: String,
    limit: Option<usize>,
    state: tauri::State<'_, AppState>,
) -> Result<SearchResultsDto, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let limit = limit.unwrap_or(20);

    let index_path = state.vault_path.join(".pkm").join("search");
    let block_index =
        pkm_index::block_search::BlockIndex::create(&index_path).map_err(|e| e.to_string())?;

    let results = block_index
        .search(&query, limit)
        .map_err(|e| e.to_string())?;

    let dtos: Vec<SearchResultDto> = results
        .into_iter()
        .map(|r| SearchResultDto {
            block_id: r.block_id,
            content: r.content,
            page_path: r.page_path,
            snippet: r.snippet,
            score: r.score,
        })
        .collect();

    Ok(SearchResultsDto { results: dtos })
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BacklinkDto {
    pub source_id: String,
    pub source_page: String,
    pub context: String,
    pub is_linked: bool,
}

#[tauri::command]
pub async fn get_page_backlinks(
    page_path: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<BacklinkDto>, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let store = pkm_block::BlockStore::open(&state.db_path).map_err(|e| e.to_string())?;
    let page_backlinks = store.get_backlinks_for_page(&page_path).map_err(|e| e.to_string())?;

    let mut results = Vec::new();
    for src_str in &page_backlinks {
        if let Ok(src_id) = uuid::Uuid::parse_str(src_str) {
            if let Ok(block) = store.get_block(src_id) {
                results.push(BacklinkDto {
                    source_id: src_str.clone(),
                    source_page: page_path.clone(),
                    context: block.content,
                    is_linked: true,
                });
            }
        }
    }

    // Also find unlinked mentions via page name in block content
    let page_name = std::path::Path::new(&page_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(&page_path)
        .replace('-', " ");
    let lower_name = page_name.to_lowercase();

    let all_pages = store.list_pages().map_err(|e| e.to_string())?;
    let linked_ids: std::collections::HashSet<String> =
        page_backlinks.iter().cloned().collect();

    for other_page in all_pages {
        let blocks = store.get_blocks_by_page(&other_page).map_err(|e| e.to_string())?;
        for block in blocks {
            if block.content.to_lowercase().contains(&lower_name)
                && !linked_ids.contains(&block.id.to_string())
            {
                results.push(BacklinkDto {
                    source_id: block.id.to_string(),
                    source_page: other_page.clone(),
                    context: block.content,
                    is_linked: false,
                });
            }
            // Limit unlinked results
            if results.len() > 50 {
                break;
            }
        }
    }

    Ok(results)
}

#[tauri::command]
pub async fn get_backlinks(
    block_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<SearchResultsDto, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let store = pkm_block::BlockStore::open(&state.db_path).map_err(|e| e.to_string())?;
    let id = uuid::Uuid::parse_str(&block_id).map_err(|e| e.to_string())?;
    let source_ids = store.get_backlinks_for_block(id).map_err(|e| e.to_string())?;

    let mut results = Vec::new();
    for src_str in source_ids {
        if let Ok(src_id) = uuid::Uuid::parse_str(&src_str) {
            if let Ok(block) = store.get_block(src_id) {
                results.push(SearchResultDto {
                    block_id: src_str.clone(),
                    content: block.content,
                    page_path: String::new(),
                    snippet: String::new(),
                    score: 0.0,
                });
            }
        }
    }

    Ok(SearchResultsDto { results })
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AutocompleteItem {
    pub text: String,
    pub kind: String, // "page", "block", "tag", "command"
    pub detail: Option<String>,
}

#[tauri::command]
pub async fn autocomplete(
    query: String,
    kind: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<AutocompleteItem>, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let store = pkm_block::BlockStore::open(&state.db_path).map_err(|e| e.to_string())?;
    let mut items = Vec::new();

    match kind.as_str() {
        "page" => {
            let pages = store.list_pages().map_err(|e| e.to_string())?;
            let lower = query.to_lowercase();
            for path in pages {
                let slug = std::path::Path::new(&path)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or(&path);
                if slug.to_lowercase().contains(&lower) || path.to_lowercase().contains(&lower) {
                    items.push(AutocompleteItem {
                        text: slug.replace('-', " "),
                        kind: "page".into(),
                        detail: Some(path),
                    });
                }
                if items.len() >= 10 {
                    break;
                }
            }
        }
        "tag" => {
            let pages = store.list_pages().map_err(|e| e.to_string())?;
            let mut seen = std::collections::HashSet::new();
            let lower = query.to_lowercase();
            for path in pages {
                if let Ok(blocks) = store.get_blocks_by_page(&path) {
                    for block in blocks {
                        for tag in block.properties.keys() {
                            if tag.to_lowercase().contains(&lower) && seen.insert(tag.clone()) {
                                items.push(AutocompleteItem {
                                    text: tag.clone(),
                                    kind: "tag".into(),
                                    detail: Some(block.content.clone()),
                                });
                            }
                        }
                    }
                }
                if items.len() >= 10 {
                    break;
                }
            }
        }
        "block" => {
            // Search blocks by content
            let lower = query.to_lowercase();
            let pages = store.list_pages().map_err(|e| e.to_string())?;
            for path in pages {
                if let Ok(blocks) = store.get_blocks_by_page(&path) {
                    for block in blocks {
                        if block.content.to_lowercase().contains(&lower) {
                            items.push(AutocompleteItem {
                                text: block.content.chars().take(60).collect(),
                                kind: "block".into(),
                                detail: Some(format!("{} ({})", path, block.id)),
                            });
                        }
                        if items.len() >= 10 {
                            break;
                        }
                    }
                }
                if items.len() >= 10 {
                    break;
                }
            }
        }
        _ => {}
    }

    Ok(items)
}
