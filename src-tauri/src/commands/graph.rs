//! Graph visualization commands — expose note-level graph data to the frontend.

use crate::commands::vault::AppState;
use serde::Serialize;

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

#[tauri::command]
pub async fn get_graph_data(state: tauri::State<'_, AppState>) -> Result<GraphDataDto, String> {
    let mut vault = state.lock().map_err(|e| e.to_string())?;
    let vault_path_str = vault.vault_path.to_string_lossy().to_string();
    let engine = vault.ensure_index()?;

    eprintln!("[stratum:graph] Scanning vault: {}", vault_path_str);

    // Rebuild to ensure graph is current
    engine
        .rebuild_all(None)
        .map_err(|e| format!("Graph rebuild failed: {}", e))?;

    let graph = engine.get_graph();
    let all_nodes = graph.all_nodes();

    eprintln!(
        "[stratum:graph] Found {} nodes, {} edges",
        graph.node_count(),
        graph.edge_count()
    );

    // Build node DTOs with degree computed
    let nodes: Vec<GraphNodeDto> = all_nodes
        .iter()
        .map(|n| {
            let degree =
                graph.get_outgoing_links(&n.slug).len() + graph.get_backlinks(&n.slug).len();
            GraphNodeDto {
                id: n.slug.clone(),
                title: n.title.clone(),
                path: n.path.clone(),
                tags: n.tags.clone(),
                degree,
            }
        })
        .collect();

    // Build edge DTOs (deduplicate reversed edges for undirected rendering)
    let mut edges: Vec<GraphEdgeDto> = Vec::new();
    for node in &nodes {
        for edge in graph.get_outgoing_links(&node.id) {
            edges.push(GraphEdgeDto {
                source: edge.source.clone(),
                target: edge.target.clone(),
                label: edge.display_text.clone(),
            });
        }
    }

    let node_count = nodes.len();
    let edge_count = edges.len();

    Ok(GraphDataDto {
        nodes,
        edges,
        node_count,
        edge_count,
        vault_path: vault_path_str,
    })
}

#[tauri::command]
pub async fn get_connected_components(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<ComponentDto>, String> {
    let mut vault = state.lock().map_err(|e| e.to_string())?;
    let engine = vault.ensure_index()?;

    let graph = engine.get_graph();
    let component_groups = graph.connected_components();

    let mut components = Vec::new();
    for group in component_groups {
        let mut nodes = Vec::new();
        for slug in &group {
            if let Some(node) = graph.get_node(slug) {
                let degree = graph.get_outgoing_links(slug).len() + graph.get_backlinks(slug).len();
                nodes.push(GraphNodeDto {
                    id: node.slug.clone(),
                    title: node.title.clone(),
                    path: node.path.clone(),
                    tags: node.tags.clone(),
                    degree,
                });
            }
        }
        let size = nodes.len();
        components.push(ComponentDto { nodes, size });
    }

    // Sort largest components first
    components.sort_by_key(|c| std::cmp::Reverse(c.size));

    Ok(components)
}

#[tauri::command]
pub async fn get_orphaned_notes(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<OrphanDto>, String> {
    let mut vault = state.lock().map_err(|e| e.to_string())?;
    let engine = vault.ensure_index()?;

    let graph = engine.get_graph();
    let orphan_slugs = graph.find_orphaned_notes();

    let orphans: Vec<OrphanDto> = orphan_slugs
        .iter()
        .filter_map(|slug| {
            graph.get_node(slug).map(|n| OrphanDto {
                slug: n.slug.clone(),
                title: n.title.clone(),
                path: n.path.clone(),
            })
        })
        .collect();

    Ok(orphans)
}

#[tauri::command]
pub async fn rebuild_graph(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let mut vault = state.lock().map_err(|e| e.to_string())?;
    let engine = vault.ensure_index()?;

    let _notes = engine
        .rebuild_all(None)
        .map_err(|e| format!("Graph rebuild failed: {}", e))?;

    let graph = engine.get_graph();
    Ok(format!(
        "Indexed {} notes with {} edges",
        graph.node_count(),
        graph.edge_count()
    ))
}
