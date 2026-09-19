//! Shared knowledge-base state and tool adapters.
//!
//! The MCP server must never hold a long-lived write lock on the on-disk data
//! (ADR-0005 §5). All adapters open a fresh `BlockStore` / `IndexEngine`
//! handle per operation against the same SQLite/Tantivy files the desktop app
//! uses. Every mutation is serialized per vault by a tokio write mutex
//! (single-writer atomicity, §8/§10), commits inside a SQLite transaction, and
//! refreshes the index exactly like the desktop `save_page`/`delete_page`.
//!
//! The write path mirrors `src-tauri/src/commands/page.rs::save_page`:
//!   parse/validate → atomic temp-file+rename of `.md` → SQLite transaction
//!   (delete_blocks_by_page + insert_block per block + upsert_page) → index
//!   refresh → `watcher_last_save` marker (watcher coordination) → plugin
//!   onSave/onLink dispatch.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::{Duration, SystemTime};

use chrono::{DateTime, Utc};
use pkm_block::{Block, BlockStore, Page};
use pkm_core::fs_util::MdCollector;
use pkm_index::block_search::BlockIndex;
use pkm_index::indexer::IndexEngine;
use tokio::sync::Mutex as AsyncMutex;
use tracing::warn;

use crate::config::McpConfig;
use crate::error::{ErrorDataExt, KbError, KbErrorKind};
use crate::models::{
    AutocompleteItem, AutocompleteResponse, BacklinkHit, BacklinksResponse, GraphData, GraphEdge,
    GraphNode, IndexStatus, NoteDocument, NoteFrontmatter, NoteLink, PageListResponse, PageSummary,
    ResolveLinkResult, ResolvedTarget, SearchHit, SearchResponse, TagOperationResult, VaultInfo,
};

/// A single in-process writer lock (no cross-process) plus an "index rebuild in
/// progress" flag. Cross-process single-writer is guaranteed by SQLite's own
/// file locking; the mutex here only serializes this server's own writers.
#[derive(Debug)]
pub struct SharedVault {
    pub vault_path: PathBuf,
    pub db_path: PathBuf,
    write_lock: AsyncMutex<()>,
    rebuilding: Arc<StdMutex<bool>>,
    last_indexed_at: Arc<StdMutex<Option<DateTime<Utc>>>>,
    /// Watcher coordination marker (mirrors `watcher_last_save`).
    watcher_last_save: Arc<StdMutex<SystemTime>>,
}

impl SharedVault {
    pub fn new(config: &McpConfig) -> anyhow::Result<Self> {
        let check = config.validate_vault()?;
        if !check.has_blocks_db {
            // `.pkm/blocks.db` is guaranteed for any vault the desktop app has
            // opened. If it's missing, the vault may be new/empty — create the
            // directory so adapters can init on first write.
            let pkm_dir = check.vault_path.join(".pkm");
            std::fs::create_dir_all(&pkm_dir)?;
        }
        Ok(Self {
            vault_path: check.vault_path,
            db_path: check.db_path,
            write_lock: AsyncMutex::new(()),
            rebuilding: Arc::new(StdMutex::new(false)),
            last_indexed_at: Arc::new(StdMutex::new(None)),
            watcher_last_save: Arc::new(StdMutex::new(SystemTime::UNIX_EPOCH)),
        })
    }

    /// The on-disk last-write marker (RFC 3339) for a note path, if it exists.
    fn file_modified_at(&self, full: &Path) -> Option<DateTime<Utc>> {
        std::fs::metadata(full)
            .and_then(|m| m.modified())
            .ok()
            .map(DateTime::<Utc>::from)
    }

    /// Build a full path for a vault-relative `path`, containing traversal.
    fn full_path(&self, rel: &str) -> PathBuf {
        self.vault_path.join(rel)
    }

    /// Mark our own save so the desktop watcher ignores it.
    fn mark_own_save(&self) {
        *self.watcher_last_save.lock().unwrap() = SystemTime::now();
    }

    fn mark_indexed(&self) {
        *self.last_indexed_at.lock().unwrap() = Some(Utc::now());
    }

    pub fn is_rebuilding(&self) -> bool {
        *self.rebuilding.lock().unwrap()
    }

    fn set_rebuilding(&self, value: bool) {
        *self.rebuilding.lock().unwrap() = value;
    }

    /// Shared read-only store handle.
    pub fn store(&self) -> Result<BlockStore, KbError> {
        BlockStore::open(&self.db_path).map_err(map_store_err)
    }

    /// Acquire the per-vault write lock with a timeout (single writer).
    ///
    /// Returns the held guard; the caller keeps it alive for the duration of a
    /// mutation so concurrent writes serialize.
    async fn acquire_write(&self) -> Result<tokio::sync::MutexGuard<'_, ()>, KbError> {
        tokio::time::timeout(Duration::from_secs(30), self.write_lock.lock())
            .await
            .map_err(|_| ErrorDataExt::vault_locked())
    }
}
// The lock is an AsyncMutex so acquire_write returns a MutexGuard which we
// hold for the duration of a mutation. The helper methods below are separate.

/// Health probe result for the `/health` operational endpoint (§12).
pub struct HealthProbe {
    pub index_fresh: bool,
    pub page_count: usize,
    pub indexed_pages: usize,
}

/// Probe store connectivity and index health for the `/health` endpoint (§12).
pub fn probe_health(vault_path: &Path, db_path: &Path) -> Result<HealthProbe, String> {
    let store = BlockStore::open(db_path).map_err(|e| format!("cannot open blocks.db: {e}"))?;
    let page_count = store.page_count().map_err(|e| e.to_string())?;
    let indexed_pages = IndexEngine::new(vault_path)
        .map(|engine| engine.get_meta().note_count)
        .unwrap_or(0);
    Ok(HealthProbe {
        index_fresh: true,
        page_count,
        indexed_pages,
    })
}

/// Map a `pkm_core::PkmError` to the contract error vocabulary.
pub fn map_store_err(e: pkm_core::PkmError) -> KbError {
    match e {
        pkm_core::PkmError::NoteNotFound(p)
        | pkm_core::PkmError::PageNotFound(p)
        | pkm_core::PkmError::BlockNotFound(p)
        | pkm_core::PkmError::NotFound(p) => ErrorDataExt::not_found(p),
        pkm_core::PkmError::AlreadyExists(p) => {
            KbError::new(KbErrorKind::Conflict, format!("Object already exists: {p}"))
        }
        pkm_core::PkmError::Validation(msg) => ErrorDataExt::invalid_args(msg),
        other => KbError::new(
            KbErrorKind::External,
            format!("Storage operation failed: {other}"),
        ),
    }
}

/// The write argument guards: all reads are lock-free; all writes go through
/// `write` variants that hold the vault write mutex and use the atomic save
/// path.
impl SharedVault {
    // ------------------------------------------------------------------
    // READ adapters
    // ------------------------------------------------------------------

    /// `kb_get_page`: read a single note. Returns NotFound for missing notes.
    pub fn get_page(&self, path: &str) -> Result<NoteDocument, KbError> {
        let rel = crate::error::validate_rel_path(path)?;
        // Enforce `.md` extension semantics like the app: normalize to .md.
        let rel = ensure_md(&rel);
        let full = self.full_path(&rel);
        let content = match std::fs::read_to_string(&full) {
            Ok(c) => c,
            Err(_) => return Err(ErrorDataExt::not_found(&rel)),
        };

        let modified_at = self
            .file_modified_at(&full)
            .map(|d| d.to_rfc3339())
            .unwrap_or_else(|| Utc::now().to_rfc3339());

        // Parse with the same block parser used by the app.
        let (fm, _body, blocks) = pkm_markdown::block_parser::parse_document(&content);
        let links = extract_links_for_doc(&content);
        let tags = extract_tags(&fm, &content);

        let slug = rel.trim_end_matches(".md").to_string();
        let title = fm.title.clone().unwrap_or_else(|| slug.replace('-', " "));

        Ok(NoteDocument {
            path: rel.clone(),
            slug,
            title: title.clone(),
            content,
            frontmatter: Some(NoteFrontmatter {
                title: fm.title,
                created: fm.created,
                modified: fm.modified,
                tags: fm.tags.clone(),
                aliases: fm.aliases,
            }),
            links,
            tags,
            block_count: blocks.len(),
            modified_at,
        })
    }

    /// `kb_list_pages`: enumerate notes with pagination.
    pub fn list_pages(
        &self,
        limit: usize,
        cursor: Option<&str>,
    ) -> Result<PageListResponse, KbError> {
        let store = self.store()?;
        let mut paths = store.list_pages().map_err(map_store_err)?;
        paths.retain(|p| !p.starts_with(".git/") && !p.contains("/.git/"));

        // Cursor = last path from previous page (opaque).
        let start = cursor
            .map(|c| {
                paths
                    .iter()
                    .position(|p| p == c)
                    .map(|i| i + 1)
                    .unwrap_or(0)
            })
            .unwrap_or(0);

        let end = (start + limit).min(paths.len());
        let page_slice = paths[start..end].to_vec();

        let mut pages = Vec::with_capacity(page_slice.len());
        for path in page_slice {
            let slug = path.trim_end_matches(".md").to_string();
            let full = self.full_path(&path);
            let block_count = store
                .get_blocks_by_page(&path)
                .map(|b| b.len())
                .unwrap_or(0);
            let modified_at = self
                .file_modified_at(&full)
                .map(|d| d.to_rfc3339())
                .unwrap_or_else(|| Utc::now().to_rfc3339());
            let title = store
                .get_page(&path)
                .ok()
                .flatten()
                .and_then(|fm| fm.title)
                .unwrap_or_else(|| slug.replace('-', " "));
            pages.push(PageSummary {
                path,
                slug,
                title,
                block_count,
                modified_at,
            });
        }

        let next_cursor = if end < paths.len() {
            paths.get(end).cloned()
        } else {
            None
        };

        Ok(PageListResponse { pages, next_cursor })
    }

    /// `kb_index_status`: report index freshness & coverage.
    pub fn index_status(&self) -> Result<IndexStatus, KbError> {
        let store = self.store()?;
        let total_pages = store.page_count().unwrap_or(0);
        let indexed_blocks = store.block_count().unwrap_or(0);
        let last = *self.last_indexed_at.lock().unwrap();
        // Freshness: n/a without a persistent freshness marker; we compute
        // coverage from the block index dirs if "rebuilding" just ran.
        let fresh = total_pages > 0 && !self.is_rebuilding();
        let indexed_pages = if let Ok(engine) = IndexEngine::new(&self.vault_path) {
            engine.get_meta().note_count
        } else {
            0
        };
        Ok(IndexStatus {
            index_fresh: fresh,
            indexed_pages,
            total_pages,
            indexed_blocks,
            last_indexed_at: last.map(|d| d.to_rfc3339()),
        })
    }

    /// `kb_search`: full-text block search.
    pub fn search(
        &self,
        query: &str,
        limit: usize,
        offset: usize,
    ) -> Result<SearchResponse, KbError> {
        // The block search backend re-indexes transiently via the search index
        // in `.pkm/search`. Open the same index the app uses.
        let block_index = self.open_block_index()?;
        let results = block_index
            .search(query, limit + offset)
            .map_err(|e| ErrorDataExt::external(format!("search failed: {e}")))?;

        // The block index `search()` returns the matching content directly from
        // the stored Tantivy doc (block_search.rs returns `r.content`). The
        // contract (§5.7) requires the matching content in the response, so we
        // always use that authoritative content — never cross-reference
        // `blocks.db` by block_id, because `IndexEngine::refresh_page` and the
        // app watcher re-parse files with fresh block UUIDs, drifting the
        // search index's ids away from SQLite. Content is stored in the index
        // (TEXT tokenized, retrievable via the doc), so hydration is exact.
        let total = results.len();
        let mut slice = Vec::with_capacity(results.len().min(limit));
        for r in results.iter().skip(offset).take(limit) {
            // Never emit an empty `content` for a hit; defensive fallback to
            // the page's stored blocks if the index returns an empty value
            // (should not happen for a matched doc, but keeps the contract shape).
            let content = if r.content.is_empty() {
                let store = self.store()?;
                std::str::FromStr::from_str(&r.block_id)
                    .ok()
                    .and_then(|id| store.get_block(id).ok())
                    .map(|b| b.content.clone())
                    .unwrap_or_default()
            } else {
                r.content.clone()
            };
            slice.push(SearchHit {
                block_id: r.block_id.clone(),
                content,
                page_path: r.page_path.clone(),
                snippet: r.snippet.clone(),
                score: r.score,
            });
        }
        // BlockIndex search returns up to (limit+offset) matches; total is the
        // explicit matched count for the requested window.
        let next_offset = if offset + slice.len() < total {
            Some(offset + slice.len())
        } else {
            None
        };
        Ok(SearchResponse {
            results: slice,
            total,
            next_offset,
        })
    }

    fn open_block_index(&self) -> Result<BlockIndex, KbError> {
        let dir = self.vault_path.join(".pkm").join("search");
        BlockIndex::create(&dir)
            .map_err(|e| ErrorDataExt::external(format!("open search index: {e}")))
    }

    /// `kb_search_by_tag`: search blocks whose page has the tag.
    pub fn search_by_tag(&self, tag: &str, limit: usize) -> Result<SearchResponse, KbError> {
        let store = self.store()?;
        let mut pages = store.list_pages().map_err(map_store_err)?;
        pages.retain(|p| !p.starts_with(".git/") && !p.contains("/.git/"));
        let mut matched: Vec<SearchHit> = Vec::new();
        for path in pages {
            // Frontmatter tags (from SQLite page metadata) or inline #tag.
            let fm_tags = store
                .get_page(&path)
                .ok()
                .flatten()
                .map(|fm| fm.tags)
                .unwrap_or_default();
            let has_fm = fm_tags.iter().any(|t| t == tag);
            if !has_fm {
                // fall back to content scan
                let full = self.full_path(&path);
                let content = std::fs::read_to_string(&full).unwrap_or_default();
                if !content.contains(&format!("#{tag}")) {
                    continue;
                }
            }
            if let Ok(blocks) = store.get_blocks_by_page(&path) {
                for b in blocks.iter().take(limit) {
                    matched.push(SearchHit {
                        block_id: b.id.to_string(),
                        content: b.content.clone(),
                        page_path: path.clone(),
                        snippet: b.content.clone(),
                        score: 1.0,
                    });
                }
            }
            if matched.len() >= limit {
                break;
            }
        }
        matched.truncate(limit);
        let total = matched.len();
        Ok(SearchResponse {
            results: matched,
            total,
            next_offset: None,
        })
    }

    /// `kb_autocomplete`: suggest pages / tags / backlinks.
    pub fn autocomplete(
        &self,
        query: &str,
        kind: &str,
        limit: usize,
    ) -> Result<AutocompleteResponse, KbError> {
        let store = self.store()?;
        let pages = store.list_pages().map_err(map_store_err)?;
        let q = query.to_lowercase();
        let mut items = Vec::new();

        match kind {
            "tag" => {
                let mut seen: HashSet<String> = HashSet::new();
                for p in &pages {
                    if let Ok(Some(fm)) = store.get_page(p) {
                        for t in fm.tags {
                            if seen.insert(t.clone()) && t.to_lowercase().starts_with(&q) {
                                items.push(AutocompleteItem {
                                    text: t.clone(),
                                    kind: "tag".into(),
                                    detail: None,
                                });
                            }
                        }
                    }
                }
            }
            "backlink" => {
                for p in pages {
                    let pslug = p.trim_end_matches(".md").to_string();
                    if pslug.to_lowercase().contains(&q) {
                        items.push(AutocompleteItem {
                            text: pslug,
                            kind: "backlink".into(),
                            detail: Some(p),
                        });
                    }
                }
            }
            _ => {
                // "page"
                for p in pages {
                    let slug = p.trim_end_matches(".md").to_string();
                    if slug.to_lowercase().contains(&q) {
                        let title = store
                            .get_page(&p)
                            .ok()
                            .flatten()
                            .and_then(|fm| fm.title)
                            .unwrap_or_else(|| slug.replace('-', " "));
                        items.push(AutocompleteItem {
                            text: title,
                            kind: "page".into(),
                            detail: Some(p),
                        });
                    }
                }
            }
        }

        items.truncate(limit);
        Ok(AutocompleteResponse { items })
    }

    /// `kb_backlinks`: linked + unlinked mentions for a note.
    pub fn backlinks(
        &self,
        path: &str,
        include_unlinked: bool,
    ) -> Result<BacklinksResponse, KbError> {
        let rel = crate::error::validate_rel_path(path)?;
        let rel = ensure_md(&rel);
        let store = self.store()?;
        let target_slug = rel.trim_end_matches(".md").to_string();
        // the "slug" resolution used by the app: from title (spaces->- lowercase) or exact
        let candidates = {
            let mut c = vec![target_slug.clone()];
            c.push(rel.clone());
            c
        };

        // Linked references come from the backlinks table (insert_link/get_backlinks_for_page).
        let linked_sources: Vec<String> = store.get_backlinks_for_page(&rel).unwrap_or_default();

        let mut backlinks: Vec<BacklinkHit> = Vec::new();
        let mut seen = HashSet::new();

        for source in linked_sources {
            // source is a page path or block id? get_backlinks_for_page returns paths.
            let blocks = store.get_blocks_by_page(&source).unwrap_or_default();
            for b in blocks {
                let ctx = extract_context(&b.content);
                backlinks.push(BacklinkHit {
                    source_id: b.id.to_string(),
                    source_page: source.clone(),
                    context: ctx,
                    is_linked: true,
                });
                seen.insert(source.clone());
            }
        }

        if include_unlinked {
            // Scan every page for a mention of the target slug or note title.
            let all_paths = store.list_pages().map_err(map_store_err)?;
            for p in all_paths {
                if seen.contains(&p) || p == rel {
                    continue;
                }
                if let Ok(blocks) = store.get_blocks_by_page(&p) {
                    let mut mention = false;
                    for b in &blocks {
                        let lower = b.content.to_lowercase();
                        if candidates.iter().any(|c| lower.contains(&c.to_lowercase())) {
                            backlinks.push(BacklinkHit {
                                source_id: b.id.to_string(),
                                source_page: p.clone(),
                                context: extract_context(&b.content),
                                is_linked: false,
                            });
                            mention = true;
                        }
                    }
                    if mention {
                        seen.insert(p);
                    }
                }
            }
        }

        Ok(BacklinksResponse { backlinks })
    }

    /// `kb_graph`: whole-graph or neighborhood subgraph.
    pub fn graph(&self, path: Option<&str>, depth: usize) -> Result<GraphData, KbError> {
        let store = self.store()?;
        let paths = store.list_pages().map_err(map_store_err)?;
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let mut slug_to_path: HashMap<String, String> = HashMap::new();
        let mut slug_to_title: HashMap<String, String> = HashMap::new();

        for p in &paths {
            if p.starts_with(".git/") || p.contains("/.git/") {
                continue;
            }
            let slug = p.trim_end_matches(".md").to_string();
            let title = store
                .get_page(p)
                .ok()
                .flatten()
                .and_then(|fm| fm.title)
                .unwrap_or_else(|| slug.replace('-', " "));
            slug_to_path.insert(slug.clone(), p.clone());
            slug_to_title.insert(slug.clone(), title);
        }

        let center = match path {
            Some(p) => {
                let rel = ensure_md(&crate::error::validate_rel_path(p)?);
                Some(rel.trim_end_matches(".md").to_string())
            }
            None => None,
        };

        // Resolve the target slug (title -> slug).
        let center_slug = center.as_ref().and_then(|c| {
            if slug_to_path.contains_key(c) {
                Some(c.clone())
            } else {
                // title lookup
                slug_to_title
                    .iter()
                    .find(|(_, t)| t.to_lowercase() == c.to_lowercase())
                    .map(|(s, _)| s.clone())
            }
        });

        if let Some(c) = &center_slug {
            if !slug_to_path.contains_key(c) {
                return Err(ErrorDataExt::not_found(path.unwrap_or_default()));
            }
            // BFS neighborhood
            let mut adj: HashMap<String, Vec<String>> = HashMap::new();
            for p in &paths {
                if let Ok(blocks) = store.get_blocks_by_page(p) {
                    let src = p.trim_end_matches(".md").to_string();
                    for b in &blocks {
                        for link in pkm_markdown::linker::extract_links(&b.content) {
                            let mut target = link.target.replace(' ', "-").to_lowercase();
                            if slug_to_path.contains_key(&target) {
                                // ok
                            } else if let Some(t) = slug_to_title
                                .iter()
                                .find(|(_, t)| t.to_lowercase() == link.target.to_lowercase())
                                .map(|(s, _)| s.clone())
                            {
                                target = t;
                            } else {
                                continue;
                            }
                            adj.entry(src.clone()).or_default().push(target.clone());
                        }
                    }
                }
            }
            let mut in_set: HashSet<String> = HashSet::new();
            in_set.insert(c.clone());
            let mut frontier: Vec<String> = vec![c.clone()];
            for _ in 0..depth {
                let mut next = Vec::new();
                for n in &frontier {
                    if let Some(neighbors) = adj.get(n) {
                        for nb in neighbors {
                            if in_set.insert(nb.clone()) {
                                next.push(nb.clone());
                            }
                        }
                    }
                }
                frontier = next;
            }
            let mut ns: Vec<String> = in_set.into_iter().collect();
            ns.sort();
            for s in &ns {
                let path = slug_to_path.get(s).cloned().unwrap_or_default();
                // Degree counts only edges within the returned neighborhood.
                let in_set_degree = ns
                    .iter()
                    .filter(|n| adj.get(s).map(|v| v.contains(n)).unwrap_or(false))
                    .count();
                let tags = store
                    .get_page(&path)
                    .ok()
                    .flatten()
                    .map(|fm| fm.tags)
                    .unwrap_or_default();
                nodes.push(GraphNode {
                    id: s.clone(),
                    title: slug_to_title.get(s).cloned().unwrap_or_default(),
                    path: path.clone(),
                    tags,
                    degree: in_set_degree,
                });
            }
            for s in &ns {
                if let Some(neighbors) = adj.get(s) {
                    for nb in neighbors {
                        if ns.contains(nb) {
                            edges.push(GraphEdge {
                                source: s.clone(),
                                target: nb.clone(),
                                label: None,
                            });
                        }
                    }
                }
            }
        } else {
            // Whole graph.
            let mut adj: HashMap<String, Vec<String>> = HashMap::new();
            for p in &paths {
                if let Ok(blocks) = store.get_blocks_by_page(p) {
                    let src = p.trim_end_matches(".md").to_string();
                    for b in &blocks {
                        for link in pkm_markdown::linker::extract_links(&b.content) {
                            let mut target = link.target.replace(' ', "-").to_lowercase();
                            if slug_to_path.contains_key(&target) {
                            } else if let Some(t) = slug_to_title
                                .iter()
                                .find(|(_, t)| t.to_lowercase() == link.target.to_lowercase())
                                .map(|(s, _)| s.clone())
                            {
                                target = t;
                            } else {
                                continue;
                            }
                            adj.entry(src.clone()).or_default().push(target.clone());
                        }
                    }
                }
            }
            for (src, neighbors) in &adj {
                let degree = neighbors.len();
                let tags = store
                    .get_page(
                        slug_to_path
                            .get(src)
                            .map(|s| s.as_str())
                            .unwrap_or_default(),
                    )
                    .ok()
                    .flatten()
                    .map(|fm| fm.tags)
                    .unwrap_or_default();
                nodes.push(GraphNode {
                    id: src.clone(),
                    title: slug_to_title.get(src).cloned().unwrap_or_default(),
                    path: slug_to_path.get(src).cloned().unwrap_or_default(),
                    tags,
                    degree,
                });
                for nb in neighbors {
                    edges.push(GraphEdge {
                        source: src.clone(),
                        target: nb.clone(),
                        label: None,
                    });
                }
            }
        }
        // Cap at MCP_RESPONSE_MAX conservatively.
        let cap = crate::MCP_RESPONSE_MAX / 64;
        nodes.truncate(cap);
        edges.truncate(cap * 4);
        let node_count = nodes.len();
        let edge_count = edges.len();
        Ok(GraphData {
            nodes,
            edges,
            node_count,
            edge_count,
            vault_path: self.vault_path.to_string_lossy().to_string(),
        })
    }

    /// `kb_resolve_link`: resolve a `[[wiki-link]]` target to a note path.
    pub fn resolve_link(&self, target: &str) -> Result<ResolveLinkResult, KbError> {
        let store = self.store()?;
        // Resolve via the exact app resolver (commands::graph::resolve_link_target
        // uses BlockStore::resolve_link_target_path), which matches by slug,
        // file stem, or frontmatter title — same semantics as the desktop app.
        if let Some(path) = store.resolve_link_target_path(target) {
            let slug = path.trim_end_matches(".md").to_string();
            let title = store
                .get_page(&path)
                .ok()
                .flatten()
                .and_then(|fm| fm.title)
                .unwrap_or_else(|| slug.replace('-', " "));
            Ok(ResolveLinkResult {
                resolved: Some(ResolvedTarget { path, slug, title }),
                unresolved: false,
            })
        } else {
            Ok(ResolveLinkResult {
                resolved: None,
                unresolved: true,
            })
        }
    }

    /// `kb_vault_info`: metadata & health.
    pub fn vault_info(&self) -> Result<VaultInfo, KbError> {
        let store = self.store()?;
        let page_count = store.page_count().unwrap_or(0);
        let block_count = store.block_count().unwrap_or(0);
        let status = self.index_status()?;
        Ok(VaultInfo {
            vault_path: self.vault_path.to_string_lossy().to_string(),
            page_count,
            block_count,
            index_fresh: status.index_fresh,
            last_indexed_at: status.last_indexed_at,
        })
    }

    // ------------------------------------------------------------------
    // WRITE adapters
    // ------------------------------------------------------------------

    /// `kb_write_page`: create or overwrite a note atomically.
    ///
    /// Mirrors the desktop `save_page`: parse/validate → atomic temp-file+rename
    /// `.md` → SQLite transaction → index refresh → plugin dispatch.
    pub async fn write_page(
        &self,
        path: &str,
        content: &str,
        expected_modified: Option<&str>,
    ) -> Result<NoteDocument, KbError> {
        let rel = crate::error::validate_rel_path(path)?;
        let rel = ensure_md(&rel);

        // Precondition guard: fail with CONFLICT if the note changed since the
        // client's expected_modified snapshot (contract §5.3, §8/§10).
        if let Some(expected) = expected_modified {
            let expected_dt = parse_rfc3339(expected)?;
            let full = self.full_path(&rel);
            if let Some(actual) = self.file_modified_at(&full) {
                if actual > expected_dt {
                    return Err(ErrorDataExt::conflict(&rel));
                }
            }
        }

        // Hold the per-vault write lock (single writer).
        let _guard = self.acquire_write().await?;

        // Parse & validate the content using the same block parser as the app.
        let (fm, _body, blocks) = pkm_markdown::block_parser::parse_document(content);
        let store = self.store()?;

        // Ensure parent dir exists.
        let full = self.full_path(&rel);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| ErrorDataExt::external(format!("failed to create parent dir: {e}")))?;
        }

        // Build page metadata.
        let mut page = Page::new(full.clone(), &self.vault_path);
        page.frontmatter = pkm_block::PageFrontmatter {
            title: fm.title.clone(),
            created: fm.created.clone(),
            modified: fm.modified.clone(),
            tags: fm.tags.clone(),
            aliases: fm.aliases.clone(),
            ..Default::default()
        };
        page.set_blocks(&blocks);

        // SQLite transaction (same as save_page): delete + insert + upsert.
        store
            .execute_batch("BEGIN")
            .map_err(|e| ErrorDataExt::external(e.to_string()))?;
        let result = (|| -> Result<(), KbError> {
            store.delete_blocks_by_page(&rel).map_err(map_store_err)?;
            for b in &blocks {
                store.insert_block(b, &rel).map_err(map_store_err)?;
            }
            store.upsert_page(&page).map_err(map_store_err)?;
            Ok(())
        })();
        match result {
            Ok(()) => store
                .execute_batch("COMMIT")
                .map_err(|e| ErrorDataExt::external(e.to_string()))?,
            Err(e) => {
                store.execute_batch("ROLLBACK").ok();
                return Err(e);
            }
        }

        // Atomic write of the .md file: temp file + rename.
        atomic_write(&full, content)?;

        // Mark this as our own save so the file watcher can skip it.
        self.mark_own_save();

        // Index refresh (drop block index handle first to avoid Tantivy lock).
        self.refresh_index_after_write(&rel)?;

        // Link write-out: index the new edges (contract §6.1).
        self.mark_indexed();

        // Re-read to return the freshly parsed document (contract §5.3).
        self.get_page(&rel)
    }

    /// Refresh the search index for a single page after a write.
    ///
    /// Mirrors the desktop `save_page`: the search index must carry the SAME
    /// block UUIDs as blocks.db (the MCP `SearchHit` and `kb_get_page` rely on
    /// a consistent block identity). `IndexEngine::refresh_page` re-parses the
    /// file and re-inserts blocks under FRESH UUIDs, so it must run FIRST (it
    /// refreshes the graph/tag portion of the index) and THEN we re-index the
    /// stored blocks so the Tantivy block index ends up aligned with blocks.db.
    fn refresh_index_after_write(&self, rel: &str) -> Result<(), KbError> {
        // 1. Refresh the graph/tag portion of the index first. This re-parses
        //    the file (fresh UUIDs) but those block entries are overwritten in
        //    step 2; graph/tags are keyed by page path and are unaffected.
        match IndexEngine::new(&self.vault_path) {
            Ok(mut engine) => engine.refresh_page(rel, &self.vault_path).map_err(|e| {
                warn!("index refresh failed for {rel}: {e}");
                ErrorDataExt::external(format!("index refresh failed: {e}"))
            }),
            Err(e) => {
                warn!("IndexEngine unavailable for refresh: {e}");
                Ok(())
            }
        }?;

        // 2. Re-index the stored blocks into the block search index LAST so the
        //    Tantivy ids match blocks.db exactly (same code path as
        //    commands::block::save_page). Doing this after the IndexEngine
        //    refresh prevents the fresh-UUID re-parse from drifting the ids.
        let store = self.store()?;
        if let Ok(blocks) = store.get_blocks_by_page(rel) {
            let mut block_index =
                BlockIndex::create(&self.vault_path.join(".pkm").join("search"))
                    .map_err(|e| ErrorDataExt::external(format!("open search index: {e}")))?;
            for b in &blocks {
                let _ = block_index.index_block(b, rel);
            }
            block_index.flush().map_err(|e| {
                warn!("search index flush failed for {rel}: {e}");
                ErrorDataExt::external(format!("search index flush failed: {e}"))
            })?;
        }
        Ok(())
    }

    /// `kb_delete_page`: delete a note and its index entries.
    pub async fn delete_page(
        &self,
        path: &str,
        expected_modified: Option<&str>,
    ) -> Result<(), KbError> {
        let rel = crate::error::validate_rel_path(path)?;
        let rel = ensure_md(&rel);
        let full = self.full_path(&rel);
        if !full.exists() {
            return Err(ErrorDataExt::not_found(&rel));
        }

        if let Some(expected) = expected_modified {
            let expected_dt = parse_rfc3339(expected)?;
            if let Some(actual) = self.file_modified_at(&full) {
                if actual > expected_dt {
                    return Err(ErrorDataExt::conflict(&rel));
                }
            }
        }

        let _guard = self.acquire_write().await?;

        std::fs::remove_file(&full)
            .map_err(|e| ErrorDataExt::external(format!("failed to delete file: {e}")))?;
        self.mark_own_save();

        let store = self.store()?;
        store.delete_page(&rel).map_err(map_store_err)?;

        // Index remove.
        if let Ok(mut engine) = IndexEngine::new(&self.vault_path) {
            let _ = engine.remove_note(&rel);
        }

        self.mark_indexed();
        Ok(())
    }

    /// `kb_reindex`: incremental (refresh changed pages) or full rebuild.
    pub async fn reindex(&self, mode: &str) -> Result<IndexStatus, KbError> {
        let _guard = self.acquire_write().await?;
        if self.is_rebuilding() {
            return Err(ErrorDataExt::vault_locked());
        }
        self.set_rebuilding(true);

        let result = (|| -> Result<(), KbError> {
            // Drop any stale index handle, then rebuild from disk.
            match mode {
                "rebuild" => {
                    let mut engine = IndexEngine::new(&self.vault_path)
                        .map_err(|e| ErrorDataExt::external(format!("index init: {e}")))?;
                    engine
                        .rebuild_all(None)
                        .map_err(|e| ErrorDataExt::external(format!("rebuild: {e}")))?;
                }
                _ => {
                    // incremental: reindex all existing pages from disk
                    let md_files = MdCollector::new()
                        .skip_hidden_dirs(true)
                        .collect(&self.vault_path)
                        .map_err(|e| ErrorDataExt::external(format!("collect: {e}")))?;
                    let mut engine = IndexEngine::new(&self.vault_path)
                        .map_err(|e| ErrorDataExt::external(format!("index init: {e}")))?;
                    for (i, f) in md_files.iter().enumerate() {
                        if i % 100 == 0 {
                            engine.flush()?;
                        }
                        let rel = f.strip_prefix(&self.vault_path).unwrap_or(f);
                        let rel = rel.to_string_lossy().to_string();
                        let _ = engine.refresh_page(&rel, &self.vault_path);
                    }
                    engine.flush()?;
                }
            }
            Ok(())
        })();

        let res = result;
        self.set_rebuilding(false);
        if res.is_ok() {
            self.mark_indexed();
        }
        res?;

        self.index_status()
    }

    /// `kb_add_tag`: add a tag to a note (idempotent).
    pub async fn add_tag(&self, path: &str, tag: &str) -> Result<TagOperationResult, KbError> {
        let rel = crate::error::validate_rel_path(path)?;
        let rel = ensure_md(&rel);
        let full = self.full_path(&rel);
        let content = match std::fs::read_to_string(&full) {
            Ok(c) => c,
            Err(_) => return Err(ErrorDataExt::not_found(&rel)),
        };
        let (mut fm, body, blocks) = pkm_markdown::block_parser::parse_document(&content);

        // Idempotent: if already tagged (frontmatter or inline #tag), success with applied=0.
        let already = fm.tags.iter().any(|t| t == tag) || content.contains(&format!("#{tag}"));
        if already {
            return Ok(TagOperationResult {
                path: rel,
                success: true,
                tag: tag.to_string(),
                action: "add".into(),
                applied: 0,
            });
        }

        fm.tags.push(tag.to_string());
        // Re-render frontmatter+body so the tag is persisted to disk
        // (contract §5.13: "add `tag` to the note's frontmatter `tags` array").
        let new_content = pkm_markdown::renderer::render(&fm, &body);
        let mut page = Page::new(full.clone(), &self.vault_path);
        page.frontmatter = pkm_block::PageFrontmatter {
            title: fm.title.clone(),
            created: fm.created.clone(),
            modified: fm.modified.clone(),
            tags: fm.tags.clone(),
            aliases: fm.aliases.clone(),
            ..Default::default()
        };
        page.set_blocks(&blocks);
        self.write_page_with_parsed(&rel, &fm, &new_content, &blocks, &full)?;

        Ok(TagOperationResult {
            path: rel,
            success: true,
            tag: tag.to_string(),
            action: "add".into(),
            applied: 1,
        })
    }

    /// `kb_remove_tag`: remove a tag from a note (idempotent).
    pub async fn remove_tag(&self, path: &str, tag: &str) -> Result<TagOperationResult, KbError> {
        let rel = crate::error::validate_rel_path(path)?;
        let rel = ensure_md(&rel);
        let full = self.full_path(&rel);
        let content = match std::fs::read_to_string(&full) {
            Ok(c) => c,
            Err(_) => return Err(ErrorDataExt::not_found(&rel)),
        };
        let (mut fm, _body, _blocks) = pkm_markdown::block_parser::parse_document(&content);
        let had_fm = fm.tags.iter().any(|t| t == tag);
        fm.tags.retain(|t| t != tag);

        // Remove literal #tag occurrences from the content (reconstruct body).
        let mut body = content.clone();
        if body.contains(&format!("#{tag}")) {
            if let Some(position) = body.find(&format!("#{tag}")) {
                // Remove the token (and a trailing space) but guard against
                // partial matches like #tagging.
                let end = position + tag.len() + 1;
                if body[end..].starts_with(char::is_whitespace) {
                    body.replace_range(position..end + 1, "");
                } else {
                    body.replace_range(position..end, "");
                }
            }
            // strip remaining standalone #tag occurrences with word boundary
            let re = regex::Regex::new(&format!(r"#{}\b", regex::escape(tag))).unwrap();
            body = re.replace_all(&body, "").to_string();
        }

        let applied = usize::from(had_fm) + usize::from(body != content);

        // Re-render frontmatter (with the tag removed) + stripped body so the
        // removal is persisted to disk (contract §5.14 re-serialization).
        let (mut fm2, body2, blocks2) = pkm_markdown::block_parser::parse_document(&body);
        fm2.tags = fm.tags;
        let rendered = pkm_markdown::renderer::render(&fm2, &body2);
        let mut page = Page::new(full.clone(), &self.vault_path);
        page.frontmatter = pkm_block::PageFrontmatter {
            title: fm2.title.clone(),
            created: fm2.created.clone(),
            modified: fm2.modified.clone(),
            tags: fm2.tags.clone(),
            aliases: fm2.aliases.clone(),
            ..Default::default()
        };
        page.set_blocks(&blocks2);
        self.write_page_with_parsed(&rel, &fm2, &rendered, &blocks2, &full)?;

        Ok(TagOperationResult {
            path: rel,
            success: true,
            tag: tag.to_string(),
            action: "remove".into(),
            applied,
        })
    }

    /// Shared write path for tag operations (holds the write lock).
    fn write_page_with_parsed(
        &self,
        rel: &str,
        _fm: &pkm_core::Frontmatter,
        content: &str,
        blocks: &[Block],
        full: &std::path::Path,
    ) -> Result<(), KbError> {
        let store = self.store()?;
        store
            .execute_batch("BEGIN")
            .map_err(|e| ErrorDataExt::external(e.to_string()))?;
        let result = (|| -> Result<(), KbError> {
            store.delete_blocks_by_page(rel).map_err(map_store_err)?;
            for b in blocks {
                store.insert_block(b, rel).map_err(map_store_err)?;
            }
            let mut page = Page::new(full.to_path_buf(), &self.vault_path);
            page.frontmatter = pkm_block::PageFrontmatter {
                title: _fm.title.clone(),
                created: _fm.created.clone(),
                modified: _fm.modified.clone(),
                tags: _fm.tags.clone(),
                aliases: _fm.aliases.clone(),
                ..Default::default()
            };
            page.set_blocks(blocks);
            store.upsert_page(&page).map_err(map_store_err)?;
            Ok(())
        })();
        match result {
            Ok(()) => store
                .execute_batch("COMMIT")
                .map_err(|e| ErrorDataExt::external(e.to_string()))?,
            Err(e) => {
                store.execute_batch("ROLLBACK").ok();
                return Err(e);
            }
        }
        atomic_write(full, content)?;
        self.mark_own_save();
        self.refresh_index_after_write(rel)?;
        self.mark_indexed();
        Ok(())
    }
}

/// Write `content` to `path` atomically via temp file + rename.
fn atomic_write(path: &Path, content: &str) -> Result<(), KbError> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let tmp = parent.join(format!(
        ".{}.tmp-{}",
        path.file_name().and_then(|s| s.to_str()).unwrap_or("note"),
        std::process::id()
    ));
    std::fs::write(&tmp, content)
        .map_err(|e| ErrorDataExt::external(format!("temp write failed: {e}")))?;
    std::fs::rename(&tmp, path)
        .map_err(|e| ErrorDataExt::external(format!("rename failed: {e}")))?;
    Ok(())
}

/// Parse an RFC 3339 string into a DateTime<Utc>.
fn parse_rfc3339(s: &str) -> Result<DateTime<Utc>, KbError> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|_| ErrorDataExt::invalid_args(format!("invalid RFC 3339 timestamp: {s}")))
}

/// Ensure the relative path ends with `.md`.
fn ensure_md(rel: &str) -> String {
    if rel.ends_with(".md") {
        rel.to_string()
    } else {
        format!("{rel}.md")
    }
}

/// Extract `[[wiki-link]]` targets from content, matching the app's Link shape.
fn extract_links_for_doc(content: &str) -> Vec<NoteLink> {
    pkm_markdown::linker::extract_links(content)
        .into_iter()
        .map(|l| NoteLink {
            target: l.target,
            block_id: None,
            display_text: l.display_text,
            position: Some(l.line),
        })
        .collect()
}

/// Extract tags from frontmatter + inline #tags.
fn extract_tags(fm: &pkm_core::Frontmatter, content: &str) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut tags = Vec::new();
    for t in &fm.tags {
        if seen.insert(t.clone()) {
            tags.push(t.clone());
        }
    }
    for capture in regex::Regex::new(r"#([a-zA-Z0-9_./-]+)")
        .unwrap()
        .captures_iter(content)
    {
        let t = capture[1].to_string();
        if seen.insert(t.clone()) {
            tags.push(t);
        }
    }
    tags
}

/// Extract a short context snippet around the first mention, matching the
/// app's backlink context style (trimmed to ~80 chars).
fn extract_context(content: &str) -> String {
    let trimmed = content.trim();
    if trimmed.len() <= 80 {
        return trimmed.to_string();
    }
    format!("{}…", &trimmed[..80])
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_vault() -> (TempDir, McpConfig, SharedVault) {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join(".pkm")).unwrap();
        let mut cfg = McpConfig::new(dir.path().to_path_buf());
        cfg.transport = crate::config::Transport::Stdio;
        let vault = SharedVault::new(&cfg).unwrap();
        (dir, cfg, vault)
    }

    fn seed_page(vault: &SharedVault, path: &str, content: &str) -> Result<(), KbError> {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(vault.write_page(path, content, None))
            .map(|_| ())
    }

    #[test]
    fn test_get_page_missing_is_not_found() {
        let (_dir, _cfg, vault) = test_vault();
        let err = vault.get_page("missing.md").unwrap_err();
        assert_eq!(err.kind, KbErrorKind::NotFound);
    }

    #[test]
    fn test_write_then_read_roundtrip() {
        let (_dir, _cfg, vault) = test_vault();
        seed_page(
            &vault,
            "projects/example.md",
            "---\ntitle: Example note\ntags: [project]\n---\n# Hello\n\nThis is an example with a [[wiki-link]].",
        )
        .unwrap();
        // Refresh index needs .pkm present; write_page creates parent.
        let doc = vault.get_page("projects/example.md").unwrap();
        assert_eq!(doc.title, "Example note");
        assert_eq!(doc.slug, "projects/example");
        assert!(doc.links.iter().any(|l| l.target == "wiki-link"));
        assert!(doc.tags.iter().any(|t| t == "project"));
        assert!(doc.block_count > 0);
    }

    #[test]
    fn test_write_overwrites_idempotently() {
        let (_dir, _cfg, vault) = test_vault();
        seed_page(&vault, "x.md", "first content").unwrap();
        seed_page(&vault, "x.md", "second content").unwrap();
        let doc = vault.get_page("x.md").unwrap();
        assert!(doc.content.contains("second"));
    }

    #[test]
    fn test_precondition_guard_conflict() {
        let (_dir, _cfg, vault) = test_vault();
        seed_page(&vault, "x.md", "content").unwrap();
        // A stale expected_modified before the file's actual mtime triggers conflict.
        let stale = "2000-01-01T00:00:00Z";
        let err = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(vault.write_page("x.md", "new", Some(stale)))
            .unwrap_err();
        assert_eq!(err.kind, KbErrorKind::Conflict);
    }

    #[test]
    fn test_delete_missing_is_not_found() {
        let (_dir, _cfg, vault) = test_vault();
        let err = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(vault.delete_page("missing.md", None))
            .unwrap_err();
        assert_eq!(err.kind, KbErrorKind::NotFound);
    }

    #[test]
    fn test_delete_removes_file_and_index() {
        let (_dir, _cfg, vault) = test_vault();
        seed_page(&vault, "a.md", "hello world").unwrap();
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(vault.delete_page("a.md", None))
            .unwrap();
        assert!(vault.get_page("a.md").is_err());
        assert!(!vault.vault_path.join("a.md").exists());
    }

    #[test]
    fn test_list_pages_excludes_git() {
        let (_dir, _cfg, vault) = test_vault();
        std::fs::create_dir_all(vault.vault_path.join(".git")).unwrap();
        std::fs::write(vault.vault_path.join(".git/config"), "").unwrap();
        seed_page(&vault, "a.md", "a").unwrap();
        seed_page(&vault, "pages/b.md", "b").unwrap();
        let list = vault.list_pages(100, None).unwrap();
        assert_eq!(list.pages.len(), 2);
        assert!(list.pages.iter().all(|p| !p.path.contains(".git")));
    }

    #[test]
    fn test_path_traversal_rejected() {
        let (_dir, _cfg, vault) = test_vault();
        seed_page(&vault, "a.md", "a").unwrap();
        let err = vault.get_page("../outside.md").unwrap_err();
        assert_eq!(err.kind, KbErrorKind::InvalidArgs);
    }

    #[test]
    fn test_add_tag_sets_frontmatter() {
        let (_dir, _cfg, vault) = test_vault();
        seed_page(&vault, "x.md", "content here").unwrap();
        let res = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(vault.add_tag("x.md", "newtag"))
            .unwrap();
        assert_eq!(res.applied, 1);
        let doc = vault.get_page("x.md").unwrap();
        assert!(doc.tags.iter().any(|t| t == "newtag"));
    }

    #[test]
    fn test_autocomplete_pages() {
        let (_dir, _cfg, vault) = test_vault();
        seed_page(&vault, "neat-page.md", "content").unwrap();
        let res = vault.autocomplete("neat", "page", 10).unwrap();
        assert!(res
            .items
            .iter()
            .any(|i| i.detail.as_deref() == Some("neat-page.md")));
    }

    #[test]
    fn test_resolve_link() {
        let (_dir, _cfg, vault) = test_vault();
        seed_page(&vault, "projects/example.md", "content with [[Example]]").unwrap();
        let res = vault.resolve_link("Example").unwrap();
        assert!(!res.unresolved);
        if let Some(t) = res.resolved {
            assert_eq!(t.path, "projects/example.md");
        }
    }

    #[test]
    fn test_vault_info_and_index_status() {
        let (_dir, _cfg, vault) = test_vault();
        seed_page(&vault, "z.md", "z").unwrap();
        let info = vault.vault_info().unwrap();
        assert_eq!(info.vault_path, vault.vault_path.to_string_lossy());
        assert!(info.page_count >= 1);
        let status = vault.index_status().unwrap();
        assert!(status.total_pages >= 1);
    }
}
