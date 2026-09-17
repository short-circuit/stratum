//! Graph visualization commands — expose note-level graph data to the frontend.

mod meta;

use crate::commands::graph::meta::PageMetaIndex;
use crate::commands::vault::AppState;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::sync::OnceLock;
use tracing::{debug, info};

/// In-memory cache for graph panel data.
/// Invalidated by `invalidate_graph_cache()` after any page/block mutation.
static GRAPH_CACHE: OnceLock<Mutex<Option<GraphPanelDataDto>>> = OnceLock::new();

/// Clear the in-memory graph cache.
/// Call this after any page or block mutation so the next graph view rebuilds fresh data.
pub fn invalidate_graph_cache() {
    if let Some(cache) = GRAPH_CACHE.get() {
        if let Ok(mut guard) = cache.lock() {
            *guard = None;
            debug!("Graph cache invalidated");
        }
    }
}

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

/// Combined graph panel data — single response for the GraphPanel frontend.
#[derive(Debug, Clone, Serialize)]
pub struct GraphPanelDataDto {
    pub graph: GraphDataDto,
    pub components: Vec<ComponentDto>,
    pub orphans: Vec<OrphanDto>,
}

/// Shared adjacency result from a single pass through all blocks.
/// Used by graph, connected components, and orphan derivation to avoid triple scanning.
struct AdjacencyList {
    outgoing: HashMap<String, Vec<GraphEdgeDto>>,
    degree: HashMap<String, usize>,
    adjacency: HashMap<String, Vec<String>>,
    connected: HashSet<String>,
}

/// Build a shared adjacency structure from all blocks in a single pass.
/// Used by graph, connected components, and orphan derivation to avoid triple-scanning blocks.
fn build_adjacency_list(
    meta: &PageMetaIndex,
    store: &pkm_block::BlockStore,
) -> Result<AdjacencyList, String> {
    let mut outgoing: HashMap<String, Vec<GraphEdgeDto>> = HashMap::new();
    let mut degree: HashMap<String, usize> = HashMap::new();
    let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();
    let mut connected: HashSet<String> = HashSet::new();

    // Batch-load all blocks for all pages in a single query instead of N+1
    let page_paths: Vec<String> = meta.slug_to_path.values().cloned().collect();
    let blocks_by_page = store
        .get_blocks_by_pages(&page_paths)
        .map_err(|e| e.to_string())?;

    for (slug, page_path) in &meta.slug_to_path {
        if let Some(blocks) = blocks_by_page.get(page_path) {
            for block in blocks {
                let links = pkm_markdown::linker::extract_links(&block.content);
                for link in links {
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
                        // Degree: source always +1, target +1 only if different
                        *degree.entry(slug.clone()).or_default() += 1;
                        if target != *slug {
                            *degree.entry(target.clone()).or_default() += 1;
                        }
                        // Bidirectional adjacency for BFS (self-links are harmless)
                        adjacency
                            .entry(slug.clone())
                            .or_default()
                            .push(target.clone());
                        adjacency
                            .entry(target.clone())
                            .or_default()
                            .push(slug.clone());
                        // Track which slugs have any connection (for orphan detection)
                        connected.insert(slug.clone());
                        connected.insert(target);
                    }
                }
            }
        }
    }

    Ok(AdjacencyList {
        outgoing,
        degree,
        adjacency,
        connected,
    })
}

/// Build graph data from a pre-computed PageMetaIndex and AdjacencyList.
fn build_graph_data_from_meta(
    meta: &PageMetaIndex,
    adj: &AdjacencyList,
    vault_path: &str,
) -> Result<GraphDataDto, String> {
    let nodes: Vec<GraphNodeDto> = meta
        .slug_to_path
        .keys()
        .map(|slug| {
            let mut node = meta.get_node(slug);
            node.degree = adj.degree.get(slug).copied().unwrap_or(0);
            node
        })
        .collect();

    let edges: Vec<GraphEdgeDto> = adj.outgoing.values().flatten().cloned().collect();
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

/// Legacy wrapper — builds meta + adjacency internally, then delegates.
fn build_graph_data_from_store(
    store: &pkm_block::BlockStore,
    vault_path: &str,
) -> Result<GraphDataDto, String> {
    let meta = PageMetaIndex::from_store(store)?;
    let adj = build_adjacency_list(&meta, store)?;
    build_graph_data_from_meta(&meta, &adj, vault_path)
}

#[derive(Debug, Serialize)]
pub struct LinkTargetDto {
    pub page_path: Option<String>,
    pub slug: Option<String>,
    pub title: Option<String>,
}

#[tauri::command]
pub async fn get_graph_data(state: tauri::State<'_, AppState>) -> Result<GraphDataDto, String> {
    let vault_path_str = {
        let s = state.lock().map_err(|e| e.to_string())?;
        s.vault_path.to_string_lossy().to_string()
    };
    let db_path = {
        let s = state.lock().map_err(|e| e.to_string())?;
        s.db_path.clone()
    };

    info!("Building graph from SQLite: {}", vault_path_str);

    let data = tokio::task::spawn_blocking(move || {
        let store = pkm_block::BlockStore::open(&db_path).map_err(|e| e.to_string())?;
        build_graph_data_from_store(&store, &vault_path_str)
    })
    .await
    .map_err(|e| e.to_string())??;

    debug!("Found {} nodes, {} edges", data.node_count, data.edge_count);

    Ok(data)
}

/// Derive connected components from a pre-computed PageMetaIndex and AdjacencyList.
fn get_connected_components_from_meta(
    meta: &PageMetaIndex,
    adj: &AdjacencyList,
) -> Result<Vec<ComponentDto>, String> {
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
            if let Some(neighbors) = adj.adjacency.get(&current) {
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
                    node.degree = adj.adjacency.get(slug).map(|n| n.len()).unwrap_or(0);
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

/// Legacy wrapper — builds meta + adjacency internally, then delegates.
fn get_connected_components_from_store(
    store: &pkm_block::BlockStore,
) -> Result<Vec<ComponentDto>, String> {
    let meta = PageMetaIndex::from_store(store)?;
    let adj = build_adjacency_list(&meta, store)?;
    get_connected_components_from_meta(&meta, &adj)
}

#[tauri::command]
pub async fn get_connected_components(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<ComponentDto>, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let store = state.get_store().map_err(|e| e.to_string())?;
    get_connected_components_from_store(&store)
}

/// Derive orphans from a pre-computed PageMetaIndex and AdjacencyList.
fn get_orphaned_notes_from_meta(
    meta: &PageMetaIndex,
    adj: &AdjacencyList,
) -> Result<Vec<OrphanDto>, String> {
    let orphans: Vec<OrphanDto> = meta
        .all_slugs()
        .filter(|slug| !adj.connected.contains(*slug))
        .map(|slug| OrphanDto {
            slug: slug.to_string(),
            title: meta.slug_to_title.get(slug).cloned().unwrap_or_default(),
            path: meta.slug_to_path.get(slug).cloned().unwrap_or_default(),
        })
        .collect();

    Ok(orphans)
}

/// Legacy wrapper — builds meta + adjacency internally, then delegates.
fn get_orphaned_notes_from_store(store: &pkm_block::BlockStore) -> Result<Vec<OrphanDto>, String> {
    let meta = PageMetaIndex::from_store(store)?;
    let adj = build_adjacency_list(&meta, store)?;
    get_orphaned_notes_from_meta(&meta, &adj)
}

#[tauri::command]
pub async fn get_orphaned_notes(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<OrphanDto>, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let store = state.get_store().map_err(|e| e.to_string())?;
    get_orphaned_notes_from_store(&store)
}

/// Combined command: builds PageMetaIndex ONCE and derives graph, components, and orphans
/// from the same adjacency structure — replacing three separate DB scans.
///
/// Results are cached in-process; call `invalidate_graph_cache()` after any page mutation.
#[tauri::command]
pub async fn get_graph_panel_data(
    state: tauri::State<'_, AppState>,
) -> Result<GraphPanelDataDto, String> {
    // Check in-memory cache first
    if let Some(cached) = GRAPH_CACHE
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
    {
        info!("Returning cached graph data");
        return Ok(cached.clone());
    }

    let vault_path_str = {
        let s = state.lock().map_err(|e| e.to_string())?;
        s.vault_path.to_string_lossy().to_string()
    };
    let db_path = {
        let s = state.lock().map_err(|e| e.to_string())?;
        s.db_path.clone()
    };

    info!("Building graph panel data from SQLite: {}", vault_path_str);

    let result = tokio::task::spawn_blocking(move || {
        let store = pkm_block::BlockStore::open(&db_path).map_err(|e| e.to_string())?;
        let meta = PageMetaIndex::from_store(&store)?;
        let adj = build_adjacency_list(&meta, &store)?;

        let graph = build_graph_data_from_meta(&meta, &adj, &vault_path_str)?;
        let components = get_connected_components_from_meta(&meta, &adj)?;
        let orphans = get_orphaned_notes_from_meta(&meta, &adj)?;

        Ok::<_, String>((graph, components, orphans))
    })
    .await
    .map_err(|e| e.to_string())?;
    let (graph, components, orphans) = result?;

    let panel_data = GraphPanelDataDto {
        graph,
        components,
        orphans,
    };

    // Cache the result
    if let Ok(mut cache) = GRAPH_CACHE.get_or_init(|| Mutex::new(None)).lock() {
        *cache = Some(panel_data.clone());
    }

    debug!(
        "Found {} nodes, {} edges, {} components, {} orphans",
        panel_data.graph.node_count,
        panel_data.graph.edge_count,
        panel_data.components.len(),
        panel_data.orphans.len(),
    );

    Ok(panel_data)
}

#[tauri::command]
pub async fn resolve_link_target(
    target: String,
    state: tauri::State<'_, AppState>,
) -> Result<LinkTargetDto, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let store = state.get_store().map_err(|e| e.to_string())?;
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

#[cfg(test)]
mod tests;
