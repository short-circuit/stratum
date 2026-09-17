//! Data-transfer objects shared by search and backlink commands.

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
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BacklinkDto {
    pub source_id: String,
    pub source_page: String,
    pub context: String,
    pub is_linked: bool,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AutocompleteItem {
    pub text: String,
    pub kind: String, // "page", "block", "tag", "command"
    pub detail: Option<String>,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BacklinkContextDto {
    pub block_id: String,
    pub content: String,
    pub page_title: Option<String>,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConnectionSuggestion {
    pub title: String,
    pub page_path: String,
    pub score: usize,
    pub snippet: String,
}
