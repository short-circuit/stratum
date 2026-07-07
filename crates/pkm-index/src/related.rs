//! Shared "find related pages" engine.
//!
//! Extracts the common logic from `suggest_connections` (search.rs) and
//! `ai_interlink_notes` (ai.rs) — both independently rebuilt a Tantivy index,
//! extracted keywords, searched, scored, and ranked. This module provides a
//! single [`RelatedFinder`] builder with configurable split predicate,
//! result count, and keyword length threshold.

use pkm_block::BlockStore;
use pkm_core::fs_util::truncate_text;
use pkm_core::{PkmError, PkmResult};
use std::path::Path;

/// A single related-page result.
#[derive(Debug, Clone)]
pub struct RelatedPageResult {
    /// Page title (slug with hyphens replaced by spaces).
    pub title: String,
    /// Vault-relative page path.
    pub page_path: String,
    /// Computed relevance score.
    pub score: usize,
    /// Truncated content snippet.
    pub snippet: String,
}

/// Builder for finding related pages via Tantivy search + keyword supplement.
///
/// # Example
///
/// ```ignore
/// use pkm_index::related::RelatedFinder;
///
/// let results = RelatedFinder::new()
///     .max_results(10)
///     .find_related(&store, &index_path, "some text to search", Some("current-page-slug"))?;
/// ```
pub struct RelatedFinder {
    max_results: usize,
    min_keyword_len: usize,
    split_predicate: Box<dyn Fn(char) -> bool + Send + Sync>,
}

impl Default for RelatedFinder {
    fn default() -> Self {
        Self::new()
    }
}

impl RelatedFinder {
    /// Create a new `RelatedFinder` with default settings.
    ///
    /// Defaults: max 10 results, min keyword length 3, split on non-alphanumeric.
    pub fn new() -> Self {
        Self {
            max_results: 10,
            min_keyword_len: 3,
            split_predicate: Box::new(|c: char| !c.is_alphanumeric()),
        }
    }

    /// Set the maximum number of results to return (default: 10).
    pub fn max_results(mut self, n: usize) -> Self {
        self.max_results = n;
        self
    }

    /// Set the minimum keyword length for extracted query words (default: 3).
    pub fn min_keyword_len(mut self, n: usize) -> Self {
        self.min_keyword_len = n;
        self
    }

    /// Set a custom split predicate for extracting keywords from text.
    ///
    /// Default: `|c| !c.is_alphanumeric()`.
    ///
    /// `ai_interlink_notes` uses `|c| c.is_whitespace() || c == '#' || c == '*' || c == '[' || c == ']'`.
    pub fn split_predicate<F>(mut self, pred: F) -> Self
    where
        F: Fn(char) -> bool + Send + Sync + 'static,
    {
        self.split_predicate = Box::new(pred);
        self
    }

    /// Find pages related to `text`, skipping `current_slug` if provided.
    ///
    /// Internally:
    /// 1. Lists all pages from `store`
    /// 2. Builds a temporary Tantivy index with all block content
    /// 3. Extracts keywords from `text` using the configured split predicate
    /// 4. Searches Tantivy, deduplicates by slug
    /// 5. Supplements with keyword matching on remaining pages
    /// 6. Sorts by score descending, truncates to `max_results`
    pub fn find_related(
        &self,
        store: &BlockStore,
        index_path: &Path,
        text: &str,
        current_slug: Option<&str>,
    ) -> PkmResult<Vec<RelatedPageResult>> {
        let pages = store
            .list_pages()
            .map_err(|e| PkmError::Internal(e.to_string()))?;

        // Step 1: Build Tantivy index from all pages
        if let Ok(mut idx) = crate::block_search::BlockIndex::create(index_path) {
            for path in &pages {
                if let Ok(blocks) = store.get_blocks_by_page(path) {
                    for b in &blocks {
                        let _ = idx.index_block(b, path);
                    }
                }
            }
            let _ = idx.flush();
        }

        // Step 2: Extract keywords from input text
        let query_words: Vec<String> = text
            .split(|c| (self.split_predicate)(c))
            .filter(|w| w.len() > self.min_keyword_len)
            .map(|w| w.to_string())
            .collect();
        let query = query_words.join(" ");

        // Step 3: Search Tantivy
        let mut results: Vec<RelatedPageResult> = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

        if let Ok(idx) = crate::block_search::BlockIndex::create(index_path) {
            if let Ok(search_results) = idx.search(&query, self.max_results * 2) {
                for r in &search_results {
                    let slug = std::path::Path::new(&r.page_path)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("");

                    // Skip the current page
                    if let Some(cur) = current_slug {
                        if slug == cur {
                            continue;
                        }
                    }

                    if seen.insert(slug.to_string()) {
                        results.push(RelatedPageResult {
                            title: slug.replace('-', " "),
                            page_path: r.page_path.clone(),
                            score: (r.score * 100.0) as usize,
                            snippet: truncate_text(&r.content, 80),
                        });
                    }
                }
            }
        }

        // Step 4: Supplement with keyword matching
        let keywords: std::collections::HashSet<String> =
            query_words.iter().map(|w| w.to_lowercase()).collect();

        if !keywords.is_empty() {
            for path in &pages {
                let slug = std::path::Path::new(path)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("");

                // Skip current page
                if let Some(cur) = current_slug {
                    if slug == cur {
                        continue;
                    }
                }

                if seen.contains(slug) {
                    continue;
                }

                let title = slug.replace('-', " ");
                let title_lower = title.to_lowercase();
                let mut score: usize =
                    keywords.iter().filter(|k| title_lower.contains(*k)).count() * 3;

                if let Ok(blocks) = store.get_blocks_by_page(path) {
                    for b in &blocks {
                        let block_lower = b.content.to_lowercase();
                        score += keywords.iter().filter(|k| block_lower.contains(*k)).count();
                    }
                }

                if score > 0 {
                    let snippet = if let Ok(blocks) = store.get_blocks_by_page(path) {
                        blocks
                            .first()
                            .map(|b| truncate_text(&b.content, 80))
                            .unwrap_or_default()
                    } else {
                        String::new()
                    };

                    results.push(RelatedPageResult {
                        title,
                        page_path: path.clone(),
                        score,
                        snippet,
                    });
                    seen.insert(slug.to_string());
                }
            }
        }

        // Step 5: Sort and truncate
        results.sort_by_key(|b| std::cmp::Reverse(b.score));
        results.truncate(self.max_results);

        Ok(results)
    }
}
