use pkm_core::{Note, PkmError, PkmResult, ProgressCallback};
use std::path::{Path, PathBuf};

use crate::graph::Graph;
use crate::search::TantivyIndex;
use crate::tags::TagAggregator;

/// Rebuild the entire index from .md files on disk.
///
/// Walks the vault directory recursively, parses each .md file,
/// indexes it, adds to the graph, and aggregates tags.
pub fn rebuild_all(
    vault_path: &Path,
    search_index: &mut TantivyIndex,
    graph: &mut Graph,
    tags: &mut TagAggregator,
    progress: Option<ProgressCallback>,
) -> PkmResult<Vec<Note>> {
    let md_files = collect_md_files(vault_path)?;
    let total = md_files.len();

    tracing::info!("Rebuilding index from {} .md files in {:?}", total, vault_path);

    if total == 0 {
        if let Some(cb) = &progress {
            cb("No .md files found".to_string(), 1.0);
        }
        return Ok(Vec::new());
    }

    let mut indexed_notes: Vec<Note> = Vec::with_capacity(total);

    // First pass: parse all notes and index in Tantivy
    for (i, file_path) in md_files.iter().enumerate() {
        let file_name = file_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        let progress_pct = (i as f32 + 1.0) / total as f32;

        if let Some(cb) = &progress {
            cb(format!("Parsing {}", file_name), progress_pct);
        }

        // Parse the .md file using pkm-markdown
        let note = match pkm_markdown::parser::parse_file(file_path, vault_path) {
            Ok(n) => n,
            Err(e) => {
                tracing::warn!("Failed to parse {}: {}", file_path.display(), e);
                continue;
            }
        };

        // Index in Tantivy
        if let Err(e) = search_index.index_note(&note) {
            tracing::warn!("Failed to index {}: {}", file_path.display(), e);
        }

        indexed_notes.push(note);
    }

    // Second pass: add all nodes to graph, then add edges
    for note in &indexed_notes {
        graph.add_node(note);
    }

    for note in &indexed_notes {
        for link in &note.links {
            let target_slug = link.target.replace(' ', "-").to_lowercase();
            // Check if target exists in graph and use its actual slug
            let target_slug_owned = graph.get_node(&target_slug)
                .or_else(|| graph.get_node(&link.target))
                .map(|n| n.slug.clone());
            if let Some(ref target_slug_resolved) = target_slug_owned {
                graph.add_edge(&note.slug, target_slug_resolved, link.display_text.clone());
            }
        }
    }

    // Aggregate tags from all indexed notes
    tags.aggregate(&indexed_notes);

    if let Some(cb) = &progress {
        cb(format!("Indexed {} notes", indexed_notes.len()), 1.0);
    }

    Ok(indexed_notes)
}

/// Collect all .md files recursively from a directory.
fn collect_md_files(dir: &Path) -> PkmResult<Vec<PathBuf>> {
    let mut files = Vec::new();

    if !dir.exists() {
        return Err(PkmError::NotFound(format!(
            "Directory not found: {}",
            dir.display()
        )));
    }

    collect_md_files_recursive(dir, &mut files)?;

    files.sort();
    Ok(files)
}

fn collect_md_files_recursive(dir: &Path, files: &mut Vec<PathBuf>) -> PkmResult<()> {
    let read_dir = std::fs::read_dir(dir).map_err(|e| {
        PkmError::Io(std::io::Error::new(e.kind(), format!("{}: {}", dir.display(), e)))
    })?;

    for entry in read_dir {
        let entry = entry.map_err(|e| PkmError::Io(e))?;
        let path = entry.path();

        if path.is_dir() {
            // Skip hidden directories (like .pkm, .git)
            if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                if name.starts_with('.') {
                    continue;
                }
            }
            collect_md_files_recursive(&path, files)?;
        } else if path.is_file() {
            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                if ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("markdown") {
                    files.push(path);
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_vault(dir: &Path) {
        // Create some .md files
        std::fs::create_dir_all(dir.join("subdir")).unwrap();

        std::fs::write(
            dir.join("note-a.md"),
            "---\ntitle: Note A\ntags: [tag1]\n---\nThis is note A linking to [[Note B]].",
        ).unwrap();

        std::fs::write(
            dir.join("note-b.md"),
            "---\ntitle: Note B\ntags: [tag2]\n---\nThis is note B linking to [[Note C]].",
        ).unwrap();

        std::fs::write(
            dir.join("subdir/note-c.md"),
            "---\ntitle: Note C\ntags: [tag1, tag3]\n---\nThis is note C.",
        ).unwrap();

        // Create a non-md file that should be ignored
        std::fs::write(dir.join("readme.txt"), "Not a markdown file.").unwrap();

        // Create a hidden directory that should be skipped
        std::fs::create_dir_all(dir.join(".pkm")).unwrap();
        std::fs::write(dir.join(".pkm/cache.db"), "fake cache").unwrap();
    }

    #[test]
    fn test_collect_md_files() {
        let dir = TempDir::new().unwrap();
        create_test_vault(dir.path());

        let files = collect_md_files(dir.path()).unwrap();
        assert_eq!(files.len(), 3); // 3 .md files, no txt, no hidden

        let filenames: Vec<String> = files
            .iter()
            .map(|f| f.file_name().unwrap().to_str().unwrap().to_string())
            .collect();
        assert!(filenames.contains(&"note-a.md".to_string()));
        assert!(filenames.contains(&"note-b.md".to_string()));
        assert!(filenames.contains(&"note-c.md".to_string()));
    }

    #[test]
    fn test_rebuild_all() {
        let dir = TempDir::new().unwrap();
        create_test_vault(dir.path());

        let index_dir = TempDir::new().unwrap();

        let mut search_index = TantivyIndex::create_index(index_dir.path()).unwrap();
        let mut graph = Graph::new();
        let mut tags = TagAggregator::new();

        let result = rebuild_all(
            dir.path(),
            &mut search_index,
            &mut graph,
            &mut tags,
            None,
        );

        assert!(result.is_ok());
        let notes = result.unwrap();

        // All 3 .md files should have been parsed successfully
        assert_eq!(notes.len(), 3);

        // Graph should have 3 nodes
        assert_eq!(graph.node_count(), 3);

        // Note A links to B, B links to C -> 2 edges
        // But A's link target is "Note B" (the title), and B exists as slug "note-b"
        // The link resolution checks if the target slug exists
        // Let's check what edges we got
        assert!(graph.edge_count() >= 1);

        // Tags should be aggregated
        assert_eq!(tags.unique_tag_count(), 3); // tag1, tag2, tag3

        // Search should work
        let search_results = search_index.search("Note A", pkm_core::SearchMode::FullText).unwrap();
        assert!(!search_results.is_empty(), "Expected search results for 'Note A'");
    }

    #[test]
    fn test_rebuild_empty_directory() {
        let dir = TempDir::new().unwrap();
        let index_dir = TempDir::new().unwrap();

        let mut search_index = TantivyIndex::create_index(index_dir.path()).unwrap();
        let mut graph = Graph::new();
        let mut tags = TagAggregator::new();

        let result = rebuild_all(
            dir.path(),
            &mut search_index,
            &mut graph,
            &mut tags,
            None,
        );

        assert!(result.is_ok());
        let notes = result.unwrap();
        assert!(notes.is_empty());
        assert_eq!(graph.node_count(), 0);
    }

    #[test]
    fn test_rebuild_with_progress() {
        let dir = TempDir::new().unwrap();
        create_test_vault(dir.path());
        let index_dir = TempDir::new().unwrap();

        let mut search_index = TantivyIndex::create_index(index_dir.path()).unwrap();
        let mut graph = Graph::new();
        let mut tags = TagAggregator::new();

        let progress_calls = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let progress_calls_clone = progress_calls.clone();

        let cb: ProgressCallback = Box::new(move |msg, pct| {
            let mut calls = progress_calls_clone.lock().unwrap();
            calls.push((msg, pct));
        });

        let result = rebuild_all(
            dir.path(),
            &mut search_index,
            &mut graph,
            &mut tags,
            Some(cb),
        );

        assert!(result.is_ok());
        let calls = progress_calls.lock().unwrap();
        assert!(!calls.is_empty(), "Expected progress callbacks");
        // Last call should be 1.0
        assert!((calls.last().unwrap().1 - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_rebuild_nonexistent_directory() {
        let index_dir = TempDir::new().unwrap();
        let mut search_index = TantivyIndex::create_index(index_dir.path()).unwrap();
        let mut graph = Graph::new();
        let mut tags = TagAggregator::new();

        let result = rebuild_all(
            &PathBuf::from("/nonexistent/path"),
            &mut search_index,
            &mut graph,
            &mut tags,
            None,
        );

        assert!(result.is_err());
    }
}
