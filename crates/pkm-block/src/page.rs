use crate::block::BlockId;
use crate::tree::BlockTree;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// A page (file) containing a tree of blocks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    /// Absolute path to the .md file.
    pub path: PathBuf,
    /// Vault-relative path.
    pub rel_path: PathBuf,
    /// File name without extension (slug).
    pub slug: String,
    /// YAML frontmatter metadata.
    pub frontmatter: PageFrontmatter,
    /// The block tree for this page.
    pub block_tree: BlockTree,
    /// Block IDs in document order (depth-first).
    pub block_order: Vec<BlockId>,
    /// File size in bytes.
    pub size_bytes: u64,
    /// Last modified timestamp.
    pub modified_at: DateTime<Utc>,
}

/// YAML frontmatter for a page.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct PageFrontmatter {
    pub title: Option<String>,
    pub created: Option<String>,
    pub modified: Option<String>,
    pub tags: Vec<String>,
    pub aliases: Vec<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_yaml::Value>,
}

impl Page {
    pub fn new(path: PathBuf, vault_root: &std::path::Path) -> Self {
        let rel_path = path
            .strip_prefix(vault_root)
            .unwrap_or(&path)
            .to_path_buf();
        let slug = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("untitled")
            .to_string();

        Self {
            path,
            rel_path,
            slug,
            frontmatter: PageFrontmatter::default(),
            block_tree: BlockTree::new(),
            block_order: Vec::new(),
            size_bytes: 0,
            modified_at: Utc::now(),
        }
    }

    pub fn display_name(&self) -> String {
        self.frontmatter
            .title
            .clone()
            .unwrap_or_else(|| self.slug.replace('-', " "))
    }

    pub fn block_count(&self) -> usize {
        self.block_tree.len()
    }

    pub fn root_blocks(&self) -> Vec<&crate::block::Block> {
        self.block_tree.roots()
    }

    pub fn is_journal(&self) -> bool {
        let path_str = self.rel_path.to_string_lossy();
        path_str.starts_with("journals/") || path_str.starts_with("journals\\")
    }

    pub fn is_page(&self) -> bool {
        let path_str = self.rel_path.to_string_lossy();
        path_str.starts_with("pages/") || path_str.starts_with("pages\\")
    }

    /// Rebuild block_order from depth-first traversal.
    pub fn rebuild_order(&mut self) {
        self.block_order = self
            .block_tree
            .depth_first_order()
            .iter()
            .map(|b| b.id)
            .collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_new() {
        let page = Page::new(
            PathBuf::from("/vault/pages/my-note.md"),
            std::path::Path::new("/vault"),
        );
        assert_eq!(page.slug, "my-note");
        assert_eq!(page.rel_path, PathBuf::from("pages/my-note.md"));
        assert_eq!(page.display_name(), "my note");
        assert_eq!(page.block_count(), 0);
        assert!(page.is_page());
        assert!(!page.is_journal());
    }

    #[test]
    fn test_page_journal() {
        let page = Page::new(
            PathBuf::from("/vault/journals/2026-06-22.md"),
            std::path::Path::new("/vault"),
        );
        assert!(page.is_journal());
        assert!(!page.is_page());
    }

    #[test]
    fn test_page_display_name_with_title() {
        let mut page = Page::new(
            PathBuf::from("/vault/pages/quantum.md"),
            std::path::Path::new("/vault"),
        );
        page.frontmatter.title = Some("Quantum Computing".into());
        assert_eq!(page.display_name(), "Quantum Computing");
    }

    #[test]
    fn test_page_frontmatter_default() {
        let fm = PageFrontmatter::default();
        assert!(fm.title.is_none());
        assert!(fm.tags.is_empty());
        assert!(fm.aliases.is_empty());
        assert!(fm.extra.is_empty());
    }
}
