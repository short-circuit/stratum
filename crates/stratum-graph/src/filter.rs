//! Pure graph filtering and capping policy, ported from the shipped
//! `src/components/GraphPanel/graphFilter.ts`.
//!
//! The policy is deterministic and unit-testable: explicit user caps always
//! win, "unlimited" vaults above the large-vault threshold get a safe default
//! cap, and edge selection is restricted to the kept node set.

use crate::data::{GraphEdge, GraphNode};
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
