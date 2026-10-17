//! Pre-built page metadata index.
//!
//! Provides a fast in-memory lookup of note slugs, titles, paths, and tags for
//! the graph commands. Built once per request from the SQLite store to avoid
//! duplicated map-building across graph, connected components, orphan, and
//! link-resolution paths.
//!
//! # Node identity
//!
//! Node identity is the **vault-relative page path** (one node per path). Two
//! notes with the same basename slug in different folders are distinct nodes;
//! the slug remains a display convenience. All resolution maps are keyed by
//! path so duplicates never silently overwrite each other.

use crate::commands::graph::GraphNodeDto;
use std::collections::{HashMap, HashSet};

/// Derive a basename slug from a vault-relative path (e.g. "pages/my-note.md" → "my-note").
pub(crate) fn slug_from_path(path: &str) -> String {
    std::path::Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("untitled")
        .to_string()
}

/// Strip surrounding `[`/`]` characters from a wiki-link target reference.
///
/// Stale `links` rows written by pre-fix versions may store bracket-wrapped
/// targets (`[[Docker]]`) because the old extractor leaked the wrapping into the
/// target. Resolution must tolerate that so a vault upgrades gracefully and its
/// edges appear on the first graph load, before any write/self-heal run.
///
/// This is a belt-and-braces fallback applied before `resolve_path_candidates`;
/// the extraction-level fix in `pkm-markdown::linker` prevents new rows from
/// being written wrapped in the first place.
pub(crate) fn strip_wiki_brackets(target: &str) -> String {
    let mut t = target.trim();
    while t.starts_with('[') {
        t = &t[1..];
    }
    while t.ends_with(']') {
        t = &t[..t.len() - 1];
    }
    t.to_string()
}

/// Pre-built index of all pages in the vault, keyed by vault-relative path.
///
/// Each map uses the path as the primary identity so that duplicate basename
/// slugs across folders are preserved as distinct entries. `slug_to_paths` and
/// `title_to_paths` collect the set of paths that share a given slug/title
/// (never overwriting), which is what makes ambiguous resolution detectable.
pub(crate) struct PageMetaIndex {
    /// sorted vault-relative page paths (one node per path)
    paths: Vec<String>,
    /// path -> basename slug (injective)
    path_to_slug: HashMap<String, String>,
    /// path -> display title
    path_to_title: HashMap<String, String>,
    /// path -> tags
    path_to_tags: HashMap<String, Vec<String>>,
    /// normalized basename slug (lowercased, spaces->-) -> paths sharing it
    slug_to_paths: HashMap<String, Vec<String>>,
    /// lowercase title -> paths sharing it
    title_to_paths: HashMap<String, Vec<String>>,
}

impl PageMetaIndex {
    pub(crate) fn from_store(store: &pkm_block::BlockStore) -> Result<Self, String> {
        let mut paths = store.list_pages().map_err(|e| e.to_string())?;
        let mut path_to_slug = HashMap::new();
        let mut path_to_title = HashMap::new();
        let mut path_to_tags = HashMap::new();
        let mut slug_to_paths: HashMap<String, Vec<String>> = HashMap::new();
        let mut title_to_paths: HashMap<String, Vec<String>> = HashMap::new();

        // Batch-load all page frontmatter in a single query
        let pages = store.get_pages(&paths).map_err(|e| e.to_string())?;

        for path in &paths {
            let slug = slug_from_path(path);
            // Fall back to the presence of the page in the batch; a missing
            // entry (shouldn't happen for listed pages) still yields a title.
            let title = pages
                .get(path)
                .and_then(|f| f.title.clone())
                .unwrap_or_else(|| slug.replace('-', " "));
            let tags = pages.get(path).map(|f| f.tags.clone()).unwrap_or_default();

            path_to_slug.insert(path.clone(), slug.clone());
            path_to_title.insert(path.clone(), title.clone());
            path_to_tags.insert(path.clone(), tags);

            // PUSH into the Vecs — never overwrite — so duplicates are preserved.
            slug_to_paths
                .entry(slug.to_lowercase().replace(' ', "-"))
                .or_default()
                .push(path.clone());
            // Also index under the case-sensitive stored slug for step 5 of
            // `resolve_path_candidates`.
            slug_to_paths
                .entry(slug.clone())
                .or_default()
                .push(path.clone());
            title_to_paths
                .entry(title.to_lowercase())
                .or_default()
                .push(path.clone());
        }

        // Deterministic output ordering for all_paths().
        paths.sort();

        Ok(Self {
            paths,
            path_to_slug,
            path_to_title,
            path_to_tags,
            slug_to_paths,
            title_to_paths,
        })
    }

    pub(crate) fn all_paths(&self) -> impl Iterator<Item = &str> {
        self.paths.iter().map(|s| s.as_str())
    }

    pub(crate) fn slug_for_path(&self, path: &str) -> Option<&str> {
        self.path_to_slug.get(path).map(|s| s.as_str())
    }

    pub(crate) fn title_for_path(&self, path: &str) -> Option<&str> {
        self.path_to_title.get(path).map(|s| s.as_str())
    }

    pub(crate) fn get_node(&self, path: &str) -> GraphNodeDto {
        let degree = 0; // placeholder — caller sets degree
        GraphNodeDto {
            id: path.to_string(),
            title: self.path_to_title.get(path).cloned().unwrap_or_default(),
            path: path.to_string(),
            tags: self.path_to_tags.get(path).cloned().unwrap_or_default(),
            degree,
        }
    }

    /// Resolve a `[[wiki-link]]` target to ALL matching page paths, in
    /// deterministic first-occurrence order. Resolution precedence:
    ///
    /// 1. exact path match: candidate == full path (with `.md`)
    /// 2. exact path minus `.md`: candidate == target with `.md` stripped; also
    ///    try `target + ".md"` when target lacks the extension
    /// 3. suffix path match: any path whose `.md`-stripped form ends with
    ///    `/` + target or `target + ".md"`
    /// 4. normalized basename slug: slugify(target) -> all paths of that basename
    /// 5. exact basename slug (case-sensitive as stored)
    /// 6. title match: lowercase title == lowercase target
    ///
    /// If the target already contains `/`, steps 4-6 are skipped — path-qualified
    /// targets only match paths.
    pub(crate) fn resolve_path_candidates(&self, target: &str) -> Vec<String> {
        let mut raw: Vec<String> = Vec::new();
        let stripped_target = target.strip_suffix(".md").unwrap_or(target);

        // (1) exact path match (with .md)
        for p in &self.paths {
            if p == target {
                raw.push(p.clone());
            }
        }

        // (2) exact path minus .md, or target + ".md"
        for p in &self.paths {
            let p_stripped = p.strip_suffix(".md").unwrap_or(p);
            if p_stripped == target || p == stripped_target {
                raw.push(p.clone());
            }
        }

        // (3) suffix path match
        let suffix_a = format!("/{target}");
        let suffix_b = format!("/{stripped_target}");
        for p in &self.paths {
            let p_stripped = p.strip_suffix(".md").unwrap_or(p);
            if p_stripped.ends_with(&suffix_a) || p_stripped.ends_with(&suffix_b) {
                raw.push(p.clone());
            }
        }

        // (4-6) only for unqualified targets
        if !target.contains('/') {
            let slugified = target.replace(' ', "-").to_lowercase();
            if let Some(paths) = self.slug_to_paths.get(&slugified) {
                raw.extend(paths.iter().cloned());
            }
            if let Some(paths) = self.slug_to_paths.get(target) {
                raw.extend(paths.iter().cloned());
            }
            let lower = target.to_lowercase();
            if let Some(paths) = self.title_to_paths.get(&lower) {
                raw.extend(paths.iter().cloned());
            }
        }

        // Deduplicate preserving first-occurrence order.
        let mut seen: HashSet<String> = HashSet::new();
        let mut result: Vec<String> = Vec::new();
        for p in raw {
            if seen.insert(p.clone()) {
                result.push(p);
            }
        }
        result
    }

    /// Resolve a target to a single path iff exactly one path matches;
    /// otherwise `None` (ambiguous or dead). This is the authoritative
    /// "resolve" used by the graph edge builder and link resolution.
    pub(crate) fn resolve_path(&self, target: &str) -> Option<String> {
        let mut candidates = self.resolve_path_candidates(target);
        if candidates.len() == 1 {
            candidates.pop()
        } else {
            None
        }
    }

    /// Alias of [`resolve_path_candidates`] exposed for the autocomplete API.
    #[allow(dead_code)] // reserved as the public autocomplete entry point
    pub(crate) fn resolve_slug_candidates(&self, target: &str) -> Vec<String> {
        self.resolve_path_candidates(target)
    }
}
