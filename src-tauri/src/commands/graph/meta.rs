//! Pre-built page metadata index.
//!
//! Provides a fast in-memory lookup of note slugs, titles, paths, and tags for
//! the graph commands. Built once per request from the SQLite store to avoid
//! duplicated map-building across graph, connected components, orphan, and
//! link-resolution paths.

use crate::commands::graph::GraphNodeDto;
use std::collections::HashMap;

/// Derive a note slug from a vault-relative path (e.g. "pages/my-note.md" → "my-note").
pub(crate) fn slug_from_path(path: &str) -> String {
    std::path::Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("untitled")
        .to_string()
}

/// Pre-built index of all pages in the vault for fast slug/title/path resolution.
/// Eliminates duplicated map-building across graph, connected components, orphans, and link resolution.
pub(crate) struct PageMetaIndex {
    pub(crate) slug_to_path: HashMap<String, String>,
    pub(crate) slug_to_title: HashMap<String, String>,
    pub(crate) slug_to_tags: HashMap<String, Vec<String>>,
    title_to_slug: HashMap<String, String>,
}

impl PageMetaIndex {
    pub(crate) fn from_store(store: &pkm_block::BlockStore) -> Result<Self, String> {
        let paths = store.list_pages().map_err(|e| e.to_string())?;
        let mut slug_to_path = HashMap::new();
        let mut slug_to_title = HashMap::new();
        let mut slug_to_tags = HashMap::new();
        let mut title_to_slug = HashMap::new();

        // Batch-load all page frontmatter in a single query
        let pages = store.get_pages(&paths).map_err(|e| e.to_string())?;

        for path in &paths {
            let slug = slug_from_path(path);
            slug_to_path.insert(slug.clone(), path.clone());

            let title = pages
                .get(path)
                .and_then(|f| f.title.clone())
                .unwrap_or_else(|| slug.replace('-', " "));
            slug_to_title.insert(slug.clone(), title.clone());
            title_to_slug.insert(title.to_lowercase(), slug.clone());

            let tags = pages.get(path).map(|f| f.tags.clone()).unwrap_or_default();
            slug_to_tags.insert(slug, tags);
        }

        Ok(Self {
            slug_to_path,
            slug_to_title,
            slug_to_tags,
            title_to_slug,
        })
    }

    pub(crate) fn resolve_slug(&self, target: &str) -> Option<String> {
        let slugified = target.replace(' ', "-").to_lowercase();
        if self.slug_to_path.contains_key(&slugified) {
            return Some(slugified);
        }
        if self.slug_to_path.contains_key(target) {
            return Some(target.to_string());
        }
        let lower = target.to_lowercase();
        if let Some(slug) = self.title_to_slug.get(&lower) {
            return Some(slug.clone());
        }
        if self.slug_to_path.contains_key(&lower) {
            return Some(lower);
        }
        None
    }

    pub(crate) fn get_node(&self, slug: &str) -> GraphNodeDto {
        let degree = 0; // placeholder — caller sets degree
        GraphNodeDto {
            id: slug.to_string(),
            title: self.slug_to_title.get(slug).cloned().unwrap_or_default(),
            path: self.slug_to_path.get(slug).cloned().unwrap_or_default(),
            tags: self.slug_to_tags.get(slug).cloned().unwrap_or_default(),
            degree,
        }
    }

    pub(crate) fn all_slugs(&self) -> impl Iterator<Item = &str> {
        self.slug_to_path.keys().map(|s| s.as_str())
    }

    #[allow(dead_code)]
    pub(crate) fn is_orphan(&self, slug: &str) -> bool {
        !self.slug_to_path.contains_key(slug)
    }
}
