//! Related-note suggestions for the "Suggested connections" panel.

use super::dto::ConnectionSuggestion;
use crate::state::StratumService;
use tracing::{debug, info};

impl StratumService {
    /// Suggest related pages for `page_path` using the Tantivy similarity index.
    pub async fn suggest_connections(
        &self,
        page_path: String,
    ) -> Result<Vec<ConnectionSuggestion>, String> {
        info!("page={}", page_path);

        let (store, index_path) = {
            let s = self.lock()?;
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
}
