//! Graph visualization commands — expose note-level graph data to the frontend.

use crate::commands::vault::AppState;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use tracing::{debug, info};

/// A node in the graph, ready for frontend rendering.
#[derive(Debug, Clone, Serialize)]
pub struct GraphNodeDto {
    pub id: String,
    pub title: String,
    pub path: String,
    pub tags: Vec<String>,
    pub degree: usize,
}

/// A directed edge between two nodes.
#[derive(Debug, Clone, Serialize)]
pub struct GraphEdgeDto {
    pub source: String,
    pub target: String,
    pub label: Option<String>,
}

/// Full graph data for force-directed layout rendering.
#[derive(Debug, Clone, Serialize)]
pub struct GraphDataDto {
    pub nodes: Vec<GraphNodeDto>,
    pub edges: Vec<GraphEdgeDto>,
    pub node_count: usize,
    pub edge_count: usize,
    pub vault_path: String,
}

/// A connected component (group of interlinked notes).
#[derive(Debug, Clone, Serialize)]
pub struct ComponentDto {
    pub nodes: Vec<GraphNodeDto>,
    pub size: usize,
}

/// Orphaned note info.
#[derive(Debug, Clone, Serialize)]
pub struct OrphanDto {
    pub slug: String,
    pub title: String,
    pub path: String,
}

/// Build graph data directly from SQLite blocks instead of re-parsing markdown files.
/// This avoids expensive file I/O, markdown parsing, and Tantivy indexing.
fn build_graph_data_from_store(
    store: &pkm_block::BlockStore,
    vault_path: &str,
) -> Result<GraphDataDto, String> {
    let paths = store.list_pages().map_err(|e| e.to_string())?;
    let meta = PageMetaIndex::from_store(store)?;

    // Build edges by scanning blocks for [[wiki-links]]
    let mut outgoing: HashMap<String, Vec<GraphEdgeDto>> = HashMap::new();
    let mut degree: HashMap<String, usize> = HashMap::new();

    for (slug, page_path) in &meta.slug_to_path {
        if let Ok(blocks) = store.get_blocks_by_page(page_path) {
            for block in &blocks {
                let links = pkm_markdown::linker::extract_links(&block.content);
                for link in links {
                    // Resolve the link target to a slug
                    let target_slug = meta.resolve_slug(&link.target);
                    if let Some(target) = target_slug {
                        // Include all resolved links including self-links
                        outgoing
                            .entry(slug.clone())
                            .or_default()
                            .push(GraphEdgeDto {
                                source: slug.clone(),
                                target: target.clone(),
                                label: link.display_text.clone(),
                            });
                        // Degree: source always +1, target +1 only if different (avoid double-count self-links)
                        *degree.entry(slug.clone()).or_default() += 1;
                        if target != *slug {
                            *degree.entry(target.clone()).or_default() += 1;
                        }
                    }
                }
            }
        }
    }

    let nodes: Vec<GraphNodeDto> = paths
        .iter()
        .map(|path| {
            let slug = slug_from_path(path);
            let mut node = meta.get_node(&slug);
            node.degree = degree.get(&slug).copied().unwrap_or(0);
            node
        })
        .collect();

    let edges: Vec<GraphEdgeDto> = outgoing.into_values().flatten().collect();
    let node_count = nodes.len();
    let edge_count = edges.len();

    Ok(GraphDataDto {
        nodes,
        edges,
        node_count,
        edge_count,
        vault_path: vault_path.to_string(),
    })
}

/// Derive a note slug from a vault-relative path (e.g. "pages/my-note.md" → "my-note").
fn slug_from_path(path: &str) -> String {
    std::path::Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("untitled")
        .to_string()
}

#[tauri::command]
pub async fn get_graph_data(state: tauri::State<'_, AppState>) -> Result<GraphDataDto, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let vault_path_str = state.vault_path.to_string_lossy().to_string();

    info!("Building graph from SQLite: {}", vault_path_str);

    let store = pkm_block::BlockStore::open(&state.db_path).map_err(|e| e.to_string())?;
    let data = build_graph_data_from_store(&store, &vault_path_str)?;

    debug!("Found {} nodes, {} edges", data.node_count, data.edge_count);

    Ok(data)
}

fn get_connected_components_from_store(
    store: &pkm_block::BlockStore,
) -> Result<Vec<ComponentDto>, String> {
    let meta = PageMetaIndex::from_store(store)?;

    // Build adjacency list
    let mut adj: HashMap<String, Vec<String>> = HashMap::new();
    for (slug, page_path) in &meta.slug_to_path {
        if let Ok(blocks) = store.get_blocks_by_page(page_path) {
            for block in &blocks {
                let links = pkm_markdown::linker::extract_links(&block.content);
                for link in links {
                    if let Some(target) = meta.resolve_slug(&link.target) {
                        // Include all links (self-links in adjacency are harmless for BFS)
                        adj.entry(slug.clone()).or_default().push(target.clone());
                        adj.entry(target).or_default().push(slug.clone());
                    }
                }
            }
        }
    }

    // BFS for connected components
    let mut visited: HashSet<String> = HashSet::new();
    let mut components: Vec<Vec<String>> = Vec::new();

    for slug in meta.slug_to_path.keys() {
        if visited.contains(slug) {
            continue;
        }
        let mut component = Vec::new();
        let mut queue = Vec::new();
        queue.push(slug.clone());
        visited.insert(slug.clone());

        while let Some(current) = queue.pop() {
            component.push(current.clone());
            if let Some(neighbors) = adj.get(&current) {
                for neighbor in neighbors {
                    if visited.insert(neighbor.clone()) {
                        queue.push(neighbor.clone());
                    }
                }
            }
        }
        components.push(component);
    }

    // Build DTOs
    let mut result: Vec<ComponentDto> = components
        .into_iter()
        .map(|group| {
            let nodes: Vec<GraphNodeDto> = group
                .iter()
                .map(|slug| {
                    let mut node = meta.get_node(slug);
                    node.degree = adj.get(slug).map(|n| n.len()).unwrap_or(0);
                    node
                })
                .collect();
            let size = nodes.len();
            ComponentDto { nodes, size }
        })
        .collect();

    result.sort_by_key(|c| std::cmp::Reverse(c.size));
    Ok(result)
}

#[tauri::command]
pub async fn get_connected_components(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<ComponentDto>, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let store = pkm_block::BlockStore::open(&state.db_path).map_err(|e| e.to_string())?;
    get_connected_components_from_store(&store)
}

fn get_orphaned_notes_from_store(store: &pkm_block::BlockStore) -> Result<Vec<OrphanDto>, String> {
    let meta = PageMetaIndex::from_store(store)?;

    // Track which slugs have any connections
    let mut connected: HashSet<String> = HashSet::new();

    for (slug, page_path) in &meta.slug_to_path {
        if let Ok(blocks) = store.get_blocks_by_page(page_path) {
            for block in &blocks {
                let links = pkm_markdown::linker::extract_links(&block.content);
                for link in links {
                    if let Some(target) = meta.resolve_slug(&link.target) {
                        // Include all links (HashSet insert is idempotent for self-links)
                        connected.insert(slug.clone());
                        connected.insert(target);
                    }
                }
            }
        }
    }

    let orphans: Vec<OrphanDto> = meta
        .all_slugs()
        .filter(|slug| !connected.contains(*slug))
        .map(|slug| OrphanDto {
            slug: slug.to_string(),
            title: meta.slug_to_title.get(slug).cloned().unwrap_or_default(),
            path: meta.slug_to_path.get(slug).cloned().unwrap_or_default(),
        })
        .collect();

    Ok(orphans)
}

#[tauri::command]
pub async fn get_orphaned_notes(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<OrphanDto>, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let store = pkm_block::BlockStore::open(&state.db_path).map_err(|e| e.to_string())?;
    get_orphaned_notes_from_store(&store)
}

#[derive(Debug, Serialize)]
pub struct LinkTargetDto {
    pub page_path: Option<String>,
    pub slug: Option<String>,
    pub title: Option<String>,
}

#[tauri::command]
pub async fn resolve_link_target(
    target: String,
    state: tauri::State<'_, AppState>,
) -> Result<LinkTargetDto, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let store = pkm_block::BlockStore::open(&state.db_path).map_err(|e| e.to_string())?;
    let meta = PageMetaIndex::from_store(&store)?;

    let resolved_slug = meta.resolve_slug(&target);
    let result = resolved_slug.map(|slug| {
        let page_path = meta.slug_to_path.get(&slug).cloned();
        let title = meta.slug_to_title.get(&slug).cloned();
        LinkTargetDto {
            page_path,
            slug: Some(slug),
            title,
        }
    });

    Ok(result.unwrap_or(LinkTargetDto {
        page_path: None,
        slug: None,
        title: None,
    }))
}

#[tauri::command]
pub async fn rebuild_graph(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let mut vault = state.lock().map_err(|e| e.to_string())?;
    let _guard = crate::commands::vault::IndexingGuard::new(&vault)?;

    let progress = super::make_progress_callback(app);

    // Drop guard before accessing IndexEngine (&mut self)
    drop(_guard);

    let engine = vault.ensure_index()?;
    let _notes = engine
        .rebuild_all(Some(progress))
        .map_err(|e| format!("Graph rebuild failed: {}", e))?;

    let graph = engine.get_graph();
    Ok(format!(
        "Indexed {} notes with {} edges",
        graph.node_count(),
        graph.edge_count()
    ))
}

/// Pre-built index of all pages in the vault for fast slug/title/path resolution.
/// Eliminates duplicated map-building across graph, connected components, orphans, and link resolution.
struct PageMetaIndex {
    slug_to_path: HashMap<String, String>,
    slug_to_title: HashMap<String, String>,
    slug_to_tags: HashMap<String, Vec<String>>,
    title_to_slug: HashMap<String, String>,
}

impl PageMetaIndex {
    fn from_store(store: &pkm_block::BlockStore) -> Result<Self, String> {
        let paths = store.list_pages().map_err(|e| e.to_string())?;
        let mut slug_to_path = HashMap::new();
        let mut slug_to_title = HashMap::new();
        let mut slug_to_tags = HashMap::new();
        let mut title_to_slug = HashMap::new();

        for path in &paths {
            let slug = slug_from_path(path);
            slug_to_path.insert(slug.clone(), path.clone());

            let fm = store.get_page(path).ok().flatten();
            let title = fm
                .as_ref()
                .and_then(|f| f.title.clone())
                .unwrap_or_else(|| slug.replace('-', " "));
            slug_to_title.insert(slug.clone(), title.clone());
            title_to_slug.insert(title.to_lowercase(), slug.clone());

            let tags = fm.map(|f| f.tags).unwrap_or_default();
            slug_to_tags.insert(slug, tags);
        }

        Ok(Self {
            slug_to_path,
            slug_to_title,
            slug_to_tags,
            title_to_slug,
        })
    }

    fn resolve_slug(&self, target: &str) -> Option<String> {
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

    fn get_node(&self, slug: &str) -> GraphNodeDto {
        let degree = 0; // placeholder — caller sets degree
        GraphNodeDto {
            id: slug.to_string(),
            title: self.slug_to_title.get(slug).cloned().unwrap_or_default(),
            path: self.slug_to_path.get(slug).cloned().unwrap_or_default(),
            tags: self.slug_to_tags.get(slug).cloned().unwrap_or_default(),
            degree,
        }
    }

    fn all_slugs(&self) -> impl Iterator<Item = &str> {
        self.slug_to_path.keys().map(|s| s.as_str())
    }

    #[allow(dead_code)]
    fn is_orphan(&self, slug: &str) -> bool {
        !self.slug_to_path.contains_key(slug)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pkm_block::{Block, BlockStore, Page};
    use std::path::{Path, PathBuf};
    use uuid::Uuid;

    fn insert_test_page(store: &BlockStore, vault_root: &Path, rel_path: &str) {
        let full_path = vault_root.join(rel_path);
        let page = Page::new(full_path, vault_root);
        store.upsert_page(&page).unwrap();
    }

    #[test]
    fn test_self_link_included_in_edges() {
        let store = BlockStore::open_in_memory().unwrap();
        let vault_root = PathBuf::from("/tmp/test-vault");
        let slug = "test-self-link";
        let rel_path = format!("pages/{}.md", slug);

        insert_test_page(&store, &vault_root, &rel_path);

        let block = Block::new(Uuid::new_v4(), format!("Self reference [[{}]]", slug));
        store.insert_block(&block, &rel_path).unwrap();

        let data = build_graph_data_from_store(&store, "/tmp/test-vault").unwrap();

        assert_eq!(data.node_count, 1, "should have 1 node");
        assert_eq!(data.edge_count, 1, "should have 1 self-link edge");
        assert_eq!(data.edges[0].source, slug, "edge source should be the slug");
        assert_eq!(
            data.edges[0].target, slug,
            "edge target should be the slug (self-link)"
        );
        assert_eq!(
            data.nodes[0].degree, 1,
            "self-link degree should be 1 (not double-counted)"
        );
    }

    #[test]
    fn test_self_link_does_not_affect_connected_components() {
        let store = BlockStore::open_in_memory().unwrap();
        let vault_root = PathBuf::from("/tmp/test-vault");

        insert_test_page(&store, &vault_root, "pages/page-a.md");
        insert_test_page(&store, &vault_root, "pages/page-b.md");

        let block_a = Block::new(Uuid::new_v4(), "[[page-a]]".into());
        store.insert_block(&block_a, "pages/page-a.md").unwrap();

        let block_b = Block::new(Uuid::new_v4(), "[[page-a]]".into());
        store.insert_block(&block_b, "pages/page-b.md").unwrap();

        let components = get_connected_components_from_store(&store).unwrap();

        assert_eq!(components.len(), 1, "should have 1 connected component");
        assert_eq!(components[0].size, 2, "component should contain both pages");
    }
}
