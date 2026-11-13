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

    // Captured from d3-force-3d with the canonical configuration (charge -30,
    // link distance 100, collide radius 4, z-spring 0.08, alpha decay 0.02,
    // velocity decay 0.4) after ten ticks.
    let expected = [
        (
            "alpha",
            -0.152_115_174_595_217,
            58.996_253_891_838_506,
            -22.864_434_584_905_76,
        ),
        (
            "beta",
            -43.370_812_955_217_076,
            -27.462_422_956_321_74,
            -46.699_622_986_894_16,
        ),
        (
            "gamma",
            51.969_285_715_891_246,
            7.788_371_145_987_84,
            -48.012_748_326_064_45,
        ),
        (
            "delta",
            8.325_763_169_395_039,
            61.126_427_762_019_965,
            22.540_000_851_705_05,
        ),
    ];
    for (index, (id, x, y, z)) in expected.iter().enumerate() {
        let node = simulation.node(index).expect("node");
        assert_eq!(node.id, *id);
        assert!((node.x - x).abs() < 1e-6, "node {id} x {} != {x}", node.x);
        assert!((node.y - y).abs() < 1e-6, "node {id} y {} != {y}", node.y);
        assert!((node.z - z).abs() < 1e-6, "node {id} z {} != {z}", node.z);
    }
    assert!((simulation.alpha() - 0.817_072_806_887_546_9).abs() < 1e-12);
}
