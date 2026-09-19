//! Contract DTOs (contract §4).
//!
//! These types mirror the existing command DTO shapes in
//! `src-tauri/src/commands/*` wherever the two surfaces overlap, but are
//! owned by this crate so the MCP server does not depend on the Tauri crate.
//! Field names are the exact contract wire names.
//!
//! All `path` values are vault-relative POSIX paths; `slug` is the page path
//! with `.md` removed; timestamps are RFC 3339 UTC.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A note's full content model (§4.1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteDocument {
    pub path: String,
    pub slug: String,
    pub title: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frontmatter: Option<NoteFrontmatter>,
    pub links: Vec<NoteLink>,
    pub tags: Vec<String>,
    pub block_count: usize,
    pub modified_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NoteFrontmatter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
}

/// A `[[wiki-link]]` target extracted from content (§4.1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteLink {
    pub target: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<usize>,
}

/// A block in the editor's DTO shape (§4.2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockDoc {
    pub id: String,
    pub content: String,
    #[serde(rename = "parent_id")]
    pub parent_id: Option<String>,
    #[serde(rename = "left_id")]
    pub left_id: Option<String>,
    pub properties: Vec<(String, String)>,
    pub marker: Option<String>,
    pub priority: Option<String>,
    pub collapsed: bool,
    pub heading_level: Option<u8>,
}

/// A search hit (§4.3).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub block_id: String,
    pub content: String,
    pub page_path: String,
    pub snippet: String,
    pub score: f32,
}

/// A backlink / unlinked mention (§4.4).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacklinkHit {
    pub source_id: String,
    pub source_page: String,
    pub context: String,
    pub is_linked: bool,
}

/// Graph node (matches `GraphNodeDto`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub title: String,
    pub path: String,
    pub tags: Vec<String>,
    pub degree: usize,
}

/// Graph edge (matches `GraphEdgeDto`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
    pub label: Option<String>,
}

/// Graph data (§4.5).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphData {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub node_count: usize,
    pub edge_count: usize,
    pub vault_path: String,
}

/// Vault info (§4.6).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultInfo {
    pub vault_path: String,
    pub page_count: usize,
    pub block_count: usize,
    pub index_fresh: bool,
    pub last_indexed_at: Option<String>,
}

/// Tag mutation result (§4.7).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagOperationResult {
    pub path: String,
    pub success: bool,
    pub tag: String,
    pub action: String,
    pub applied: usize,
}

/// Index status (§4.8).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStatus {
    pub index_fresh: bool,
    pub indexed_pages: usize,
    pub total_pages: usize,
    pub indexed_blocks: usize,
    pub last_indexed_at: Option<String>,
}

/// Page summary row from `kb_list_pages` (§5.2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageSummary {
    pub path: String,
    pub slug: String,
    pub title: String,
    pub block_count: usize,
    pub modified_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageListResponse {
    pub pages: Vec<PageSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub results: Vec<SearchHit>,
    pub total: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_offset: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutocompleteItem {
    pub text: String,
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutocompleteResponse {
    pub items: Vec<AutocompleteItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacklinksResponse {
    pub backlinks: Vec<BacklinkHit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolveLinkResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved: Option<ResolvedTarget>,
    pub unresolved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedTarget {
    pub path: String,
    pub slug: String,
    pub title: String,
}

/// Utility to produce RFC 3339 UTC timestamps.
pub fn now_rfc3339() -> String {
    Utc::now().to_rfc3339()
}

pub fn to_rfc3339(dt: DateTime<Utc>) -> String {
    dt.to_rfc3339()
}
