use super::*;
use pkm_block::{Block, BlockStore, Page};
use std::path::{Path, PathBuf};
use uuid::Uuid;

fn insert_test_page(store: &BlockStore, vault_root: &Path, rel_path: &str) {
    let full_path = vault_root.join(rel_path);
    let page = Page::new(full_path, vault_root);
    store.upsert_page(&page).unwrap();
}

#[test]
fn test_self_link_included_in_edges() {
    let store = BlockStore::open_in_memory().unwrap();
    let vault_root = PathBuf::from("/tmp/test-vault");
    let slug = "test-self-link";
    let rel_path = format!("pages/{}.md", slug);

    insert_test_page(&store, &vault_root, &rel_path);

    let block = Block::new(Uuid::new_v4(), format!("Self reference [[{}]]", slug));
    store.insert_block(&block, &rel_path).unwrap();

    let data = build_graph_data_from_store(&store, "/tmp/test-vault").unwrap();

    assert_eq!(data.node_count, 1, "should have 1 node");
    assert_eq!(data.edge_count, 1, "should have 1 self-link edge");
    assert_eq!(data.edges[0].source, slug, "edge source should be the slug");
    assert_eq!(
        data.edges[0].target, slug,
        "edge target should be the slug (self-link)"
    );
    assert_eq!(
        data.nodes[0].degree, 1,
        "self-link degree should be 1 (not double-counted)"
    );
}

#[test]
fn test_self_link_does_not_affect_connected_components() {
    let store = BlockStore::open_in_memory().unwrap();
    let vault_root = PathBuf::from("/tmp/test-vault");

    insert_test_page(&store, &vault_root, "pages/page-a.md");
    insert_test_page(&store, &vault_root, "pages/page-b.md");

    let block_a = Block::new(Uuid::new_v4(), "[[page-a]]".into());
    store.insert_block(&block_a, "pages/page-a.md").unwrap();

    let block_b = Block::new(Uuid::new_v4(), "[[page-a]]".into());
    store.insert_block(&block_b, "pages/page-b.md").unwrap();

    let components = get_connected_components_from_store(&store).unwrap();

    assert_eq!(components.len(), 1, "should have 1 connected component");
    assert_eq!(components[0].size, 2, "component should contain both pages");
}
