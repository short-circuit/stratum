//! Search commands: full-text block search plus re-exports of the backlink,
//! autocomplete and suggestion DTOs/methods.
//!
//! Split into [`dto`] (wire types), [`backlinks`] (index rebuild, backlink
//! listing, snippet/context), [`autocomplete`] (autocomplete + tag search) and
//! [`suggestions`] (related-note suggestions).

pub mod autocomplete;
pub mod backlinks;
pub mod dto;
pub mod suggestions;

pub use backlinks::build_backlink_snippet_from_store;
pub use dto::{
    AutocompleteItem, BacklinkContextDto, BacklinkDto, BacklinkSnippetDto, ConnectionSuggestion,
    SearchResultDto, SearchResultsDto,
};

use crate::state::StratumService;

impl StratumService {
    /// Full-text block search over the Tantivy index, dispatching the
    /// `onSearch` plugin hook afterwards.
    pub async fn search_blocks(
        &self,
        query: String,
        limit: Option<usize>,
    ) -> Result<SearchResultsDto, String> {
        let mut state = self.lock()?;
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
            crate::plugins::dispatch_on_search(manager, &query, limit);
        }

        Ok(SearchResultsDto { results: dtos })
    }
}
