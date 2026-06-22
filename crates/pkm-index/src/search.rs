use pkm_core::{PkmError, PkmResult, SearchMode, SearchResult};
use tantivy::collector::TopDocs;
use tantivy::query::{QueryParser, RegexQuery};
use tantivy::schema::*;
use tantivy::{doc, Index, IndexWriter, ReloadPolicy, TantivyDocument};
use std::path::Path;
use std::sync::Arc;
use tracing::info;

/// Wrapper around a Tantivy search index.
pub struct TantivyIndex {
    index: Index,
    schema: Arc<Schema>,
    writer: Option<IndexWriter>,
}

impl TantivyIndex {
    /// Create (or open) a Tantivy index at the given directory path.
    pub fn create_index(path: &Path) -> PkmResult<Self> {
        let mut schema_builder = Schema::builder();

        schema_builder.add_text_field("title", TEXT | STORED);
        schema_builder.add_text_field("body", TEXT);
        schema_builder.add_text_field("tags", TEXT);
        schema_builder.add_text_field("path", STRING | STORED);
        schema_builder.add_text_field("slug", STRING | STORED);

        let schema = Arc::new(schema_builder.build());

        let index = if Self::index_exists(path) {
            info!("Opening existing Tantivy index at {:?}", path);
            Index::open_in_dir(path)
                .map_err(|e| PkmError::Index(format!("Failed to open index: {}", e)))?
        } else {
            info!("Creating new Tantivy index at {:?}", path);
            std::fs::create_dir_all(path).map_err(|e| PkmError::Io(e))?;
            Index::create_in_dir(path, (*schema).clone())
                .map_err(|e| PkmError::Index(format!("Failed to create index: {}", e)))?
        };

        let writer = index
            .writer(50_000_000)
            .map_err(|e| PkmError::Index(format!("Failed to create writer: {}", e)))?;

        Ok(Self {
            index,
            schema,
            writer: Some(writer),
        })
    }

    /// Check if a Tantivy index exists in the given directory.
    fn index_exists(path: &Path) -> bool {
        path.join("meta.json").exists()
    }

    /// Get the schema fields by name.
    fn field(&self, name: &str) -> Field {
        self.schema
            .get_field(name)
            .unwrap_or_else(|_| panic!("Field '{}' not found in schema", name))
    }

    /// Index a note.
    pub fn index_note(&mut self, note: &pkm_core::Note) -> PkmResult<()> {
        let title = note.derive_title();
        let tags_str = note
            .tags
            .iter()
            .map(|t| t.name.clone())
            .collect::<Vec<_>>()
            .join(" ");
        let path = note.rel_path.to_string_lossy().to_string();

        let title_field = self.field("title");
        let body_field = self.field("body");
        let tags_field = self.field("tags");
        let path_field = self.field("path");
        let slug_field = self.field("slug");

        // Delete existing doc for this path first
        self.delete_note(&path)?;

        let document = doc!(
            title_field => title,
            body_field => note.body.clone(),
            tags_field => tags_str,
            path_field => path.clone(),
            slug_field => note.slug.clone(),
        );

        let writer = self
            .writer
            .as_mut()
            .ok_or_else(|| PkmError::Index("Writer not available".to_string()))?;

        writer
            .add_document(document)
            .map_err(|e| PkmError::Index(format!("Failed to add document: {}", e)))?;

        // Commit periodically
        writer
            .commit()
            .map_err(|e| PkmError::Index(format!("Failed to commit: {}", e)))?;

        Ok(())
    }

    /// Search the index.
    pub fn search(&self, query: &str, mode: SearchMode) -> PkmResult<Vec<SearchResult>> {
        let reader = self
            .index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .map_err(|e| PkmError::Search(format!("Failed to create reader: {}", e)))?;

        let searcher = reader.searcher();

        match mode {
            SearchMode::FullText | SearchMode::Semantic => {
                // Full-text search across title, body, and tags
                let title_field = self.field("title");
                let body_field = self.field("body");
                let tags_field = self.field("tags");

                let query_parser = QueryParser::for_index(
                    &self.index,
                    vec![title_field, body_field, tags_field],
                );

                let tantivy_query = query_parser
                    .parse_query(query)
                    .map_err(|e| PkmError::Search(format!("Query parse error: {}", e)))?;

                let top_docs = searcher
                    .search(&tantivy_query, &TopDocs::with_limit(50))
                    .map_err(|e| PkmError::Search(format!("Search error: {}", e)))?;

                let mut results = Vec::new();
                for (score, doc_addr) in top_docs {
                    let doc: TantivyDocument = searcher
                        .doc::<TantivyDocument>(doc_addr)
                        .map_err(|e| PkmError::Search(format!("Doc retrieval error: {}", e)))?;

                    let path_val = doc
                        .get_first(self.field("path"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let title_val = doc
                        .get_first(self.field("title"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let body_val = doc
                        .get_first(self.field("body"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    // Generate a snippet from the body
                    let snippet = Self::make_snippet(&body_val, query, 100);

                    results.push(SearchResult {
                        path: path_val,
                        title: title_val,
                        snippet,
                        score: score as f64,
                        matched_terms: vec![query.to_string()],
                    });
                }

                Ok(results)
            }
            SearchMode::Regex => {
                // Regex-based filtering
                let body_field = self.field("body");
                let _title_field = self.field("title");

                let regex_pattern = format!("(?i){}", regex::escape(query));
                let regex_query =
                    RegexQuery::from_pattern(&regex_pattern, body_field)
                        .map_err(|e| PkmError::Search(format!("Regex error: {}", e)))?;

                let top_docs = searcher
                    .search(&regex_query, &TopDocs::with_limit(50))
                    .map_err(|e| PkmError::Search(format!("Search error: {}", e)))?;

                let mut results = Vec::new();
                for (score, doc_addr) in top_docs {
                    let doc: TantivyDocument = searcher
                        .doc::<TantivyDocument>(doc_addr)
                        .map_err(|e| PkmError::Search(format!("Doc retrieval error: {}", e)))?;

                    let path_val = doc
                        .get_first(self.field("path"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let title_val = doc
                        .get_first(self.field("title"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let body_val = doc
                        .get_first(self.field("body"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    let snippet = Self::make_snippet(&body_val, query, 100);

                    results.push(SearchResult {
                        path: path_val,
                        title: title_val,
                        snippet,
                        score: score as f64,
                        matched_terms: vec![query.to_string()],
                    });
                }

                Ok(results)
            }
            SearchMode::Graph => {
                // Graph mode: just return empty, as graph search is handled by the graph module
                Ok(Vec::new())
            }
        }
    }

    /// Delete a note from the index by its path.
    pub fn delete_note(&mut self, path: &str) -> PkmResult<()> {
        let path_field = self.field("path");
        let writer = self
            .writer
            .as_mut()
            .ok_or_else(|| PkmError::Index("Writer not available".to_string()))?;

        let term = tantivy::Term::from_field_text(path_field, path);
        writer.delete_term(term);

        writer
            .commit()
            .map_err(|e| PkmError::Index(format!("Failed to commit: {}", e)))?;

        Ok(())
    }

    /// Create a text snippet around the first match of query.
    fn make_snippet(body: &str, query: &str, max_len: usize) -> String {
        let lower_body = body.to_lowercase();
        let lower_query = query.to_lowercase();

        if let Some(pos) = lower_body.find(&lower_query) {
            let start = pos.saturating_sub(max_len / 4);
            let end = (pos + query.len() + max_len / 2).min(body.len());

            let mut snippet = String::new();
            if start > 0 {
                snippet.push_str("...");
            }
            snippet.push_str(&body[start..end]);
            if end < body.len() {
                snippet.push_str("...");
            }

            // Clean up newlines in snippet
            snippet.truncate(max_len + 6); // account for ... overhead
            snippet = snippet.replace('\n', " ");
            snippet
        } else {
            // No match found, return truncated beginning
            let truncated: String = body.chars().take(max_len).collect();
            if body.len() > max_len {
                format!("{}...", truncated)
            } else {
                truncated
            }
        }
    }
}

impl Drop for TantivyIndex {
    fn drop(&mut self) {
        if let Some(writer) = self.writer.take() {
            let _ = writer.wait_merging_threads();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pkm_core::{Frontmatter, Note, Tag, TagSource};
    use std::path::PathBuf;

    fn create_test_index(tmp_dir: &Path) -> TantivyIndex {
        TantivyIndex::create_index(tmp_dir).unwrap()
    }

    fn make_note(path: &str, title: &str, _slug: &str, body: &str, tags: Vec<&str>) -> Note {
        let path = PathBuf::from(path);
        let vault_root = PathBuf::from("/vault");
        let pkm_tags: Vec<Tag> = tags
            .iter()
            .map(|t| Tag {
                name: t.to_string(),
                source: TagSource::Frontmatter,
                line: 0,
            })
            .collect();

        Note::new(
            path.clone(),
            &vault_root,
            Frontmatter {
                title: Some(title.to_string()),
                tags: tags.iter().map(|s| s.to_string()).collect(),
                ..Default::default()
            },
            body.to_string(),
            format!("---\ntitle: {}\ntags: {:?}\n---\n{}", title, tags, body),
            vec![],
            pkm_tags,
            chrono::Utc::now(),
        )
    }

    #[test]
    fn test_create_index() {
        let dir = tempfile::TempDir::new().unwrap();
        let mut index = create_test_index(dir.path());
        let note = make_note(
            "/vault/test.md",
            "Test Note",
            "test",
            "This is a test note about quantum computing.",
            vec!["quantum"],
        );
        index.index_note(&note).unwrap();
        assert!(dir.path().exists());
    }

    #[test]
    fn test_search_found() {
        let dir = tempfile::TempDir::new().unwrap();
        let mut index = create_test_index(dir.path());
        let note = make_note(
            "/vault/quantum.md",
            "Quantum Computing",
            "quantum",
            "Quantum computing uses qubits for computation.",
            vec!["physics", "computing"],
        );
        index.index_note(&note).unwrap();

        // Need a small delay for index to be searchable
        let results = index.search("quantum", SearchMode::FullText).unwrap();
        assert!(!results.is_empty(), "Expected at least one result for 'quantum'");
        assert_eq!(results[0].title, "Quantum Computing");
    }

    #[test]
    fn test_search_not_found() {
        let dir = tempfile::TempDir::new().unwrap();
        let mut index = create_test_index(dir.path());
        let note = make_note(
            "/vault/quantum.md",
            "Quantum Computing",
            "quantum",
            "Quantum computing uses qubits.",
            vec!["physics"],
        );
        index.index_note(&note).unwrap();

        let results = index.search("nonexistent_term_xyz", SearchMode::FullText).unwrap();
        assert!(results.is_empty(), "Expected no results for nonexistent term");
    }

    #[test]
    fn test_delete_note() {
        let dir = tempfile::TempDir::new().unwrap();
        let mut index = create_test_index(dir.path());
        let note = make_note(
            "/vault/delete-me.md",
            "Delete Me",
            "delete-me",
            "This note will be deleted.",
            vec![],
        );
        index.index_note(&note).unwrap();

        // Search before delete
        let results = index.search("delete", SearchMode::FullText).unwrap();
        assert!(!results.is_empty());

        // Delete
        index.delete_note("delete-me.md").unwrap();

        // Search after delete
        let results = index.search("delete", SearchMode::FullText).unwrap();
        assert!(results.is_empty(), "Expected no results after deletion");
    }

    #[test]
    fn test_regex_search() {
        let dir = tempfile::TempDir::new().unwrap();
        let mut index = create_test_index(dir.path());
        let note = make_note(
            "/vault/regex-test.md",
            "Regex Test",
            "regex-test",
            "The quick brown fox jumps over the lazy dog.",
            vec![],
        );
        index.index_note(&note).unwrap();

        let results = index.search("fox", SearchMode::Regex).unwrap();
        assert!(!results.is_empty(), "Expected regex match for 'fox'");
    }

    #[test]
    fn test_make_snippet() {
        let body = "This is a very long body text that contains the search term quantum computing somewhere in the middle of it.";
        let snippet = TantivyIndex::make_snippet(body, "quantum computing", 60);
        assert!(snippet.contains("quantum computing"));
        assert!(snippet.len() <= body.len() + 6);
    }
}
