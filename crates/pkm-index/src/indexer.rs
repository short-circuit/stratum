use pkm_core::{
    Backlink, Note, PkmError, PkmResult, ProgressCallback, SearchMode, SearchResult, VaultMeta,
};
use std::path::{Path, PathBuf};

use crate::block_search::BlockIndex;
use crate::graph::Graph;
use crate::rebuild;
use crate::tags::TagAggregator;

/// High-level IndexEngine that coordinates graph + search + tags.
pub struct IndexEngine {
    vault_path: PathBuf,
    block_index: BlockIndex,
    graph: Graph,
    tags: TagAggregator,
    meta: VaultMeta,
}

impl IndexEngine {
    /// Create a new IndexEngine for a given vault path.
    ///
    /// The index directory is created inside `.pkm/search` relative to the vault path.
    pub fn new(vault_path: &Path) -> PkmResult<Self> {
        let pkm_dir = vault_path.join(".pkm");
        let index_path = pkm_dir.join("search");

        // Ensure .pkm directory exists
        std::fs::create_dir_all(&pkm_dir).map_err(PkmError::Io)?;

        let block_index = BlockIndex::create(&index_path)?;

        let mut meta = VaultMeta::new();
        meta.last_indexed = Some(chrono::Utc::now());

        Ok(Self {
            vault_path: vault_path.to_path_buf(),
            block_index,
            graph: Graph::new(),
            tags: TagAggregator::new(),
            meta,
        })
    }

    /// Index a single note (add or update).
    ///
    /// Parses the note's markdown content into blocks and indexes each block
    /// in the block-level Tantivy index.
    pub fn index_note(&mut self, note: &Note) -> PkmResult<()> {
        // Parse note content into blocks and index each block
        let rel_path = note.rel_path.to_string_lossy().to_string();
        let (_fm, _body, blocks) = pkm_markdown::block_parser::parse_document(&note.raw);

        // Delete existing blocks for this page path
        let _ = self.block_index.delete_blocks_by_page(&rel_path);

        for block in &blocks {
            self.block_index.index_block(block, &rel_path)?;
        }

        // Add/update node in graph
        self.graph.add_node(note);

        // Add edges for each link
        for link in &note.links {
            let target_slug = link.target.replace(' ', "-").to_lowercase();
            if self.graph.get_node(&target_slug).is_some()
                || self.graph.get_node(&link.target).is_some()
            {
                self.graph
                    .add_edge(&note.slug, &link.target, link.display_text.clone());
            }
        }

        // Update tag aggregator: decrement old tags, then add new ones
        self.tags.decrement_note(&note.slug);
        self.tags.aggregate(std::slice::from_ref(note));

        // Update meta
        self.meta.note_count = self.graph.node_count();
        self.meta.last_indexed = Some(chrono::Utc::now());

        Ok(())
    }

    /// Explicitly flush the underlying block index, committing all pending writes.
    ///
    /// This is exposed so that callers (e.g., the file watcher or CLI) can control
    /// when a Tantivy commit happens, rather than committing on every single
    /// `index_note` call which causes index fragmentation.
    pub fn flush(&mut self) -> PkmResult<()> {
        self.block_index.flush()
    }

    /// Remove a note from the index by its vault-relative path.
    pub fn remove_note(&mut self, path: &str) -> PkmResult<()> {
        // Derive slug from path (e.g. "subdir/note.md" -> "note")
        let slug = std::path::Path::new(path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();

        // Decrement tag counts for this note
        self.tags.decrement_note(&slug);

        self.block_index.delete_blocks_by_page(path)?;
        self.block_index.flush()?;

        tracing::debug!("Removed {} from block search index", path);
        Ok(())
    }

    /// Re-index a single page from disk by reading the .md file, parsing it,
    /// and updating the index in place.
    ///
    /// If the file does not exist on disk (e.g. it was just deleted), this is a
    /// no-op that returns `Ok(())` — the caller should call `remove_note` for
    /// deletions.
    pub fn refresh_page(&mut self, rel_path: &str, vault_path: &Path) -> PkmResult<()> {
        let full_path = vault_path.join(rel_path);

        // If file doesn't exist, just return Ok — caller handles deletion
        if !full_path.exists() {
            tracing::debug!("refresh_page: file not found, skipping: {}", rel_path);
            return Ok(());
        }

        let content = std::fs::read_to_string(&full_path).map_err(PkmError::Io)?;
        let metadata = std::fs::metadata(&full_path).map_err(PkmError::Io)?;
        let modified_at: chrono::DateTime<chrono::Utc> =
            metadata.modified().map_err(PkmError::Io)?.into();
        let parsed = pkm_markdown::parser::parse_raw(&content);

        let note = Note::new(
            full_path,
            vault_path,
            parsed.frontmatter,
            parsed.body,
            parsed.raw,
            parsed.links,
            parsed.tags,
            modified_at,
        );

        self.index_note(&note)?;
        self.block_index.flush()?;
        Ok(())
    }

    /// Rebuild the entire index from .md files on disk.
    pub fn rebuild_all(&mut self, progress: Option<ProgressCallback>) -> PkmResult<Vec<Note>> {
        // Reset state
        self.graph = Graph::new();
        self.tags = TagAggregator::new();

        // Rebuild everything
        let notes = rebuild::rebuild_all(
            &self.vault_path,
            &mut self.block_index,
            &mut self.graph,
            &mut self.tags,
            progress,
        )?;

        // Update meta
        self.meta.note_count = self.graph.node_count();
        self.meta.last_indexed = Some(chrono::Utc::now());
        self.meta.total_size_bytes = notes.iter().map(|n| n.size_bytes).sum();

        Ok(notes)
    }

    /// Search the block index, converting block-level results to note-level SearchResults.
    pub fn search(&self, query: &str, _mode: SearchMode) -> PkmResult<Vec<SearchResult>> {
        let block_results = self.block_index.search(query, 50)?;

        let results: Vec<SearchResult> = block_results
            .into_iter()
            .map(|r| {
                // Derive a title from the page path
                let title = Path::new(&r.page_path)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .map(|s| s.replace('-', " "))
                    .unwrap_or_default();

                SearchResult {
                    path: r.page_path,
                    title,
                    snippet: r.snippet,
                    score: r.score as f64,
                    matched_terms: vec![query.to_string()],
                }
            })
            .collect();

        Ok(results)
    }

    /// Get backlinks for a note by its slug.
    pub fn get_backlinks(&self, slug: &str) -> Vec<Backlink> {
        self.graph.get_backlinks(slug)
    }

    /// Get the tag cloud.
    pub fn get_tag_cloud(&self) -> Vec<(String, usize)> {
        self.tags.get_tag_cloud()
    }

    /// Get a reference to the graph.
    pub fn get_graph(&self) -> &Graph {
        &self.graph
    }

    /// Get vault metadata.
    pub fn get_meta(&self) -> &VaultMeta {
        &self.meta
    }

    /// Get a mutable reference to vault metadata.
    pub fn meta_mut(&mut self) -> &mut VaultMeta {
        &mut self.meta
    }

    /// Find orphaned notes.
    pub fn find_orphaned_notes(&self) -> Vec<String> {
        self.graph.find_orphaned_notes()
    }

    /// Find unlinked mentions for a note.
    pub fn find_unlinked_mentions(&self, note: &Note) -> Vec<String> {
        self.graph.find_unlinked_mentions(note)
    }

    /// Get connected components of the graph.
    pub fn connected_components(&self) -> Vec<Vec<String>> {
        self.graph.connected_components()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pkm_core::{Frontmatter, Tag, TagSource};
    use tempfile::TempDir;

    fn make_note(slug: &str, title: &str, body: &str, link_targets: Vec<&str>) -> Note {
        let vault_root = PathBuf::from("/vault");
        let path = vault_root.join(format!("{}.md", slug));
        let links = link_targets
            .iter()
            .enumerate()
            .map(|(i, t)| pkm_core::Link {
                target: t.to_string(),
                display_text: None,
                resolved: false,
                line: i + 1,
            })
            .collect();

        Note::new(
            path,
            &vault_root,
            Frontmatter {
                title: Some(title.to_string()),
                ..Default::default()
            },
            body.to_string(),
            format!("---\ntitle: {}\n---\n\n{}", title, body),
            links,
            vec![],
            chrono::Utc::now(),
        )
    }

    #[test]
    fn test_index_engine_new() {
        let dir = TempDir::new().unwrap();
        let engine = IndexEngine::new(dir.path());
        assert!(engine.is_ok());
        let engine = engine.unwrap();
        assert_eq!(engine.meta.note_count, 0);
        assert!(engine.meta.last_indexed.is_some());
        assert!(dir
            .path()
            .join(".pkm/search/blocks")
            .join("meta.json")
            .exists());
    }

    #[test]
    fn test_index_and_search() {
        let dir = TempDir::new().unwrap();
        let mut engine = IndexEngine::new(dir.path()).unwrap();

        let note = make_note(
            "test-note",
            "Test Note",
            "This is a test note about Rust programming.",
            vec![],
        );

        engine.index_note(&note).unwrap();
        engine.flush().unwrap();
        assert_eq!(engine.meta.note_count, 1);

        let results = engine.search("Rust", SearchMode::FullText).unwrap();
        assert!(!results.is_empty());
        assert!(results[0].title.contains("test") || results[0].title.contains("Test"));
    }

    #[test]
    fn test_get_backlinks() {
        let dir = TempDir::new().unwrap();
        let mut engine = IndexEngine::new(dir.path()).unwrap();

        let note_a = make_note("note-a", "Note A", "Links to [[note-b]].", vec!["note-b"]);
        let note_b = make_note("note-b", "Note B", "I am B.", vec![]);

        // Index B first so the graph has it when A's links are processed
        engine.index_note(&note_b).unwrap();
        engine.index_note(&note_a).unwrap();

        let backlinks = engine.get_backlinks("note-b");
        assert_eq!(backlinks.len(), 1);
    }

    #[test]
    fn test_tag_cloud() {
        let dir = TempDir::new().unwrap();
        let mut engine = IndexEngine::new(dir.path()).unwrap();

        let mut note = make_note("tagged-note", "Tagged Note", "Content", vec![]);
        note.tags = vec![
            Tag {
                name: "rust".to_string(),
                source: TagSource::Frontmatter,
                line: 0,
            },
            Tag {
                name: "programming".to_string(),
                source: TagSource::Inline,
                line: 1,
            },
        ];

        engine.index_note(&note).unwrap();
        let cloud = engine.get_tag_cloud();
        assert_eq!(cloud.len(), 2);
    }

    #[test]
    fn test_rebuild_all() {
        let vault_dir = TempDir::new().unwrap();

        // Create test .md files
        std::fs::write(
            vault_dir.path().join("alpha.md"),
            "---\ntitle: Alpha\ntags: [letter]\n---\nFirst letter [[Beta]].",
        )
        .unwrap();
        std::fs::write(
            vault_dir.path().join("beta.md"),
            "---\ntitle: Beta\ntags: [letter]\n---\nSecond letter.",
        )
        .unwrap();

        let mut engine = IndexEngine::new(vault_dir.path()).unwrap();
        let notes = engine.rebuild_all(None).unwrap();

        assert_eq!(notes.len(), 2);
        assert_eq!(engine.meta.note_count, 2);

        // Search should work after rebuild — search for content words in blocks
        let results = engine.search("letter", SearchMode::FullText).unwrap();
        assert!(!results.is_empty(), "Expected results for 'letter'");
    }

    #[test]
    fn test_remove_note() {
        let dir = TempDir::new().unwrap();
        let mut engine = IndexEngine::new(dir.path()).unwrap();

        let note = make_note("remove-me", "Remove Me", "This will be removed.", vec![]);
        engine.index_note(&note).unwrap();
        engine.flush().unwrap();

        let results = engine.search("removed", SearchMode::FullText).unwrap();
        assert!(!results.is_empty());

        engine.remove_note("remove-me.md").unwrap();

        let results = engine.search("remove", SearchMode::FullText).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_find_orphaned_notes() {
        let dir = TempDir::new().unwrap();
        let mut engine = IndexEngine::new(dir.path()).unwrap();

        let connected = make_note(
            "connected",
            "Connected",
            "Links to [[other]].",
            vec!["other"],
        );
        let other = make_note("other", "Other", "I am linked.", vec![]);
        let orphan = make_note("orphan", "Orphan", "All alone.", vec![]);

        // Index 'other' first so the edge from 'connected' resolves
        engine.index_note(&other).unwrap();
        engine.index_note(&connected).unwrap();
        engine.index_note(&orphan).unwrap();

        let orphans = engine.find_orphaned_notes();
        assert!(orphans.contains(&"orphan".to_string()));
        assert!(!orphans.contains(&"connected".to_string()));
    }
}
