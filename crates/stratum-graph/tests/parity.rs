//! Parity checks against the shipped React/three.js and `d3-force-3d`
//! behaviour: seed hashes, palette selection, filter policy and a tick-for-tick
//! comparison with a JavaScript reference run.

use stratum_graph::{
    apply_cap, effective_node_cap, node_color, seed_position, GraphData, GraphEdge, GraphNode,
    Simulation, Vec3, DEFAULT_LARGE_VAULT_CAP, LARGE_VAULT_THRESHOLD,
};

fn node(id: &str, degree: usize) -> GraphNode {
    GraphNode {
        id: id.to_string(),
        title: id.to_string(),
        path: format!("{id}.md"),
        tags: Vec::new(),
        degree,
    }
}

fn edge(source: &str, target: &str) -> GraphEdge {
    GraphEdge {
        source: source.to_string(),
        target: target.to_string(),
        label: None,
    }
}

#[test]
fn seed_positions_match_the_shipped_hash() {
    assert_eq!(seed_position("alpha"), Vec3::new(1.0, 41.0, -21.0));
    assert_eq!(
        seed_position("[[Page Name]]"),
        Vec3::new(25.0, -31.0, -45.0)
    );
}

#[test]
fn palette_selection_matches_the_shipped_accessor() {
    let alpha = node("alpha", 0);
    assert_eq!(node_color(&alpha), [0x2d, 0xd4, 0xbf]);
    let mut tagged = alpha.clone();
    tagged.tags = vec!["tag-0".to_string()];
    assert_eq!(node_color(&tagged), [0x60, 0xa5, 0xfa]);
    let fixture_node = node("note-0001", 0);
    assert_eq!(node_color(&fixture_node), [0xa7, 0x8b, 0xfa]);
}

#[test]
fn filter_policy_matches_the_shipped_thresholds() {
    assert_eq!(effective_node_cap(500, 10_000), 500);
    assert_eq!(effective_node_cap(0, LARGE_VAULT_THRESHOLD), 0);
    assert_eq!(
        effective_node_cap(0, LARGE_VAULT_THRESHOLD + 1),
        DEFAULT_LARGE_VAULT_CAP
    );
    let nodes = vec![node("a", 1), node("b", 5), node("c", 3), node("d", 0)];
    let edges = vec![
        edge("a", "b"),
        edge("b", "c"),
        edge("c", "d"),
        edge("a", "d"),
    ];
    let (kept, kept_edges) = apply_cap(&nodes, &edges, 2);
    assert_eq!(
        kept.iter().map(|node| node.id.as_str()).collect::<Vec<_>>(),
        vec!["b", "c"]
    );
    assert_eq!(kept_edges, vec![edge("b", "c")]);
}

/// The JavaScript reference graph used for the tick parity check.
fn parity_graph() -> GraphData {
    GraphData::new(
        vec![
            node("alpha", 0),
            node("beta", 0),
            node("gamma", 0),
            node("delta", 0),
        ],
        vec![
            edge("alpha", "beta"),
            edge("beta", "gamma"),
            edge("gamma", "delta"),
        ],
        "/parity",
    )
}

#[test]
fn sequential_ticks_match_the_javascript_reference() {
    let data = parity_graph();
    let mut simulation = Simulation::new(&data, 1).expect("simulation");
    simulation.advance_sequential(10);

    // Captured from d3-force-3d with the shipped configuration (charge -30,
    // link distance 50, collide radius 4, z-spring 0.08, alpha decay 0.15,
    // velocity decay 0.4) after ten ticks.
    let expected = [
        (
            "alpha",
            -6.992_758_825_533_778,
            37.770_671_135_327_64,
            -24.181_864_944_584_78,
        ),
        (
            "beta",
            -22.236_461_576_630_187,
            -6.717_417_500_904_180_5,
            -43.842_825_955_201_72,
        ),
        (
            "gamma",
            28.074_021_830_180_502,
            10.227_499_844_987_038,
            -40.465_559_459_141_37,
        ),
        (
            "delta",
            20.132_209_080_587_426,
            37.708_827_812_128_035,
            4.133_586_275_505_727,
        ),
    ];
    for (index, (id, x, y, z)) in expected.iter().enumerate() {
        let node = simulation.node(index).expect("node");
        assert_eq!(node.id, *id);
        assert!((node.x - x).abs() < 1e-6, "node {id} x {} != {x}", node.x);
        assert!((node.y - y).abs() < 1e-6, "node {id} y {} != {y}", node.y);
        assert!((node.z - z).abs() < 1e-6, "node {id} z {} != {z}", node.z);
    }
    assert!((simulation.alpha() - 0.196_874_404_340_722_62).abs() < 1e-12);
}
