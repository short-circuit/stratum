//! Backlink commands: rebuild the search index, list linked/unlinked
//! mentions, and build bounded backlink snippets/context windows.

use super::dto::{
    BacklinkContextDto, BacklinkDto, BacklinkSnippetDto, SearchResultDto, SearchResultsDto,
};
use crate::events::{EventSink, ProgressEventPayload};
use crate::state::{IndexingGuard, StratumService};
use pkm_core::error::PkmError;
use pkm_index::block_search::BlockIndex;
use pkm_markdown::linker::extract_links;
use std::collections::HashSet;

impl StratumService {
    /// Rebuild the Tantivy block index from SQLite, emitting progress events.
    pub async fn rebuild_search_index(&self, sink: &dyn EventSink) -> Result<String, String> {
        let state = self.lock()?;
        let _guard = IndexingGuard::new(&state)?;
        let store = state.get_store().map_err(|e| e.to_string())?;
        let pages = store.list_pages().map_err(|e| e.to_string())?;

        // Create a local BlockIndex for rebuild (IndexingGuard prevents state mutation)
        let mut block_index = BlockIndex::create(&state.vault_path.join(".pkm").join("search"))
            .map_err(|e| e.to_string())?;

        let total = pages.len();
        let mut count = 0usize;
        for (i, page_path) in pages.iter().enumerate() {
            sink.emit(
                "reindex-progress",
                serde_json::to_value(ProgressEventPayload {
                    message: format!("Indexing page {}/{}", i + 1, total),
                    percent: if total > 0 {
                        (i as f32 + 1.0) / total as f32
                    } else {
                        1.0
                    },
                })
                .unwrap_or(serde_json::Value::Null),
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
        sink.emit(
            "reindex-progress",
            serde_json::to_value(ProgressEventPayload {
                message: format!("Indexed {} blocks from {} pages", count, total),
                percent: 1.0,
            })
            .unwrap_or(serde_json::Value::Null),
        );
        Ok(format!(
            "Indexed {} blocks from {} pages",
            count,
            pages.len()
        ))
    }

    /// List all blocks that link to `page_path` (linked + unlinked mentions).
    pub async fn get_page_backlinks(&self, page_path: String) -> Result<Vec<BacklinkDto>, String> {
        let state = self.lock()?;
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
        let mut seen_source_ids: HashSet<String> = page_backlinks.iter().cloned().collect();

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

    /// List blocks that link to `block_id`.
    pub async fn get_backlinks(&self, block_id: String) -> Result<SearchResultsDto, String> {
        let state = self.lock()?;
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

    /// Build a bounded snippet around a backlinked anchor block.
    pub async fn get_backlink_snippet(
        &self,
        note_id: String,
        backlink_ref: String,
    ) -> Result<BacklinkSnippetDto, String> {
        let state = self.lock()?;
        let store = state.get_store().map_err(|e| e.to_string())?;
        build_backlink_snippet_from_store(&store, &note_id, &backlink_ref)
            .map_err(|e| e.to_string())
    }

    /// Find the first block in `target_page` that wiki-links to `current_page`.
    pub async fn get_backlink_context(
        &self,
        target_page: String,
        current_page: String,
    ) -> Result<Option<BacklinkContextDto>, String> {
        let state = self.lock()?;
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
