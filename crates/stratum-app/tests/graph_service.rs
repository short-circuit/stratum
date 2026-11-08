//! Headless tests for the graph service commands:
//! `get_graph_data`, `get_connected_components`, `get_orphaned_notes`,
//! `get_graph_panel_data`, `resolve_link_target`.

mod common;

use common::{block_dto, create_test_vault};

#[tokio::test]
async fn get_graph_data_builds_nodes_and_edges() {
    let vault = create_test_vault();
    vault
        .service
        .create_page("pages/alpha.md".into(), Some("Alpha".into()))
        .await
        .unwrap();
    vault
        .service
        .create_page("pages/beta.md".into(), Some("Beta".into()))
        .await
        .unwrap();
    vault
        .service
        .save_blocks(
            "pages/alpha.md".into(),
            vec![block_dto("see [[beta]]")],
            None,
        )
        .await
        .unwrap();

    let graph = vault.service.get_graph_data().await.unwrap();

    assert!(
        graph.nodes.iter().any(|n| n.path == "pages/alpha.md"),
        "alpha node must exist"
    );
    assert!(
        graph.nodes.iter().any(|n| n.path == "pages/beta.md"),
        "beta node must exist"
    );
    assert!(
        graph
            .edges
            .iter()
            .any(|e| e.source == "pages/alpha.md" && e.target == "pages/beta.md"),
        "alpha→beta edge must exist: {:?}",
        graph.edges
    );
    assert_eq!(graph.node_count, graph.nodes.len());
    assert_eq!(graph.edge_count, graph.edges.len());
}

#[tokio::test]
async fn get_connected_components_groups_linked_pages() {
    let vault = create_test_vault();
    vault
        .service
        .create_page("pages/a.md".into(), None)
        .await
        .unwrap();
    vault
        .service
        .create_page("pages/b.md".into(), None)
        .await
        .unwrap();
    vault
        .service
        .save_blocks("pages/a.md".into(), vec![block_dto("[[b]]")], None)
        .await
        .unwrap();

    let components = vault.service.get_connected_components().await.unwrap();

    assert!(
        components.iter().any(|c| c.size >= 2),
        "linked pages must share a component: {components:?}"
    );
}

#[tokio::test]
async fn get_orphaned_notes_reports_unlinked_page() {
    let vault = create_test_vault();
    vault
        .service
        .create_page("pages/lonely.md".into(), Some("Lonely".into()))
        .await
        .unwrap();

    let orphans = vault.service.get_orphaned_notes().await.unwrap();

    assert!(
        orphans.iter().any(|o| o.path == "pages/lonely.md"),
        "unlinked page must be an orphan: {orphans:?}"
    );
}

#[tokio::test]
async fn get_graph_panel_data_combines_graph_components_and_orphans() {
    let vault = create_test_vault();
    vault
        .service
        .create_page("pages/a.md".into(), None)
        .await
        .unwrap();
    vault
        .service
        .create_page("pages/b.md".into(), None)
        .await
        .unwrap();
    vault
        .service
        .create_page("pages/isolated.md".into(), None)
        .await
        .unwrap();
    vault
        .service
        .save_blocks("pages/a.md".into(), vec![block_dto("[[b]]")], None)
        .await
        .unwrap();

    let panel = vault.service.get_graph_panel_data().await.unwrap();

    assert!(panel.graph.node_count >= 3);
    assert!(!panel.components.is_empty());
    assert!(
        panel.orphans.iter().any(|o| o.path == "pages/isolated.md"),
        "isolated page must be reported as orphan"
    );
}

#[tokio::test]
async fn resolve_link_target_resolves_page_identity() {
    let vault = create_test_vault();
    vault
        .service
        .create_page("pages/beta.md".into(), Some("Beta".into()))
        .await
        .unwrap();

    let target = vault
        .service
        .resolve_link_target("beta".into())
        .await
        .unwrap();

    assert_eq!(target.page_path.as_deref(), Some("pages/beta.md"));
    assert_eq!(target.slug.as_deref(), Some("beta"));
    assert_eq!(target.title.as_deref(), Some("Beta"));
}

#[tokio::test]
async fn resolve_link_target_reports_missing_target() {
    let vault = create_test_vault();

    let target = vault
        .service
        .resolve_link_target("does-not-exist".into())
        .await
        .unwrap();

    assert!(target.page_path.is_none());
    assert!(target.slug.is_none());
    assert!(target.title.is_none());
}
