//! Search and query commands.

use crate::commands::vault::{AppState, IndexingGuard};
use pkm_index::block_search::BlockIndex;
use pkm_markdown::linker::extract_links;
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::collections::HashMap;
use tauri::Emitter;
use tracing::{debug, info};

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
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let _guard = IndexingGuard::new(&state)?;
    let store = state.get_store().map_err(|e| e.to_string())?;
    let pages = store.list_pages().map_err(|e| e.to_string())?;

    // Create a local BlockIndex for rebuild (IndexingGuard prevents state mutation)
    let mut block_index = BlockIndex::create(&state.vault_path.join(".pkm").join("search"))
        .map_err(|e| e.to_string())?;

    let total = pages.len();
    let mut count = 0usize;
    for (i, page_path) in pages.iter().enumerate() {
        let _ = app.emit(
            "reindex-progress",
            super::ProgressEventPayload {
                message: format!("Indexing page {}/{}", i + 1, total),
                percent: if total > 0 {
                    (i as f32 + 1.0) / total as f32
                } else {
                    1.0
                },
            },
        );

        let blocks = store
            .get_blocks_by_page(page_path)
            .map_err(|e| e.to_string())?;
        for block in &blocks {
            block_index.index_block(block, page_path).ok();
            count += 1;
        }
    }

    block_index.flush().map_err(|e| e.to_string())?;
    let _ = app.emit(
        "reindex-progress",
        super::ProgressEventPayload {
            message: format!("Indexed {} blocks from {} pages", count, total),
            percent: 1.0,
        },
    );
    Ok(format!(
        "Indexed {} blocks from {} pages",
        count,
        pages.len()
    ))
}

#[tauri::command]
pub async fn search_blocks(
    query: String,
    limit: Option<usize>,
    state: tauri::State<'_, AppState>,
) -> Result<SearchResultsDto, String> {
    let mut state = state.lock().map_err(|e| e.to_string())?;
    let limit = limit.unwrap_or(20);

    let block_index = state.ensure_block_index()?;

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
    let store = state.get_store().map_err(|e| e.to_string())?;
    let page_backlinks = store
        .get_backlinks_for_page(&page_path)
        .map_err(|e| e.to_string())?;

    // Build a set of identifiers for this page (slug, title, path stem)
    let page_stem = std::path::Path::new(&page_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(&page_path)
        .to_string();
    let page_slug = page_stem.replace(' ', "-").to_lowercase();
    let page_display = page_stem.replace('-', " ");

    // Collect known page identifiers for link resolution
    let all_pages = store.list_pages().map_err(|e| e.to_string())?;
    let mut page_identifiers: Vec<String> = Vec::new();
    page_identifiers.push(page_stem.clone());
    page_identifiers.push(page_slug.clone());
    page_identifiers.push(page_display.clone());
    if let Ok(Some(pg)) = store.get_page(&page_path) {
        if let Some(ref t) = pg.title {
            page_identifiers.push(t.clone());
            page_identifiers.push(t.to_lowercase());
        }
    }

    let mut results = Vec::new();
    let mut seen_source_ids: std::collections::HashSet<String> =
        page_backlinks.iter().cloned().collect();

    // First, extract wiki-links on-the-fly from all blocks (like the graph does)
    for other_page in &all_pages {
        if let Ok(blocks) = store.get_blocks_by_page(other_page) {
            for block in &blocks {
                let links = extract_links(&block.content);
                let is_linked = links.iter().any(|l| {
                    let t = l.target.trim().to_lowercase();
                    page_identifiers.iter().any(|id| id.to_lowercase() == t)
                });
                if is_linked && !seen_source_ids.contains(&block.id.to_string()) {
                    seen_source_ids.insert(block.id.to_string());
                    results.push(BacklinkDto {
                        source_id: block.id.to_string(),
                        source_page: other_page.clone(),
                        context: block.content.clone(),
                        is_linked: true,
                    });
                }
            }
        }
    }

    // Also find unlinked mentions via text matching in block content
    // Skip the current page itself (self-references cause no-op navigation)
    let lower_name = page_display.to_lowercase();

    for other_page in &all_pages {
        if other_page == &page_path {
            continue;
        }
        if let Ok(blocks) = store.get_blocks_by_page(other_page) {
            for block in &blocks {
                if block.content.to_lowercase().contains(&lower_name)
                    && !seen_source_ids.contains(&block.id.to_string())
                {
                    seen_source_ids.insert(block.id.to_string());
                    results.push(BacklinkDto {
                        source_id: block.id.to_string(),
                        source_page: other_page.clone(),
                        context: block.content.clone(),
                        is_linked: false,
                    });
                }
                if results.len() > 50 {
                    break;
                }
            }
        }
        if results.len() > 50 {
            break;
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
    let store = state.get_store().map_err(|e| e.to_string())?;
    let id = uuid::Uuid::parse_str(&block_id).map_err(|e| e.to_string())?;
    let source_ids = store
        .get_backlinks_for_block(id)
        .map_err(|e| e.to_string())?;

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
    let store = state.get_store().map_err(|e| e.to_string())?;
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
        "backlink" => {
            let pages = store.list_pages().map_err(|e| e.to_string())?;
            let lower = query.to_lowercase();

            // Build slug → path lookup map
            let mut slug_to_path: HashMap<String, String> = HashMap::new();
            for path in &pages {
                let slug = std::path::Path::new(&path)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or(path)
                    .to_string();
                slug_to_path.insert(slug, path.clone());
            }

            // Count incoming links per page using indexed SQL query
            let link_counts = store.get_backlink_counts().map_err(|e| e.to_string())?;
            let mut incoming_count: HashMap<String, usize> = HashMap::new();
            for (target_page, cnt) in link_counts {
                let target_slug = target_page.replace(' ', "-").to_lowercase();
                // Only count links to pages that actually exist
                if slug_to_path.contains_key(&target_slug) {
                    *incoming_count.entry(target_slug).or_default() += cnt as usize;
                }
            }

            // Collect results, filtered by query
            let mut results: Vec<(usize, String, String)> = Vec::new();
            for path in pages {
                let slug = std::path::Path::new(&path)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or(&path)
                    .to_string();
                if slug.to_lowercase().contains(&lower) || path.to_lowercase().contains(&lower) {
                    let count = incoming_count.get(&slug).copied().unwrap_or(0);
                    results.push((count, slug, path));
                }
            }

            // Sort by incoming link count descending, take top 10
            results.sort_by_key(|b| Reverse(b.0));
            for (_, slug, path) in results.into_iter().take(10) {
                items.push(AutocompleteItem {
                    text: slug.replace('-', " "),
                    kind: "backlink".into(),
                    detail: Some(path),
                });
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BacklinkContextDto {
    pub block_id: String,
    pub content: String,
    pub page_title: Option<String>,
}

#[tauri::command]
pub async fn get_backlink_context(
    target_page: String,
    current_page: String,
    state: tauri::State<'_, AppState>,
) -> Result<Option<BacklinkContextDto>, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let store = state.get_store().map_err(|e| e.to_string())?;

    let current_slug = std::path::Path::new(&current_page)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();
    let current_title = current_slug.replace('-', " ");

    let page_fm = store.get_page(&target_page).ok().flatten();
    let page_title = page_fm.and_then(|f| f.title);

    let blocks = store
        .get_blocks_by_page(&target_page)
        .map_err(|e| e.to_string())?;

    // Find the first block in target_page that contains a [[link]] to current_page
    for block in &blocks {
        let links = pkm_markdown::linker::extract_links(&block.content);
        for link in &links {
            let link_lower = link.target.to_lowercase();
            if link_lower == current_slug.to_lowercase()
                || link_lower == current_title.to_lowercase()
                || link_lower == current_title.replace('-', " ").to_lowercase()
            {
                return Ok(Some(BacklinkContextDto {
                    block_id: block.id.to_string(),
                    content: block.content.clone(),
                    page_title: page_title.clone(),
                }));
            }
        }
    }

    Ok(None)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConnectionSuggestion {
    pub title: String,
    pub page_path: String,
    pub score: usize,
    pub snippet: String,
}

#[tauri::command]
pub async fn suggest_connections(
    page_path: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<ConnectionSuggestion>, String> {
    info!("page={}", page_path);

    let (store, index_path) = {
        let s = state.lock().map_err(|e| e.to_string())?;
        let store = s.get_store().map_err(|e| e.to_string())?;
        (store, s.vault_path.join(".pkm").join("search"))
    };
    let current_blocks = store
        .get_blocks_by_page(&page_path)
        .map_err(|e| e.to_string())?;

    let current_slug = std::path::Path::new(&page_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();

    let current_text: String = current_blocks
        .iter()
        .map(|b| b.content.as_str())
        .collect::<Vec<_>>()
        .join(" ");

    let results = pkm_index::related::RelatedFinder::new()
        .find_related(&store, &index_path, &current_text, Some(&current_slug))
        .map_err(|e| e.to_string())?;

    let suggestions: Vec<ConnectionSuggestion> = results
        .into_iter()
        .map(|r| ConnectionSuggestion {
            title: r.title,
            page_path: r.page_path,
            score: r.score,
            snippet: r.snippet,
        })
        .collect();

    debug!("found {} connections", suggestions.len());
    Ok(suggestions)
}

#[tauri::command]
pub async fn search_by_tag(
    tag: String,
    state: tauri::State<'_, AppState>,
) -> Result<SearchResultsDto, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let store = state.get_store().map_err(|e| e.to_string())?;
    let pages = store.list_pages().map_err(|e| e.to_string())?;
    let tag_lower = tag.to_lowercase();
    let tag_pattern = format!("#{}", tag_lower);

    let mut results = Vec::new();
    let mut seen_pages = std::collections::HashSet::new();

    for page_path in &pages {
        // Check page frontmatter tags
        let has_tag = store
            .get_page(page_path)
            .ok()
            .flatten()
            .map(|fm| fm.tags.iter().any(|t| t.to_lowercase() == tag_lower))
            .unwrap_or(false);

        if let Ok(blocks) = store.get_blocks_by_page(page_path) {
            for block in &blocks {
                // Check inline #tag in block content
                let has_inline_tag = block.content.to_lowercase().contains(&tag_pattern);

                if has_tag || has_inline_tag {
                    let key = format!("{}:{}", page_path, block.id);
                    if seen_pages.insert(key) {
                        results.push(SearchResultDto {
                            block_id: block.id.to_string(),
                            content: block.content.clone(),
                            page_path: page_path.clone(),
                            snippet: block.content.clone(),
                            score: 1.0,
                        });
                    }
                }
            }
        }
    }

    Ok(SearchResultsDto { results })
}
