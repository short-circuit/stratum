//! Graph data model mirroring the shipped `GraphPanelDataDto` shape.
//!
//! The crate deliberately mirrors the DTOs instead of depending on
//! `stratum-app`; hosts can construct [`GraphData`] directly from the service
//! layer response. The helpers here reproduce the shipped React behaviour:
//! deterministic seed positions (the same hash formula as `GraphCanvas.tsx`),
//! the eight-colour palette accessor, and the deterministic fixture generator
//! used by the golden runner and contract tests.

use crate::math::Vec3;

/// Node colours in the same order as the shipped `GraphCanvas.tsx` palette.
pub const PALETTE: [&str; 8] = [
    "#fbbf24", "#60a5fa", "#34d399", "#f472b6", "#a78bfa", "#fb923c", "#2dd4bf", "#e879f9",
];

/// A note-level graph node, ready for force-directed layout rendering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphNode {
    /// Stable node identity (vault-relative page path).
    pub id: String,
    /// Display title.
    pub title: String,
    /// Vault-relative page path.
    pub path: String,
    /// Tags attached to the page.
    pub tags: Vec<String>,
    /// Number of incident edges.
    pub degree: usize,
}

/// A directed edge between two nodes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphEdge {
    /// Source node id.
    pub source: String,
    /// Target node id.
    pub target: String,
    /// Optional edge label (unused by the shipped renderer, reserved).
    pub label: Option<String>,
}

/// Full graph data for force-directed layout rendering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphData {
    /// Graph nodes in source order.
    pub nodes: Vec<GraphNode>,
    /// Graph edges in source order.
    pub edges: Vec<GraphEdge>,
    /// Number of nodes (mirrors the DTO field).
    pub node_count: usize,
    /// Number of edges (mirrors the DTO field).
    pub edge_count: usize,
    /// Vault path that produced the graph.
    pub vault_path: String,
}

impl GraphData {
    /// Build graph data from nodes and edges, deriving the DTO count fields.
    pub fn new(
        nodes: Vec<GraphNode>,
        edges: Vec<GraphEdge>,
        vault_path: impl Into<String>,
    ) -> Self {
        Self {
            node_count: nodes.len(),
            edge_count: edges.len(),
            nodes,
            edges,
            vault_path: vault_path.into(),
        }
    }

    /// Return the deterministic seed position for every node.
    ///
    /// Mirrors `GraphCanvas.tsx`: the shipped panel only seeds a coordinate when
    /// the DTO omits it, so this helper is the "no stored position" path.
    pub fn seed_positions(&self) -> Vec<Vec3> {
        self.nodes
            .iter()
            .map(|node| seed_position(&node.id))
            .collect()
    }
}

/// A connected component (group of interlinked notes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Component {
    /// Nodes in the component.
    pub nodes: Vec<GraphNode>,
    /// Component size.
    pub size: usize,
}

/// Orphaned note info.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Orphan {
    /// Basename slug.
    pub slug: String,
    /// Display title.
    pub title: String,
    /// Vault-relative path.
    pub path: String,
}

/// Combined graph panel data — single response for the GraphPanel surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphPanelData {
    /// Graph nodes and edges.
    pub graph: GraphData,
    /// Connected components.
    pub components: Vec<Component>,
    /// Orphaned notes.
    pub orphans: Vec<Orphan>,
}

/// Sum the UTF-16 code units of `text` the way the shipped JavaScript does.
fn char_code_sum(text: &str, multiplier: i64) -> i64 {
    text.encode_utf16()
        .map(|unit| i64::from(unit) * multiplier)
        .sum()
}

/// Deterministic seed position for a node id.
///
/// This is the exact formula shipped in `GraphCanvas.tsx`:
/// `z = (sum*7+13)%120-60`, `x = (sum(c*3)*11+7)%120-60`,
/// `y = (sum(c*7)*13+3)%120-60`.
pub fn seed_position(id: &str) -> Vec3 {
    let z_hash = char_code_sum(id, 1);
    let x_hash = char_code_sum(id, 3);
    let y_hash = char_code_sum(id, 7);
    Vec3::new(
        ((x_hash * 11 + 7) % 120 - 60) as f32,
        ((y_hash * 13 + 3) % 120 - 60) as f32,
        ((z_hash * 7 + 13) % 120 - 60) as f32,
    )
}

/// Parse a `#rrggbb` palette entry into linear-ish sRGB bytes.
fn hex_to_rgb(hex: &str) -> [u8; 3] {
    let digits = hex.trim_start_matches('#');
    if digits.len() != 6 {
        return [255, 255, 255];
    }
    let parse =
        |range: std::ops::Range<usize>| u8::from_str_radix(&digits[range], 16).unwrap_or(255);
    [parse(0..2), parse(2..4), parse(4..6)]
}

/// Shipped node colour accessor: first tag when present, otherwise the id.
///
/// Returns sRGB bytes for the palette slot selected by the character-code sum.
pub fn node_color(node: &GraphNode) -> [u8; 3] {
    let key = node
        .tags
        .first()
        .map(String::as_str)
        .unwrap_or(node.id.as_str());
    let sum = char_code_sum(key, 1);
    let index = sum.rem_euclid(PALETTE.len() as i64) as usize;
    hex_to_rgb(PALETTE[index])
}

/// Colour used for nodes that are dimmed by a highlight selection.
pub const DIM_COLOR: [u8; 4] = [200, 200, 200, 31];

/// Convert sRGB bytes into the linear values expected by an sRGB swap-chain.
pub fn srgb_to_linear(color: [u8; 3]) -> [f32; 3] {
    let channel = |value: u8| {
        let v = f32::from(value) / 255.0;
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    [channel(color[0]), channel(color[1]), channel(color[2])]
}

/// Generate a deterministic fixture graph with `count` nodes.
///
/// The fixture is intentionally cheap and stable: ids are `note-NNNN`, tags are
/// assigned round-robin, and edges form a deterministic sparse ring with
/// cross-links. `seed` shifts the edge pattern but never the node identity.
pub fn fixture(count: usize, seed: u64) -> GraphData {
    let mut nodes = Vec::with_capacity(count);
    let mut edges = Vec::new();
    for i in 0..count {
        let id = format!("note-{i:04}");
        let tags = if i % 5 == 0 {
            vec![format!("tag-{}", i % 8)]
        } else {
            Vec::new()
        };
        nodes.push(GraphNode {
            title: format!("Note {i}"),
            path: format!("notes/{id}.md"),
            id,
            tags,
            degree: 0,
        });
    }
    if count > 1 {
        let stride = 7_u64 + seed % 5;
        for i in 0..count {
            let target = ((i as u64 + stride) % count as u64) as usize;
            if target != i {
                edges.push(GraphEdge {
                    source: nodes[i].id.clone(),
                    target: nodes[target].id.clone(),
                    label: None,
                });
                nodes[i].degree += 1;
                nodes[target].degree += 1;
            }
            if i % 3 == 0 {
                let cross = ((i as u64 * 13 + 11 + seed) % count as u64) as usize;
                if cross != i {
                    edges.push(GraphEdge {
                        source: nodes[i].id.clone(),
                        target: nodes[cross].id.clone(),
                        label: None,
                    });
                    nodes[i].degree += 1;
                    nodes[cross].degree += 1;
                }
            }
        }
    }
    GraphData::new(nodes, edges, "/fixture")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_positions_match_shipped_hash() {
        assert_eq!(seed_position("alpha"), Vec3::new(1.0, 41.0, -21.0));
        assert_eq!(
            seed_position("[[Page Name]]"),
            Vec3::new(25.0, -31.0, -45.0)
        );
    }

    #[test]
    fn palette_matches_shipped_accessor() {
        let node = GraphNode {
            id: "alpha".into(),
            title: "Alpha".into(),
            path: "alpha.md".into(),
            tags: vec![],
            degree: 0,
        };
        assert_eq!(node_color(&node), [0x2d, 0xd4, 0xbf]);
        let mut tagged = node.clone();
        tagged.tags = vec!["tag-0".into()];
        assert_eq!(node_color(&tagged), [0x60, 0xa5, 0xfa]);
    }

    #[test]
    fn fixture_is_deterministic_and_bounded() {
        let a = fixture(50, 1);
        let b = fixture(50, 1);
        assert_eq!(a, b);
        assert_eq!(a.node_count, 50);
        assert_eq!(a.nodes.len(), 50);
        assert!(a.edges.iter().all(|e| e.source != e.target));
        let ids: std::collections::HashSet<_> = a.nodes.iter().map(|n| &n.id).collect();
        assert!(a
            .edges
            .iter()
            .all(|e| ids.contains(&e.source) && ids.contains(&e.target)));
    }

    #[test]
    fn fixture_zero_and_one_node_have_no_edges() {
        assert_eq!(fixture(0, 1).edges.len(), 0);
        assert_eq!(fixture(1, 1).edges.len(), 0);
    }
}
