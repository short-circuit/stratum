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
