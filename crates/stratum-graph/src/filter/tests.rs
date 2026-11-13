//! Unit tests for the graph filter and cap policy.
//!
//! Extracted from `filter/mod.rs` so the module stays inside the repository
//! file-length limit.

use super::*;
use crate::data::{Component, Orphan};

fn node(id: &str, degree: usize) -> GraphNode {
    GraphNode {
        id: id.into(),
        title: id.into(),
        path: format!("{id}.md"),
        tags: vec![],
        degree,
    }
}

fn edge(source: &str, target: &str) -> GraphEdge {
    GraphEdge {
        source: source.into(),
        target: target.into(),
        label: None,
    }
}

#[test]
fn explicit_caps_win_at_any_size() {
    assert_eq!(effective_node_cap(500, 10), 500);
    assert_eq!(effective_node_cap(500, 10_000), 500);
    assert_eq!(effective_node_cap(10_000, 10_000), 10_000);
}

#[test]
fn unlimited_normal_vaults_stay_uncapped() {
    assert_eq!(effective_node_cap(0, 0), 0);
    assert_eq!(effective_node_cap(0, 5), 0);
    assert_eq!(effective_node_cap(0, LARGE_VAULT_THRESHOLD), 0);
}

#[test]
fn large_unlimited_vaults_get_the_safe_default() {
    assert_eq!(
        effective_node_cap(0, LARGE_VAULT_THRESHOLD + 1),
        DEFAULT_LARGE_VAULT_CAP
    );
    assert_eq!(effective_node_cap(0, 10_000), DEFAULT_LARGE_VAULT_CAP);
}

#[test]
fn cap_keeps_highest_degree_nodes_in_source_order() {
    let nodes = vec![
        node("a", 1),
        node("b", 5),
        node("c", 3),
        node("d", 0),
        node("e", 2),
    ];
    let kept = cap_nodes_by_degree(&nodes, 3);
    assert_eq!(
        kept.iter().map(|n| n.id.as_str()).collect::<Vec<_>>(),
        vec!["b", "c", "e"]
    );
    assert_eq!(cap_nodes_by_degree(&nodes, 0).len(), 5);
    assert_eq!(cap_nodes_by_degree(&nodes, 99).len(), 5);
}

#[test]
fn cap_is_deterministic_for_ties() {
    let nodes = vec![node("a", 2), node("b", 2), node("c", 2)];
    let first = cap_nodes_by_degree(&nodes, 2);
    let second = cap_nodes_by_degree(&nodes, 2);
    assert_eq!(first, second);
    assert_eq!(
        first.iter().map(|n| n.id.as_str()).collect::<Vec<_>>(),
        vec!["a", "b"]
    );
}

#[test]
fn edges_require_both_endpoints() {
    let edges = vec![
        edge("a", "b"),
        edge("b", "c"),
        edge("a", "x"),
        edge("x", "a"),
    ];
    let ids: HashSet<String> = ["a", "b", "c"].iter().map(|s| s.to_string()).collect();
    assert_eq!(
        edges_within(&edges, &ids),
        vec![edge("a", "b"), edge("b", "c")]
    );
    assert!(edges_within(&edges, &HashSet::new()).is_empty());
}

#[test]
fn apply_cap_keeps_edges_consistent() {
    let nodes = vec![node("a", 5), node("b", 4), node("c", 0)];
    let edges = vec![edge("a", "b"), edge("a", "c"), edge("b", "c")];
    let (kept, kept_edges) = apply_cap(&nodes, &edges, 2);
    assert_eq!(kept.len(), 2);
    assert_eq!(kept_edges, vec![edge("a", "b")]);
}

fn panel() -> GraphPanelData {
    let mut nodes = vec![
        node("a", 2),
        node("b", 1),
        node("c", 1),
        node("d", 1),
        node("e", 1),
        node("orphan-1", 0),
    ];
    nodes[4].tags = vec!["project".to_string()];
    let edges = vec![
        edge("a", "b"),
        edge("a", "c"),
        edge("d", "e"),
        edge("a", "orphan-1"),
    ];
    let components = vec![
        Component {
            nodes: vec![node("a", 2), node("b", 1), node("c", 1)],
            size: 3,
        },
        Component {
            nodes: vec![node("d", 1), node("e", 1)],
            size: 2,
        },
    ];
    let orphans = vec![
        Orphan {
            slug: "orphan-1".to_string(),
            title: "Orphan 1".to_string(),
            path: "orphan-1".to_string(),
        },
        Orphan {
            slug: "orphan-2".to_string(),
            title: "Orphan 2".to_string(),
            path: "orphan-2".to_string(),
        },
    ];
    GraphPanelData {
        graph: crate::data::GraphData::new(nodes, edges, "/fixture"),
        components,
        orphans,
    }
}

fn ids(nodes: &[GraphNode]) -> Vec<&str> {
    nodes.iter().map(|node| node.id.as_str()).collect()
}

#[test]
fn full_mode_keeps_every_node_and_edge_by_default() {
    let panel = panel();
    let filtered = filter_graph(&panel, &GraphFilterRequest::default());
    assert_eq!(filtered.nodes.len(), 6);
    assert_eq!(filtered.edges.len(), 4);
    assert_eq!(filtered.pre_cap_node_count, 6);
    assert!(!filtered.node_cap_active);
    assert!(!filtered.default_cap_active);
    assert_eq!(filtered.effective_cap, 0);
}

#[test]
fn visibility_toggles_split_connected_and_orphaned_notes() {
    let panel = panel();
    let only_orphaned = filter_graph(
        &panel,
        &GraphFilterRequest {
            visibility: GraphVisibility {
                show_connected: false,
                show_orphaned: true,
                show_tags: true,
            },
            ..GraphFilterRequest::default()
        },
    );
    assert_eq!(ids(&only_orphaned.nodes), vec!["orphan-1"]);

    let only_connected = filter_graph(
        &panel,
        &GraphFilterRequest {
            visibility: GraphVisibility {
                show_connected: true,
                show_orphaned: false,
                show_tags: true,
            },
            ..GraphFilterRequest::default()
        },
    );
    assert_eq!(ids(&only_connected.nodes), vec!["a", "b", "c", "d", "e"]);
    assert_eq!(only_connected.edges.len(), 3);

    let hidden = filter_graph(
        &panel,
        &GraphFilterRequest {
            visibility: GraphVisibility {
                show_connected: false,
                show_orphaned: false,
                show_tags: true,
            },
            ..GraphFilterRequest::default()
        },
    );
    assert!(hidden.nodes.is_empty());
    assert!(hidden.edges.is_empty());
}

#[test]
fn search_matches_title_id_and_tags_then_keeps_internal_edges() {
    let panel = panel();
    let filtered = filter_graph(
        &panel,
        &GraphFilterRequest {
            search: "PROJECT".to_string(),
            ..GraphFilterRequest::default()
        },
    );
    assert_eq!(ids(&filtered.nodes), vec!["e"]);
    assert!(filtered.edges.is_empty());
}

#[test]
fn component_mode_selects_one_component_and_its_edges() {
    let panel = panel();
    let filtered = filter_graph(
        &panel,
        &GraphFilterRequest {
            view_mode: ViewMode::Component,
            selected_component: 1,
            ..GraphFilterRequest::default()
        },
    );
    assert_eq!(ids(&filtered.nodes), vec!["d", "e"]);
    assert_eq!(filtered.edges, vec![edge("d", "e")]);
    let missing = filter_graph(
        &panel,
        &GraphFilterRequest {
            view_mode: ViewMode::Component,
            selected_component: 9,
            ..GraphFilterRequest::default()
        },
    );
    assert!(missing.nodes.is_empty());
}

#[test]
fn orphan_mode_uses_paths_and_drops_edges() {
    let panel = panel();
    let filtered = filter_graph(
        &panel,
        &GraphFilterRequest {
            view_mode: ViewMode::Orphans,
            ..GraphFilterRequest::default()
        },
    );
    assert_eq!(ids(&filtered.nodes), vec!["orphan-1"]);
    assert!(filtered.edges.is_empty());
}

#[test]
fn caps_report_explicit_and_default_banner_inputs() {
    let panel = panel();
    let explicit = filter_graph(
        &panel,
        &GraphFilterRequest {
            node_cap: 2,
            ..GraphFilterRequest::default()
        },
    );
    assert_eq!(explicit.nodes.len(), 2);
    assert!(explicit.node_cap_active);
    assert!(!explicit.default_cap_active);
    assert_eq!(explicit.effective_cap, 2);
    assert_eq!(explicit.pre_cap_node_count, 6);
    assert!(explicit.edges.iter().all(|edge| explicit
        .nodes
        .iter()
        .any(|node| node.id == edge.source)
        && explicit.nodes.iter().any(|node| node.id == edge.target)));
}
