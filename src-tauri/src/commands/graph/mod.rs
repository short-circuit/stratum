//! Graph visualization commands — expose note-level graph data to the frontend.
//!
//! Node identity is the **vault-relative page path** (one node per path), not
//! the basename slug. Two notes with the same basename in different folders are
//! distinct nodes, so duplicate slugs never collide.

mod meta;

use crate::commands::graph::meta::PageMetaIndex;
use crate::commands::vault::AppState;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, OnceLock};
use tracing::{debug, info};

/// In-memory cache for graph panel data, keyed by vault path so concurrent
/// vaults (multiple windows, tests running in parallel) never observe each
/// other's graphs. Invalidated per-vault by `invalidate_graph_cache()` after
/// any page/block mutation; a full `clear_graph_cache()` drops all entries.
static GRAPH_CACHE: OnceLock<Mutex<HashMap<String, Arc<GraphPanelDataDto>>>> = OnceLock::new();

/// Clear the in-memory cache for a specific vault.
/// Call this after any page or block mutation in that vault so the next graph
/// view rebuilds fresh data.
pub fn invalidate_graph_cache() {
    if let Some(cache) = GRAPH_CACHE.get() {
        if let Ok(mut guard) = cache.lock() {
            guard.clear();
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

/// Build a shared adjacency structure from the authoritative `links` table.
/// Used by graph, connected components, and orphan derivation to avoid triple-scanning.
///
/// The `links` table is populated by `reconcile_page_links` on every block/page write
/// and self-healed at boot (`sync_filesystem_to_db`), so graph edges are read directly
/// from it instead of re-reading and re-parsing every block's content on each load.
fn build_adjacency_list(
    meta: &PageMetaIndex,
    store: &pkm_block::BlockStore,
) -> Result<AdjacencyList, String> {
    let mut outgoing: HashMap<String, Vec<GraphEdgeDto>> = HashMap::new();
    let mut degree: HashMap<String, usize> = HashMap::new();
    let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();
    let mut connected: HashSet<String> = HashSet::new();

    // The `links` table is the authoritative graph-edge source: every write path
    // (`save_blocks`, `save_page`, `sync_page_from_disk`, reindex, marker ops)
    // reconciles it transactionally, and boot-time backfill (`sync_filesystem_to_db`)
    // heals any pre-existing vault that predates that wiring. So the graph path is a
    // pure read here — no re-reading/re-parsing block content, and no per-read heal
    // (which would not converge for pages that legitimately have zero links).
    let edges = store.get_page_ref_edges().map_err(|e| e.to_string())?;

    for (src_path, target_ref) in &edges {
        // `src_path` is already the authoritative page path (from the `links`
        // table), so no slug lookup is needed for the source.
        // Resolve the stored target against the CURRENT page set — a target
        // written as raw `[[Text]]` (page did not exist at write time) resolves
        // once that page exists; dead targets resolve to nothing and are dropped.
        // Ambiguous targets (multiple paths share a slug/title) also resolve to
        // nothing and are dropped.
        // Belt-and-braces: stale pre-fix rows may store bracket-wrapped targets
        // (`[[Text]]`); strip the wrapping before resolution.
        let clean_ref = crate::commands::graph::meta::strip_wiki_brackets(target_ref);
        let Some(target_path) = meta.resolve_path(&clean_ref) else {
            continue;
        };

        outgoing
            .entry(src_path.clone())
            .or_default()
            .push(GraphEdgeDto {
                source: src_path.clone(),
                target: target_path.clone(),
                label: None, // not consumed by the frontend; reserved
            });

        // Degree: source always +1, target +1 only if different (self-links never double-count)
        *degree.entry(src_path.clone()).or_default() += 1;
        if target_path != *src_path {
            *degree.entry(target_path.clone()).or_default() += 1;
        }

        // Bidirectional adjacency for BFS (self-links are harmless)
        adjacency
            .entry(src_path.clone())
            .or_default()
            .push(target_path.clone());
        adjacency
            .entry(target_path.clone())
            .or_default()
            .push(src_path.clone());

        connected.insert(src_path.clone());
        connected.insert(target_path.clone());
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
        .all_paths()
        .map(|path| {
            let mut node = meta.get_node(path);
            node.degree = adj.degree.get(path).copied().unwrap_or(0);
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
    // BFS for connected components over path-keyed adjacency.
    let mut visited: HashSet<String> = HashSet::new();
    let mut components: Vec<Vec<String>> = Vec::new();

    for path in meta.all_paths() {
        if visited.contains(path) {
            continue;
        }
        let mut component = Vec::new();
        let mut queue = Vec::new();
        queue.push(path.to_string());
        visited.insert(path.to_string());

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
                .map(|path| {
                    let mut node = meta.get_node(path);
                    node.degree = adj.adjacency.get(path).map(|n| n.len()).unwrap_or(0);
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
        .all_paths()
        .filter(|path| !adj.connected.contains(*path))
        .map(|path| OrphanDto {
            slug: meta.slug_for_path(path).unwrap_or_default().to_string(),
            title: meta.title_for_path(path).unwrap_or_default().to_string(),
            path: path.to_string(),
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
) -> Result<Arc<GraphPanelDataDto>, String> {
    let vault_path_str = {
        let s = state.lock().map_err(|e| e.to_string())?;
        s.vault_path.to_string_lossy().to_string()
    };
    // Check in-memory cache first (keyed by vault path so concurrent vaults /
    // parallel test binaries never observe each other's graphs).
    if let Some(cached) = GRAPH_CACHE
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .map_err(|e| e.to_string())?
        .get(&vault_path_str)
    {
        info!("Returning cached graph data");
        // Cheap Arc clone — the DTO is immutable once cached, so no deep copy.
        return Ok(Arc::clone(cached));
    }

    let db_path = {
        let s = state.lock().map_err(|e| e.to_string())?;
        s.db_path.clone()
    };

    info!("Building graph panel data from SQLite: {}", vault_path_str);

    let cache_key = vault_path_str.clone();
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

    let panel_data = Arc::new(GraphPanelDataDto {
        graph,
        components,
        orphans,
    });

    // Cache the result (keyed by vault path) — store the Arc so cache hits only
    // do a cheap Arc clone instead of deep-copying the (potentially multi-MB) DTO.
    if let Ok(mut cache) = GRAPH_CACHE
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
    {
        cache.insert(cache_key, Arc::clone(&panel_data));
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

    let resolved_path = meta.resolve_path(&target);
    let result = resolved_path.map(|path| {
        let title = meta.title_for_path(&path).map(|s| s.to_string());
        LinkTargetDto {
            page_path: Some(path.clone()),
            slug: meta.slug_for_path(&path).map(|s| s.to_string()),
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
