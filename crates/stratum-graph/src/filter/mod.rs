//! Pure graph filtering and capping policy, ported from the shipped
//! `src/components/GraphPanel/graphFilter.ts`.
//!
//! The policy is deterministic and unit-testable: explicit user caps always
//! win, "unlimited" vaults above the large-vault threshold get a safe default
//! cap, and edge selection is restricted to the kept node set.

use crate::data::{GraphEdge, GraphNode, GraphPanelData};
use std::collections::HashSet;

/// Vaults at or above this node count are considered "large".
pub const LARGE_VAULT_THRESHOLD: usize = 2000;

/// Safe default cap applied to large vaults when the user configured unlimited.
pub const DEFAULT_LARGE_VAULT_CAP: usize = 2000;

const _: () = assert!(DEFAULT_LARGE_VAULT_CAP <= LARGE_VAULT_THRESHOLD);

/// Compute the effective node cap to apply to the rendered set.
///
/// An explicit user cap (`node_cap > 0`) always wins. `node_cap == 0` means
/// unlimited for normal vaults, but applies [`DEFAULT_LARGE_VAULT_CAP`] once the
/// vault exceeds [`LARGE_VAULT_THRESHOLD`].
pub fn effective_node_cap(node_cap: usize, total_nodes: usize) -> usize {
    if node_cap > 0 {
        node_cap
    } else if total_nodes > LARGE_VAULT_THRESHOLD {
        DEFAULT_LARGE_VAULT_CAP
    } else {
        0
    }
}

/// Reduce `nodes` to at most `cap` entries, preferring higher-degree nodes.
///
/// The kept subset preserves the original source order, and ties are broken by
/// original index, so the result is stable across runs. A cap of `0`, or one at
/// least as large as the input, returns the nodes unchanged.
pub fn cap_nodes_by_degree(nodes: &[GraphNode], cap: usize) -> Vec<GraphNode> {
    if cap == 0 || cap >= nodes.len() {
        return nodes.to_vec();
    }
    let mut indices: Vec<usize> = (0..nodes.len()).collect();
    indices.sort_by(|&a, &b| {
        nodes[b]
            .degree
            .cmp(&nodes[a].degree)
            .then_with(|| a.cmp(&b))
    });
    indices.truncate(cap);
    indices.sort_unstable();
    indices.into_iter().map(|i| nodes[i].clone()).collect()
}

/// Filter `edges` to those whose source and target are both present in `ids`.
pub fn edges_within(edges: &[GraphEdge], ids: &HashSet<String>) -> Vec<GraphEdge> {
    edges
        .iter()
        .filter(|edge| ids.contains(&edge.source) && ids.contains(&edge.target))
        .cloned()
        .collect()
}

/// Apply the shipped filter order: cap the node set, then keep only edges that
/// stay inside it. Returns the filtered nodes and edges.
pub fn apply_cap(
    nodes: &[GraphNode],
    edges: &[GraphEdge],
    node_cap: usize,
) -> (Vec<GraphNode>, Vec<GraphEdge>) {
    let effective = effective_node_cap(node_cap, nodes.len());
    let kept = cap_nodes_by_degree(nodes, effective);
    let ids: HashSet<String> = kept.iter().map(|node| node.id.clone()).collect();
    let kept_edges = edges_within(edges, &ids);
    (kept, kept_edges)
}

/// Graph view mode selected by the toolbar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    /// Show the whole graph with the visibility toggles applied.
    Full,
    /// Show one connected component.
    Component,
    /// Show only orphaned notes.
    Orphans,
}

/// Visibility toggles from the graph settings panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GraphVisibility {
    /// Include notes that participate in at least one connection.
    pub show_connected: bool,
    /// Include notes with no incoming or outgoing connections.
    pub show_orphaned: bool,
    /// Render tag labels on hover (kept for settings parity; it does not filter).
    pub show_tags: bool,
}

impl Default for GraphVisibility {
    fn default() -> Self {
        Self {
            show_connected: true,
            show_orphaned: true,
            show_tags: true,
        }
    }
}

/// One filtering request mirroring the shipped `useGraphPanel` inputs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphFilterRequest {
    /// Toolbar view mode.
    pub view_mode: ViewMode,
    /// Selected component index (used by [`ViewMode::Component`]).
    pub selected_component: usize,
    /// Lowercased title/id/tag search text; empty disables the filter.
    pub search: String,
    /// Settings visibility toggles.
    pub visibility: GraphVisibility,
    /// Explicit user node cap (`0` means unlimited).
    pub node_cap: usize,
}

impl Default for GraphFilterRequest {
    fn default() -> Self {
        Self {
            view_mode: ViewMode::Full,
            selected_component: 0,
            search: String::new(),
            visibility: GraphVisibility::default(),
            node_cap: 0,
        }
    }
}

/// The filtered and capped render set plus the banner inputs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilteredGraph {
    /// Nodes that should be rendered.
    pub nodes: Vec<GraphNode>,
    /// Edges whose endpoints are both rendered.
    pub edges: Vec<GraphEdge>,
    /// Node count before the cap was applied.
    pub pre_cap_node_count: usize,
    /// Node count before progressive batching (the native panel renders in one
    /// pass, so this equals the capped node count).
    pub total_before_batch: usize,
    /// Cap value that was applied (`0` when no cap is active).
    pub effective_cap: usize,
    /// Whether the safe default cap (rather than an explicit user cap) is
    /// limiting the rendered set.
    pub default_cap_active: bool,
    /// Whether any cap is limiting the rendered set.
    pub node_cap_active: bool,
}

/// Apply the shipped view-mode, visibility, search and cap policy.
///
/// Mirrors `src/components/GraphPanel/useGraphPanel.ts`: component and orphan
/// modes ignore the search text, full mode applies the visibility toggles and
/// the title/id/tag search, and every mode then applies the node cap. The edge
/// set always ends up restricted to the kept node ids, matching the shipped
/// `graphDataProp` (`edgesWithin`) step.
pub fn filter_graph(panel: &GraphPanelData, request: &GraphFilterRequest) -> FilteredGraph {
    let orphan_paths: HashSet<&str> = panel
        .orphans
        .iter()
        .map(|orphan| orphan.path.as_str())
        .collect();
    let mut nodes = match request.view_mode {
        ViewMode::Component => match panel.components.get(request.selected_component) {
            Some(component) => component.nodes.clone(),
            None => Vec::new(),
        },
        ViewMode::Orphans => panel
            .graph
            .nodes
            .iter()
            .filter(|node| orphan_paths.contains(node.id.as_str()))
            .cloned()
            .collect(),
        ViewMode::Full => {
            let mut nodes: Vec<GraphNode> = panel
                .graph
                .nodes
                .iter()
                .filter(|node| {
                    let orphan = orphan_paths.contains(node.id.as_str());
                    match (
                        request.visibility.show_connected,
                        request.visibility.show_orphaned,
                    ) {
                        (false, false) => false,
                        (false, true) => orphan,
                        (true, false) => !orphan,
                        (true, true) => true,
                    }
                })
                .cloned()
                .collect();
            let search = request.search.trim().to_lowercase();
            if !search.is_empty() {
                nodes.retain(|node| {
                    node.title.to_lowercase().contains(&search)
                        || node.id.to_lowercase().contains(&search)
                        || node
                            .tags
                            .iter()
                            .any(|tag| tag.to_lowercase().contains(&search))
                });
            }
            nodes
        }
    };

    let pre_cap_node_count = nodes.len();
    let effective_cap = effective_node_cap(request.node_cap, pre_cap_node_count);
    let node_cap_active = effective_cap > 0 && pre_cap_node_count > effective_cap;
    if node_cap_active {
        nodes = cap_nodes_by_degree(&nodes, effective_cap);
    }
    let default_cap_active = node_cap_active && request.node_cap == 0;
    let total_before_batch = nodes.len();

    let edges = if request.view_mode == ViewMode::Orphans {
        Vec::new()
    } else {
        let ids: HashSet<String> = nodes.iter().map(|node| node.id.clone()).collect();
        edges_within(&panel.graph.edges, &ids)
    };

    FilteredGraph {
        nodes,
        edges,
        pre_cap_node_count,
        total_before_batch,
        effective_cap: if node_cap_active { effective_cap } else { 0 },
        default_cap_active,
        node_cap_active,
    }
}

#[cfg(test)]
mod tests;
