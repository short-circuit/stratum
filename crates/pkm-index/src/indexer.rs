use pkm_core::{Backlink, Note, PkmError, PkmResult, ProgressCallback, SearchMode, SearchResult, VaultMeta};
use std::path::{Path, PathBuf};

use crate::graph::Graph;
use crate::rebuild;
use crate::search::TantivyIndex;
use crate::tags::TagAggregator;

/// High-level IndexEngine that coordinates graph + search + tags.
pub struct IndexEngine {
    vault_path: PathBuf,
    search_index: TantivyIndex,
    graph: Graph,
    tags: TagAggregator,
    meta: VaultMeta,
}

impl IndexEngine {
    /// Create a new IndexEngine for a given vault path.
    ///
    /// The index directory is created inside `.pkm/search.idx` relative to the vault path.
    pub fn new(vault_path: &Path) -> PkmResult<Self> {
        let pkm_dir = vault_path.join(".pkm");
        let index_path = pkm_dir.join("search.idx");

        // Ensure .pkm directory exists
        std::fs::create_dir_all(&pkm_dir)
            .map_err(|e| PkmError::Io(e))?;

        let search_index = TantivyIndex::create_index(&index_path)?;

        let mut meta = VaultMeta::new();
        meta.last_indexed = Some(chrono::Utc::now());

        Ok(Self {
            vault_path: vault_path.to_path_buf(),
            search_index,
            graph: Graph::new(),
            tags: TagAggregator::new(),
            meta,
        })
    }

    /// Index a single note (add or update).
    pub fn index_note(&mut self, note: &Note) -> PkmResult<()> {
        // Index in Tantivy
        self.search_index.index_note(note)?;

        // Add/update node in graph
        self.graph.add_node(note);

        // Add edges for each link
        for link in &note.links {
            let target_slug = link.target.replace(' ', "-").to_lowercase();
            if self.graph.get_node(&target_slug).is_some()
                || self.graph.get_node(&link.target).is_some()
            {
                self.graph.add_edge(&note.slug, &link.target, link.display_text.clone());
            }
        }

        // Update tag aggregator
        self.tags.aggregate(&[note.clone()]);

        // Update meta
        self.meta.note_count = self.graph.node_count();
        self.meta.last_indexed = Some(chrono::Utc::now());

        Ok(())
    }

    /// Remove a note from the index by its vault-relative path.
    pub fn remove_note(&mut self, path: &str) -> PkmResult<()> {
        self.search_index.delete_note(path)?;

        // Note: We don't remove from graph/tags on removal since that would
        // require complex graph reconstruction. A full rebuild is recommended
        // after bulk changes. For single note removal, the graph just becomes
        // stale for that note.

        tracing::debug!("Removed {} from search index", path);
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
            &mut self.search_index,
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

    /// Search the index.
    pub fn search(&self, query: &str, mode: SearchMode) -> PkmResult<Vec<SearchResult>> {
        self.search_index.search(query, mode)
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
            format!("---\ntitle: {}\n---\n{}", title, body),
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
        assert!(dir.path().join(".pkm/search.idx").exists());
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
        assert_eq!(engine.meta.note_count, 1);

        let results = engine
            .search("Rust", SearchMode::FullText)
            .unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].title, "Test Note");
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
            Tag { name: "rust".to_string(), source: TagSource::Frontmatter, line: 0 },
            Tag { name: "programming".to_string(), source: TagSource::Inline, line: 1 },
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
        ).unwrap();
        std::fs::write(
            vault_dir.path().join("beta.md"),
            "---\ntitle: Beta\ntags: [letter]\n---\nSecond letter.",
        ).unwrap();

        let mut engine = IndexEngine::new(vault_dir.path()).unwrap();
        let notes = engine.rebuild_all(None).unwrap();

        assert_eq!(notes.len(), 2);
        assert_eq!(engine.meta.note_count, 2);

        // Search should work after rebuild
        let results = engine.search("Alpha", SearchMode::FullText).unwrap();
        assert!(!results.is_empty());
    }

    #[test]
    fn test_remove_note() {
        let dir = TempDir::new().unwrap();
        let mut engine = IndexEngine::new(dir.path()).unwrap();

        let note = make_note("remove-me", "Remove Me", "This will be removed.", vec![]);
        engine.index_note(&note).unwrap();

        let results = engine.search("remove", SearchMode::FullText).unwrap();
        assert!(!results.is_empty());

        engine.remove_note("remove-me.md").unwrap();

        let results = engine.search("remove", SearchMode::FullText).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_find_orphaned_notes() {
        let dir = TempDir::new().unwrap();
        let mut engine = IndexEngine::new(dir.path()).unwrap();

        let connected = make_note("connected", "Connected", "Links to [[other]].", vec!["other"]);
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
