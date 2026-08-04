//! Enumerate existing vault tags for auto-tagging the transcript.

use pkm_block::BlockStore;
use pkm_index::tags::TagAggregator;
use pkm_markdown::parse_file;
use std::collections::HashSet;
use std::path::Path;

/// All distinct tag names currently used anywhere in the vault
/// (frontmatter + inline), lowercased, sorted.
pub fn existing_tags(store: &BlockStore, vault_path: &Path) -> Vec<String> {
    let mut aggregator = TagAggregator::new();
    let mut notes = Vec::new();
    if let Ok(pages) = store.list_pages() {
        for rel in pages {
            if let Ok(note) = parse_file(&vault_path.join(&rel), vault_path) {
                notes.push(note);
            }
        }
    }
    aggregator.aggregate(&notes);
    let mut tags: HashSet<String> = aggregator.all_tags().keys().cloned().collect();
    tags.remove("");
    let mut sorted: Vec<String> = tags.into_iter().collect();
    sorted.sort();
    sorted
}

#[cfg(test)]
mod tests {
    use super::*;
    use pkm_block::BlockStore;
    use tempfile::tempdir;

    fn write(path: &std::path::Path, content: &str) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, content).unwrap();
    }

    #[test]
    fn test_seeded_store() {
        let dir = tempdir().unwrap();
        let vault = dir.path();
        write(
            &vault.join("pages/one.md"),
            "This is about #rust and #homelab stuff.\n",
        );
        write(
            &vault.join("pages/two.md"),
            "---\ntags: [project, rust]\n---\nBody.\n",
        );
        write(
            &vault.join("pages/three.md"),
            "# Heading with words\n\nplain\n",
        );

        std::fs::create_dir_all(vault.join(".pkm")).unwrap();
        let store = BlockStore::open(&vault.join(".pkm/blocks.db")).unwrap();
        for rel in ["pages/one.md", "pages/two.md", "pages/three.md"] {
            let block = pkm_block::Block::new(uuid::Uuid::new_v4(), rel.to_string());
            store.insert_block(&block, rel).unwrap();
            let page = pkm_block::Page {
                path: vault.join(rel),
                rel_path: rel.into(),
                slug: std::path::Path::new(rel)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("page")
                    .to_string(),
                frontmatter: pkm_block::PageFrontmatter::default(),
                block_tree: pkm_block::tree::BlockTree::default(),
                block_order: Vec::new(),
                size_bytes: 0,
                modified_at: chrono::Utc::now(),
            };
            store.upsert_page(&page).unwrap();
        }

        let tags = existing_tags(&store, vault);
        assert!(tags.contains(&"rust".to_string()), "got {tags:?}");
        assert!(tags.contains(&"homelab".to_string()), "got {tags:?}");
        assert!(tags.contains(&"project".to_string()), "got {tags:?}");
        assert!(
            !tags.iter().any(|t| t == "heading"),
            "heading must not be a tag: {tags:?}"
        );
    }
}
