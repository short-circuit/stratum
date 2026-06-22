use serde::{Deserialize, Serialize};

/// FFI-safe types for the Flutter frontend bridge.
/// These types serialize to/from JSON for flutter_rust_bridge.

/// Request to open a note.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenNoteRequest {
    pub path: String,
}

/// Response with parsed note data for the editor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenNoteResponse {
    pub path: String,
    pub title: String,
    pub frontmatter: FrontmatterDto,
    pub body: String,
    pub links: Vec<LinkDto>,
    pub tags: Vec<TagDto>,
    pub backlinks: Vec<BacklinkDto>,
    pub unlinked_mentions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontmatterDto {
    pub title: Option<String>,
    pub created: Option<String>,
    pub modified: Option<String>,
    pub tags: Vec<String>,
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkDto {
    pub target: String,
    pub display_text: Option<String>,
    pub resolved: bool,
    pub line: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagDto {
    pub name: String,
    pub source: String, // "frontmatter" or "inline"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacklinkDto {
    pub source_path: String,
    pub source_title: String,
    pub context_snippet: String,
    pub line: u32,
}

/// Request to save a note.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveNoteRequest {
    pub path: String,
    pub frontmatter: FrontmatterDto,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveNoteResponse {
    pub path: String,
    pub success: bool,
    pub error: Option<String>,
}

/// Search request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    pub mode: String, // "fulltext", "semantic", "graph", "regex"
    pub limit: u32,
}

/// Search result for Flutter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultDto {
    pub path: String,
    pub title: String,
    pub snippet: String,
    pub score: f64,
    pub matched_terms: Vec<String>,
}

/// Sync request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRequest {
    pub action: String, // "push", "pull", "sync", "status"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStatusDto {
    pub status: String, // "idle", "syncing", "conflicts", "error", "up_to_date"
    pub last_sync: Option<String>,
    pub pending_commits: u32,
    pub conflict_files: Vec<String>,
}

/// Graph data for the graph view.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphDto {
    pub nodes: Vec<GraphNodeDto>,
    pub edges: Vec<GraphEdgeDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNodeDto {
    pub id: String,
    pub label: String,
    pub slug: String,
    pub tags: Vec<String>,
    pub link_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdgeDto {
    pub source: String,
    pub target: String,
    pub label: Option<String>,
}

/// Vault statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultStatsDto {
    pub note_count: u32,
    pub tag_count: u32,
    pub link_count: u32,
    pub total_size_bytes: u64,
    pub indexed: bool,
}

/// Plugin info for frontend display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDto {
    pub name: String,
    pub enabled: bool,
    pub version: String,
    pub description: String,
    pub hooks: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_note_response_serialization() {
        let resp = OpenNoteResponse {
            path: "notes/test.md".to_string(),
            title: "Test Note".to_string(),
            frontmatter: FrontmatterDto {
                title: Some("Test Note".to_string()),
                created: Some("2026-06-22".to_string()),
                modified: None,
                tags: vec!["test".to_string()],
                aliases: vec![],
            },
            body: "# Test\n\nHello world.".to_string(),
            links: vec![LinkDto {
                target: "Other Note".to_string(),
                display_text: None,
                resolved: true,
                line: 3,
            }],
            tags: vec![TagDto {
                name: "test".to_string(),
                source: "frontmatter".to_string(),
            }],
            backlinks: vec![],
            unlinked_mentions: vec![],
        };

        let json = serde_json::to_string(&resp).unwrap();
        let deserialized: OpenNoteResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.title, "Test Note");
        assert_eq!(deserialized.links.len(), 1);
        assert!(deserialized.backlinks.is_empty());
    }

    #[test]
    fn test_search_result_dto() {
        let result = SearchResultDto {
            path: "notes/quantum.md".to_string(),
            title: "Quantum Computing".to_string(),
            snippet: "... **quantum** superposition ...".to_string(),
            score: 0.95,
            matched_terms: vec!["quantum".to_string()],
        };
        assert!(result.score > 0.9);
        assert_eq!(result.matched_terms[0], "quantum");
    }

    #[test]
    fn test_sync_status_dto() {
        let status = SyncStatusDto {
            status: "up_to_date".to_string(),
            last_sync: Some("2026-06-22T14:30:00Z".to_string()),
            pending_commits: 0,
            conflict_files: vec![],
        };
        assert_eq!(status.status, "up_to_date");
        assert!(status.conflict_files.is_empty());
    }

    #[test]
    fn test_graph_dto_roundtrip() {
        let graph = GraphDto {
            nodes: vec![
                GraphNodeDto {
                    id: "note1".to_string(),
                    label: "Note 1".to_string(),
                    slug: "note-1".to_string(),
                    tags: vec!["tag1".to_string()],
                    link_count: 2,
                },
                GraphNodeDto {
                    id: "note2".to_string(),
                    label: "Note 2".to_string(),
                    slug: "note-2".to_string(),
                    tags: vec![],
                    link_count: 1,
                },
            ],
            edges: vec![GraphEdgeDto {
                source: "note1".to_string(),
                target: "note2".to_string(),
                label: Some("related".to_string()),
            }],
        };

        let json = serde_json::to_string(&graph).unwrap();
        let deserialized: GraphDto = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.nodes.len(), 2);
        assert_eq!(deserialized.edges.len(), 1);
    }

    #[test]
    fn test_save_note_response() {
        let resp = SaveNoteResponse {
            path: "notes/test.md".to_string(),
            success: true,
            error: None,
        };
        assert!(resp.success);
        assert!(resp.error.is_none());

        let err_resp = SaveNoteResponse {
            path: "notes/bad.md".to_string(),
            success: false,
            error: Some("Permission denied".to_string()),
        };
        assert!(!err_resp.success);
        assert_eq!(err_resp.error.unwrap(), "Permission denied");
    }

    #[test]
    fn test_vault_stats_dto() {
        let stats = VaultStatsDto {
            note_count: 42,
            tag_count: 15,
            link_count: 128,
            total_size_bytes: 1_048_576,
            indexed: true,
        };
        assert_eq!(stats.note_count, 42);
        assert_eq!(stats.total_size_bytes, 1_048_576);
        assert!(stats.indexed);
    }
}
