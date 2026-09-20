//! Search and query commands.

use crate::commands::vault::{AppState, IndexingGuard};
use pkm_core::error::PkmError;
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

    // Release Tantivy directory lock — search is read-only, no need to hold writer.
    drop(state.block_index.take());

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

    // Dispatch the `onSearch` hook to enabled plugins that declare it (spec §8).
    // Runs synchronously; a trapping plugin logs and is skipped, never aborting
    // the search.
    if let Some(manager) = state.plugin_manager.as_deref() {
        crate::commands::plugins::dispatch_on_search(manager, &query, limit);
    }

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
            if let Ok((block, page_path)) = store.get_block_with_page_path(src_id) {
                results.push(SearchResultDto {
                    block_id: src_str.clone(),
                    content: block.content,
                    page_path,
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

/// A snippet of a note's content surrounding a backlink anchor.
///
/// Returned by `get_backlink_snippet`. The `anchor_*` fields identify and
/// carry the exact block that is backlinked; `context` is a bounded window of
/// adjacent blocks (document order) providing surrounding context for display.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BacklinkSnippetDto {
    /// Vault-relative path of the note that contains the anchor (the ":id" in
    /// GET /api/notes/:id/backlink-snippet).
    pub note_id: String,
    /// Display title of the note (frontmatter title, falling back to slug).
    pub note_title: String,
    /// The block id of the backlinked anchor (the "?ref=" backlink-ref).
    pub anchor_id: String,
    /// The exact content of the backlinked block/paragraph.
    pub anchor_content: String,
    /// A window of content around the anchor for display. Includes the anchor
    /// block itself plus up to `context_before` prior and `context_after`
    /// subsequent blocks in document order.
    pub context: Vec<String>,
}

/// Build a backlink snippet for the note at `note_id`, anchored at the block
/// `anchor_id`.
///
/// * 404 for a missing note: `PkmError::NoteNotFound`
/// * 404 for a missing anchor: `PkmError::BlockNotFound`
///
/// Surrounding context is bounded to avoid shipping the whole note: at most
/// `CONTEXT_BEFORE` blocks before and `CONTEXT_AFTER` blocks after the anchor,
/// in the same document order the editor uses (block rowid order).
const CONTEXT_BEFORE: usize = 2;
const CONTEXT_AFTER: usize = 2;

pub fn build_backlink_snippet_from_store(
    store: &pkm_block::BlockStore,
    note_id: &str,
    anchor_id: &str,
) -> Result<BacklinkSnippetDto, PkmError> {
    let note_fm = store
        .get_page(note_id)?
        .ok_or_else(|| PkmError::NoteNotFound(note_id.to_string()))?;

    let anchor_uuid = uuid::Uuid::parse_str(anchor_id)
        .map_err(|_| PkmError::BlockNotFound(anchor_id.to_string()))?;
    // Early existence check: unknown block ids are a missing-anchor 404. The
    // returned `_page_path` is not used; membership on the requested note is
    // verified below against `blocks` so a valid id on a different page still
    // counts as a missing anchor.
    let (anchor, _page_path) = store
        .get_block_with_page_path(anchor_uuid)
        .map_err(|_| PkmError::BlockNotFound(anchor_id.to_string()))?;

    // Load the note's blocks in document order. `get_blocks_by_page` orders by
    // rowid (`ORDER BY rowid`), which is the same order the editor loads and
    // renders the page — so the context window below matches what a user sees.
    let blocks = store.get_blocks_by_page(note_id)?;

    // Verify the anchor actually lives on the requested note. A block id may
    // exist in the store while the caller passed a note id for a *different*
    // page; treat that as a missing anchor rather than silently returning
    // content from another note.
    let anchor_pos = blocks
        .iter()
        .position(|b| b.id == anchor_uuid)
        .ok_or_else(|| PkmError::BlockNotFound(anchor_id.to_string()))?;

    let start = anchor_pos.saturating_sub(CONTEXT_BEFORE);
    let end = (anchor_pos + CONTEXT_AFTER + 1).min(blocks.len());
    let context: Vec<String> = blocks[start..end]
        .iter()
        .map(|b| b.content.clone())
        .collect();

    let note_title = note_fm.title.unwrap_or_else(|| {
        std::path::Path::new(note_id)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(note_id)
            .replace('-', " ")
    });

    Ok(BacklinkSnippetDto {
        note_id: note_id.to_string(),
        note_title: note_title.clone(),
        anchor_id: anchor_id.to_string(),
        anchor_content: anchor.content.clone(),
        context,
    })
}

#[tauri::command]
pub async fn get_backlink_snippet(
    note_id: String,
    backlink_ref: String,
    state: tauri::State<'_, AppState>,
) -> Result<BacklinkSnippetDto, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let store = state.get_store().map_err(|e| e.to_string())?;
    build_backlink_snippet_from_store(&store, &note_id, &backlink_ref).map_err(|e| e.to_string())
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

#[cfg(test)]
mod tests {
    use super::*;
    use pkm_block::Block;
    use std::path::PathBuf;
    use uuid::Uuid;

    /// Insert a page into the store with the given vault-relative path and
    /// frontmatter title (if any).
    fn insert_page(
        store: &pkm_block::BlockStore,
        vault_root: &std::path::Path,
        rel_path: &str,
        title: Option<&str>,
    ) {
        let mut page = pkm_block::Page::new(vault_root.join(rel_path), vault_root);
        page.frontmatter.title = title.map(|t| t.to_string());
        store.upsert_page(&page).unwrap();
    }

    /// A test fixture with a source note that links to a target note.
    fn fixture() -> (pkm_block::BlockStore, Uuid) {
        let store = pkm_block::BlockStore::open_in_memory().unwrap();
        let vault_root = PathBuf::from("/tmp/test-vault");

        insert_page(&store, &vault_root, "pages/source.md", Some("Source"));
        insert_page(&store, &vault_root, "pages/target.md", Some("Target"));

        // Blocks on the source page in document order.
        let b1 = Block::new(Uuid::new_v4(), "Intro paragraph that sets context.".into());
        let b2 = Block::new(
            Uuid::new_v4(),
            "Here is a [[target]] backlink anchor.".into(),
        );
        let b3 = Block::new(Uuid::new_v4(), "Follow-up paragraph after the link.".into());

        store.insert_block(&b1, "pages/source.md").unwrap();
        store.insert_block(&b2, "pages/source.md").unwrap();
        store.insert_block(&b3, "pages/source.md").unwrap();

        // A separate block on the target page must not be picked up.
        let other = Block::new(Uuid::new_v4(), "Unrelated block on the target page.".into());
        store.insert_block(&other, "pages/target.md").unwrap();

        (store, b2.id)
    }

    #[test]
    fn backlink_snippet_success() {
        let (store, anchor) = fixture();
        let result =
            build_backlink_snippet_from_store(&store, "pages/source.md", &anchor.to_string())
                .unwrap();

        assert_eq!(result.note_id, "pages/source.md");
        assert_eq!(result.note_title, "Source");
        assert_eq!(result.anchor_id, anchor.to_string());
        assert_eq!(
            result.anchor_content,
            "Here is a [[target]] backlink anchor."
        );
        // Context window: the anchor block plus adjacent blocks in document order.
        assert!(result
            .context
            .contains(&"Here is a [[target]] backlink anchor.".to_string()));
        assert!(result
            .context
            .contains(&"Intro paragraph that sets context.".to_string()));
        assert!(result
            .context
            .contains(&"Follow-up paragraph after the link.".to_string()));
        // The unrelated block on the target page must not appear.
        assert!(!result
            .context
            .contains(&"Unrelated block on the target page.".to_string()));
    }

    #[test]
    fn backlink_snippet_missing_note() {
        let (store, anchor) = fixture();
        let err = build_backlink_snippet_from_store(&store, "pages/ghost.md", &anchor.to_string())
            .unwrap_err();
        assert!(
            matches!(err, PkmError::NoteNotFound(_)),
            "expected NoteNotFound, got {err}"
        );
    }

    #[test]
    fn backlink_snippet_missing_anchor() {
        let (store, _anchor) = fixture();
        // A well-formed UUID that does not exist in the store.
        let ghost = Uuid::new_v4();
        let err = build_backlink_snippet_from_store(&store, "pages/source.md", &ghost.to_string())
            .unwrap_err();
        assert!(
            matches!(err, PkmError::BlockNotFound(_)),
            "expected BlockNotFound, got {err}"
        );
    }

    #[test]
    fn backlink_snippet_anchor_from_wrong_note() {
        let (store, anchor) = fixture();
        // The anchor lives on pages/source.md; asking for pages/target.md with
        // that anchor must be a missing-anchor error (not scavenged content).
        let err = build_backlink_snippet_from_store(&store, "pages/target.md", &anchor.to_string())
            .unwrap_err();
        assert!(
            matches!(err, PkmError::BlockNotFound(_)),
            "expected BlockNotFound, got {err}"
        );
    }

    #[test]
    fn backlink_snippet_malformed_anchor() {
        let (store, _anchor) = fixture();
        let err =
            build_backlink_snippet_from_store(&store, "pages/source.md", "not-a-uuid").unwrap_err();
        assert!(
            matches!(err, PkmError::BlockNotFound(_)),
            "expected BlockNotFound, got {err}"
        );
    }
}
