//! Autocomplete and tag search commands.

use super::dto::{AutocompleteItem, SearchResultDto, SearchResultsDto};
use crate::state::StratumService;
use std::cmp::Reverse;
use std::collections::HashMap;

impl StratumService {
    /// Autocomplete pages, tags, backlink targets or block content by `kind`.
    pub async fn autocomplete(
        &self,
        query: String,
        kind: String,
    ) -> Result<Vec<AutocompleteItem>, String> {
        let state = self.lock()?;
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
                    if slug.to_lowercase().contains(&lower) || path.to_lowercase().contains(&lower)
                    {
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
                    if slug.to_lowercase().contains(&lower) || path.to_lowercase().contains(&lower)
                    {
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

    /// Find blocks carrying `tag`, either as page frontmatter or inline `#tag`.
    pub async fn search_by_tag(&self, tag: String) -> Result<SearchResultsDto, String> {
        let state = self.lock()?;
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
}
