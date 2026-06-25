//! Graph visualization commands — expose note-level graph data to the frontend.

use crate::commands::vault::AppState;
use serde::Serialize;
use std::collections::{HashMap, HashSet};

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

    // Build slug → (path, page_frontmatter) and a lookup for link resolution
    let mut slug_to_path: HashMap<String, String> = HashMap::new();
    let mut slug_to_title: HashMap<String, String> = HashMap::new();
    let mut slug_to_tags: HashMap<String, Vec<String>> = HashMap::new();
    // Map from display title → slug for resolving [[Title]] links
    let mut title_to_slug: HashMap<String, String> = HashMap::new();

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

    // Build edges by scanning blocks for [[wiki-links]]
    let mut outgoing: HashMap<String, Vec<GraphEdgeDto>> = HashMap::new();
    let mut degree: HashMap<String, usize> = HashMap::new();

    for (slug, page_path) in &slug_to_path {
        if let Ok(blocks) = store.get_blocks_by_page(page_path) {
            for block in &blocks {
                let links = pkm_markdown::linker::extract_links(&block.content);
                for link in links {
                    // Resolve the link target to a slug
                    let target_slug = resolve_slug(&link.target, &slug_to_path, &title_to_slug);
                    if let Some(target) = target_slug {
                        if target != *slug {
                            outgoing
                                .entry(slug.clone())
                                .or_default()
                                .push(GraphEdgeDto {
                                    source: slug.clone(),
                                    target: target.clone(),
                                    label: link.display_text.clone(),
                                });
                            *degree.entry(slug.clone()).or_default() += 1;
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
            let title = slug_to_title.get(&slug).cloned().unwrap_or_default();
            let tags = slug_to_tags.get(&slug).cloned().unwrap_or_default();
            let deg = degree.get(&slug).copied().unwrap_or(0usize);
            GraphNodeDto {
                id: slug,
                title,
                path: path.clone(),
                tags,
                degree: deg,
            }
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

/// Resolve a wiki-link target to an existing note slug.
/// Tries: (1) slugified link target, (2) as-is, (3) title lookup, (4) slugified title.
fn resolve_slug(
    target: &str,
    slug_to_path: &HashMap<String, String>,
    title_to_slug: &HashMap<String, String>,
) -> Option<String> {
    let slugified = target.replace(' ', "-").to_lowercase();
    // Try slugified version
    if slug_to_path.contains_key(&slugified) {
        return Some(slugified);
    }
    // Try as-is
    if slug_to_path.contains_key(target) {
        return Some(target.to_string());
    }
    // Try as title
    let lower = target.to_lowercase();
    if let Some(slug) = title_to_slug.get(&lower) {
        return Some(slug.clone());
    }
    // Try as title → slugify
    if slug_to_path.contains_key(&lower) {
        return Some(lower);
    }
    None
}

#[tauri::command]
pub async fn get_graph_data(state: tauri::State<'_, AppState>) -> Result<GraphDataDto, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let vault_path_str = state.vault_path.to_string_lossy().to_string();

    eprintln!(
        "[stratum:graph] Building graph from SQLite: {}",
        vault_path_str
    );

    let store = pkm_block::BlockStore::open(&state.db_path).map_err(|e| e.to_string())?;
    let data = build_graph_data_from_store(&store, &vault_path_str)?;

    eprintln!(
        "[stratum:graph] Found {} nodes, {} edges",
        data.node_count, data.edge_count
    );

    Ok(data)
}

fn get_connected_components_from_store(
    store: &pkm_block::BlockStore,
) -> Result<Vec<ComponentDto>, String> {
    let paths = store.list_pages().map_err(|e| e.to_string())?;
    let mut slug_to_path: HashMap<String, String> = HashMap::new();
    let mut slug_to_title: HashMap<String, String> = HashMap::new();
    let mut slug_to_tags: HashMap<String, Vec<String>> = HashMap::new();
    let mut title_to_slug: HashMap<String, String> = HashMap::new();

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

    // Build adjacency list
    let mut adj: HashMap<String, Vec<String>> = HashMap::new();
    for (slug, page_path) in &slug_to_path {
        if let Ok(blocks) = store.get_blocks_by_page(page_path) {
            for block in &blocks {
                let links = pkm_markdown::linker::extract_links(&block.content);
                for link in links {
                    if let Some(target) = resolve_slug(&link.target, &slug_to_path, &title_to_slug)
                    {
                        if target != *slug {
                            adj.entry(slug.clone()).or_default().push(target.clone());
                            adj.entry(target).or_default().push(slug.clone());
                        }
                    }
                }
            }
        }
    }

    // BFS for connected components
    let mut visited: HashSet<String> = HashSet::new();
    let mut components: Vec<Vec<String>> = Vec::new();

    for slug in slug_to_path.keys() {
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
                    let degree = adj.get(slug).map(|n| n.len()).unwrap_or(0);
                    GraphNodeDto {
                        id: slug.clone(),
                        title: slug_to_title.get(slug).cloned().unwrap_or_default(),
                        path: slug_to_path.get(slug).cloned().unwrap_or_default(),
                        tags: slug_to_tags.get(slug).cloned().unwrap_or_default(),
                        degree,
                    }
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
    let paths = store.list_pages().map_err(|e| e.to_string())?;
    let mut slug_to_path: HashMap<String, String> = HashMap::new();
    let mut slug_to_title: HashMap<String, String> = HashMap::new();
    let mut title_to_slug: HashMap<String, String> = HashMap::new();

    for path in &paths {
        let slug = slug_from_path(path);
        slug_to_path.insert(slug.clone(), path.clone());
        let fm = store.get_page(path).ok().flatten();
        let title = fm
            .as_ref()
            .and_then(|f| f.title.clone())
            .unwrap_or_else(|| slug.replace('-', " "));
        slug_to_title.insert(slug.clone(), title.clone());
        title_to_slug.insert(title.to_lowercase(), slug);
    }

    // Track which slugs have any connections
    let mut connected: HashSet<String> = HashSet::new();

    for (slug, page_path) in &slug_to_path {
        if let Ok(blocks) = store.get_blocks_by_page(page_path) {
            for block in &blocks {
                let links = pkm_markdown::linker::extract_links(&block.content);
                for link in links {
                    if let Some(target) = resolve_slug(&link.target, &slug_to_path, &title_to_slug)
                    {
                        if target != *slug {
                            connected.insert(slug.clone());
                            connected.insert(target);
                        }
                    }
                }
            }
        }
    }

    let orphans: Vec<OrphanDto> = slug_to_path
        .keys()
        .filter(|slug| !connected.contains(*slug))
        .map(|slug| OrphanDto {
            slug: slug.clone(),
            title: slug_to_title.get(slug).cloned().unwrap_or_default(),
            path: slug_to_path.get(slug).cloned().unwrap_or_default(),
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
    let paths = store.list_pages().map_err(|e| e.to_string())?;

    let mut slug_to_path: HashMap<String, String> = HashMap::new();
    let mut title_to_slug: HashMap<String, String> = HashMap::new();
    let mut slug_to_title: HashMap<String, String> = HashMap::new();

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
    }

    let resolved_slug = resolve_slug(&target, &slug_to_path, &title_to_slug);
    let result = resolved_slug.map(|slug| {
        let page_path = slug_to_path.get(&slug).cloned();
        let title = slug_to_title.get(&slug).cloned();
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
