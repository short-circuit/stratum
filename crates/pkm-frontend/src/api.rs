use crate::types::*;
use pkm_core::PkmResult;

/// High-level API that the Flutter frontend calls via FFI.
/// Each method is FFI-friendly (serializes to/from JSON).

/// Vault handle for the frontend API.
pub struct FrontendApi {
    vault_path: String,
    // These would be initialized with real engines in production
}

impl FrontendApi {
    /// Create a new frontend API for the given vault path.
    pub fn new(vault_path: impl Into<String>) -> Self {
        Self {
            vault_path: vault_path.into(),
        }
    }

    /// Get the vault path.
    pub fn vault_path(&self) -> &str {
        &self.vault_path
    }

    /// Open a note and return its contents with metadata.
    pub fn open_note(&self, request: OpenNoteRequest) -> PkmResult<OpenNoteResponse> {
        let path = std::path::Path::new(&self.vault_path).join(&request.path);
        let content = std::fs::read_to_string(&path)
            .map_err(|e| pkm_core::PkmError::NoteNotFound(format!("{}: {}", request.path, e)))?;

        let parsed = pkm_markdown::parser::parse_raw(&content);

        let title = parsed
            .frontmatter
            .title
            .clone()
            .unwrap_or_else(|| {
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("untitled")
                    .to_string()
            });

        let backlinks = Vec::new(); // Would query index engine
        let unlinked_mentions = Vec::new();

        Ok(OpenNoteResponse {
            path: request.path,
            title,
            frontmatter: FrontmatterDto {
                title: parsed.frontmatter.title,
                created: parsed.frontmatter.created,
                modified: parsed.frontmatter.modified,
                tags: parsed.frontmatter.tags.clone(),
                aliases: parsed.frontmatter.aliases.clone(),
            },
            body: parsed.body,
            links: parsed
                .links
                .iter()
                .map(|l| LinkDto {
                    target: l.target.clone(),
                    display_text: l.display_text.clone(),
                    resolved: l.resolved,
                    line: l.line as u32,
                })
                .collect(),
            tags: parsed
                .tags
                .iter()
                .map(|t| TagDto {
                    name: t.name.clone(),
                    source: match t.source {
                        pkm_core::TagSource::Frontmatter => "frontmatter".to_string(),
                        pkm_core::TagSource::Inline => "inline".to_string(),
                    },
                })
                .collect(),
            backlinks,
            unlinked_mentions,
        })
    }

    /// Save a note to disk.
    pub fn save_note(&self, request: SaveNoteRequest) -> PkmResult<SaveNoteResponse> {
        let path = std::path::Path::new(&self.vault_path).join(&request.path);

        let frontmatter = pkm_core::Frontmatter {
            title: request.frontmatter.title,
            created: request.frontmatter.created,
            modified: request.frontmatter.modified,
            tags: request.frontmatter.tags,
            aliases: request.frontmatter.aliases,
            extra: std::collections::HashMap::new(),
        };

        let raw = pkm_markdown::renderer::render(&frontmatter, &request.body);
        std::fs::write(&path, &raw).map_err(pkm_core::PkmError::Io)?;

        Ok(SaveNoteResponse {
            path: request.path,
            success: true,
            error: None,
        })
    }

    /// Search the vault.
    pub fn search(&self, request: SearchRequest) -> PkmResult<Vec<SearchResultDto>> {
        // In production, this would use pkm_index::search::TantivyIndex
        // For now, do a simple file scan
        let vault = std::path::Path::new(&self.vault_path);
        let query_lower = request.query.to_lowercase();
        let mut results = Vec::new();

        if vault.is_dir() {
            for entry in walkdir(vault, 3) {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.extension().map_or(false, |e| e == "md") {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            let body_lower = content.to_lowercase();
                            if body_lower.contains(&query_lower) {
                        let rel_path = path
                                    .strip_prefix(vault)
                                    .unwrap_or(path.as_path())
                                    .display()
                                    .to_string();
                                let title = path
                                    .file_stem()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or("untitled");
                                results.push(SearchResultDto {
                                    path: rel_path,
                                    title: title.to_string(),
                                    snippet: content
                                        .lines()
                                        .find(|l| l.to_lowercase().contains(&query_lower))
                                        .unwrap_or("")
                                        .to_string(),
                                    score: 1.0,
                                    matched_terms: vec![request.query.clone()],
                                });
                            }
                        }
                    }
                }
            }
        }

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(request.limit as usize);
        Ok(results)
    }

    /// Get vault statistics.
    pub fn get_stats(&self) -> PkmResult<VaultStatsDto> {
        let vault = std::path::Path::new(&self.vault_path);
        let mut note_count = 0u32;
        let mut total_size = 0u64;
        let mut tags = std::collections::HashSet::new();
        let mut link_count = 0u32;

        if vault.is_dir() {
            for entry in walkdir(vault, 5) {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.extension().map_or(false, |e| e == "md") {
                        note_count += 1;
                        if let Ok(meta) = path.metadata() {
                            total_size += meta.len();
                        }
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            let parsed =
                                pkm_markdown::parser::parse_raw(&content);
                            for tag in &parsed.tags {
                                tags.insert(tag.name.clone());
                            }
                            link_count += parsed.links.len() as u32;
                        }
                    }
                }
            }
        }

        Ok(VaultStatsDto {
            note_count,
            tag_count: tags.len() as u32,
            link_count,
            total_size_bytes: total_size,
            indexed: false,
        })
    }

    /// Get graph data for visualization.
    pub fn get_graph(&self) -> PkmResult<GraphDto> {
        let vault = std::path::Path::new(&self.vault_path);
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        if vault.is_dir() {
            let mut note_map: std::collections::HashMap<String, Vec<String>> =
                std::collections::HashMap::new();
            let mut note_tags: std::collections::HashMap<String, Vec<String>> =
                std::collections::HashMap::new();

            for entry in walkdir(vault, 5) {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.extension().map_or(false, |e| e == "md") {
                        let _rel_path = path
                            .strip_prefix(vault)
                            .unwrap_or(path.as_path())
                            .display()
                            .to_string();
                        let slug = pkm_core::Note::path_to_slug(path.as_path());
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            let parsed =
                                pkm_markdown::parser::parse_raw(&content);
                            let targets: Vec<String> =
                                parsed.links.iter().map(|l| l.target.clone()).collect();
                            note_map.insert(slug.clone(), targets);
                            note_tags.insert(
                                slug.clone(),
                                parsed.tags.iter().map(|t| t.name.clone()).collect(),
                            );
                        }
                    }
                }
            }

            for (slug, targets) in &note_map {
                let tags = note_tags.get(slug).cloned().unwrap_or_default();
                nodes.push(GraphNodeDto {
                    id: slug.clone(),
                    label: slug.replace('-', " "),
                    slug: slug.clone(),
                    tags,
                    link_count: targets.len() as u32,
                });
                for target in targets {
                    edges.push(GraphEdgeDto {
                        source: slug.clone(),
                        target: target.clone(),
                        label: None,
                    });
                }
            }
        }

        Ok(GraphDto { nodes, edges })
    }
}

/// Simple recursive directory walker (avoids pulling in walkdir dep).
fn walkdir(dir: &std::path::Path, max_depth: u32) -> Vec<std::io::Result<std::fs::DirEntry>> {
    let mut results = Vec::new();
    walkdir_recursive(dir, &mut results, 0, max_depth);
    results
}

fn walkdir_recursive(
    dir: &std::path::Path,
    results: &mut Vec<std::io::Result<std::fs::DirEntry>>,
    depth: u32,
    max_depth: u32,
) {
    if depth > max_depth {
        return;
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries {
            if let Ok(ref e) = entry {
                let path = e.path();
                if path.is_dir() {
                    // Skip .pkm directory
                    if path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .map_or(false, |s| s == ".pkm")
                    {
                        continue;
                    }
                    walkdir_recursive(&path, results, depth + 1, max_depth);
                }
            }
            results.push(entry);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_vault() -> (TempDir, FrontendApi) {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("notes")).unwrap();

        let note1 = "---\ntitle: Note One\ntags: [test]\n---\n\n# Note One\n\nHello from [[Note Two]].\n\n#test";
        let note2 = "---\ntitle: Note Two\ntags: [test, important]\n---\n\n# Note Two\n\nReply to [[Note One]].\n\nSee [[Note Three]] for more.";

        std::fs::write(dir.path().join("notes/note-one.md"), note1).unwrap();
        std::fs::write(dir.path().join("notes/note-two.md"), note2).unwrap();

        let api = FrontendApi::new(dir.path().display().to_string());
        (dir, api)
    }

    #[test]
    fn test_open_note() {
        let (_dir, api) = setup_vault();
        let request = OpenNoteRequest {
            path: "notes/note-one.md".to_string(),
        };
        let response = api.open_note(request).unwrap();
        assert_eq!(response.title, "Note One");
        assert_eq!(response.tags.len(), 2);
        assert_eq!(response.links.len(), 1);
    }

    #[test]
    fn test_open_nonexistent_note() {
        let (_dir, api) = setup_vault();
        let request = OpenNoteRequest {
            path: "nonexistent.md".to_string(),
        };
        let result = api.open_note(request);
        assert!(result.is_err());
    }

    #[test]
    fn test_save_and_reopen_note() {
        let (_dir, api) = setup_vault();
        let save_req = SaveNoteRequest {
            path: "notes/note-one.md".to_string(),
            frontmatter: FrontmatterDto {
                title: Some("Updated Note".to_string()),
                created: Some("2026-06-22".to_string()),
                modified: Some("2026-06-23".to_string()),
                tags: vec!["updated".to_string()],
                aliases: vec![],
            },
            body: "# Updated\n\nNew content.".to_string(),
        };
        let save_resp = api.save_note(save_req).unwrap();
        assert!(save_resp.success);

        let open_req = OpenNoteRequest {
            path: "notes/note-one.md".to_string(),
        };
        let open_resp = api.open_note(open_req).unwrap();
        assert_eq!(open_resp.title, "Updated Note");
        assert!(open_resp.body.contains("New content."));
    }

    #[test]
    fn test_search() {
        let (_dir, api) = setup_vault();
        let request = SearchRequest {
            query: "Hello".to_string(),
            mode: "fulltext".to_string(),
            limit: 10,
        };
        let results = api.search(request).unwrap();
        assert!(!results.is_empty());
        assert!(results.iter().any(|r| r.path.contains("note-one")));
    }

    #[test]
    fn test_stats() {
        let (_dir, api) = setup_vault();
        let stats = api.get_stats().unwrap();
        assert_eq!(stats.note_count, 2);
        assert!(stats.tag_count >= 2); // test + important
        assert!(stats.link_count >= 3); // 2 + 1 links
    }

    #[test]
    fn test_graph() {
        let (_dir, api) = setup_vault();
        let graph = api.get_graph().unwrap();
        assert_eq!(graph.nodes.len(), 2);
        assert!(!graph.edges.is_empty());
    }

    #[test]
    fn test_empty_vault() {
        let dir = TempDir::new().unwrap();
        let api = FrontendApi::new(dir.path().display().to_string());

        let stats = api.get_stats().unwrap();
        assert_eq!(stats.note_count, 0);

        let graph = api.get_graph().unwrap();
        assert!(graph.nodes.is_empty());
        assert!(graph.edges.is_empty());

        let search_req = SearchRequest {
            query: "anything".to_string(),
            mode: "fulltext".to_string(),
            limit: 10,
        };
        let results = api.search(search_req).unwrap();
        assert!(results.is_empty());
    }
}
