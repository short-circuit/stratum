use super::*;
use pkm_block::{Block, BlockStore, Page};
use std::path::{Path, PathBuf};
use uuid::Uuid;

fn insert_test_page(store: &BlockStore, vault_root: &Path, rel_path: &str) {
    let full_path = vault_root.join(rel_path);
    let page = Page::new(full_path, vault_root);
    store.upsert_page(&page).unwrap();
}

/// Insert a block AND reconcile the `links` table for its page — mirrors the
/// real write path (`save_blocks` → `reconcile_page_links`), which the graph
/// read path depends on. `reconcile_page_links` stores the RAW `[[Target]]`
/// string as the link target, exactly as production writes do.
fn insert_block_with_links(store: &BlockStore, block: Block, rel_path: &str) {
    store.insert_block(&block, rel_path).unwrap();
    crate::commands::page::reconcile_page_links(store, rel_path, &[block]).unwrap();
}

#[test]
fn test_self_link_included_in_edges() {
    let store = BlockStore::open_in_memory().unwrap();
    let vault_root = PathBuf::from("/tmp/test-vault");
    let slug = "test-self-link";
    let rel_path = format!("pages/{}.md", slug);

    insert_test_page(&store, &vault_root, &rel_path);

    let block = Block::new(Uuid::new_v4(), format!("Self reference [[{}]]", slug));
    insert_block_with_links(&store, block, &rel_path);

    let data = build_graph_data_from_store(&store, "/tmp/test-vault").unwrap();

    assert_eq!(data.node_count, 1, "should have 1 node");
    assert_eq!(data.edge_count, 1, "should have 1 self-link edge");
    assert_eq!(
        data.edges[0].source, rel_path,
        "node/edge identity is the vault-relative path"
    );
    assert_eq!(
        data.edges[0].target, rel_path,
        "edge target is the path (self-link)"
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
    insert_block_with_links(&store, block_a.clone(), "pages/page-a.md");

    let block_b = Block::new(Uuid::new_v4(), "[[page-a]]".into());
    insert_block_with_links(&store, block_b, "pages/page-b.md");

    let components = get_connected_components_from_store(&store).unwrap();

    assert_eq!(components.len(), 1, "should have 1 connected component");
    assert_eq!(components[0].size, 2, "component should contain both pages");
}

/// Duplicate basename slugs in different folders must be DISTINCT nodes.
///
///   pages/a/hub.md:  "[[b/note]]"          → resolves (path-qualified) → pages/b/note.md
///   pages/b/note.md: "self [[note]] x"     → `[[note]]` is ambiguous → no edge
///   pages/a/note.md: "lonely"              → no links
#[test]
fn test_duplicate_slugs_are_distinct_nodes() {
    let store = BlockStore::open_in_memory().unwrap();
    let vault_root = PathBuf::from("/tmp/test-vault");

    insert_test_page(&store, &vault_root, "pages/a/hub.md");
    insert_test_page(&store, &vault_root, "pages/a/note.md");
    insert_test_page(&store, &vault_root, "pages/b/note.md");

    let hub_block = Block::new(Uuid::new_v4(), "[[b/note]]".into());
    insert_block_with_links(&store, hub_block, "pages/a/hub.md");

    let note_b_block = Block::new(Uuid::new_v4(), "self [[note]] x".into());
    insert_block_with_links(&store, note_b_block, "pages/b/note.md");

    let note_a_block = Block::new(Uuid::new_v4(), "lonely".into());
    insert_block_with_links(&store, note_a_block, "pages/a/note.md");

    let data = build_graph_data_from_store(&store, "/tmp/test-vault").unwrap();

    assert_eq!(data.node_count, 3, "each path is a distinct node");
    assert_eq!(data.edge_count, 1, "exactly one edge survives");
    assert_eq!(data.edges[0].source, "pages/a/hub.md");
    assert_eq!(data.edges[0].target, "pages/b/note.md");

    let orphans = get_orphaned_notes_from_store(&store).unwrap();
    assert_eq!(
        orphans.len(),
        1,
        "only a/note is an orphan (b/note has an incoming edge)"
    );
    assert_eq!(orphans[0].path, "pages/a/note.md");

    let components = get_connected_components_from_store(&store).unwrap();
    assert_eq!(
        components.len(),
        2,
        "hub+b/note in one component, a/note alone"
    );
    let expected_sizes: Vec<usize> = components.iter().map(|c| c.size).collect();
    assert!(expected_sizes.contains(&2));
    assert!(expected_sizes.contains(&1));
}

/// A title shared by two paths is ambiguous → no resolution; a unique title
/// still resolves.
#[test]
fn test_resolve_ambiguous_title_is_none() {
    let store = BlockStore::open_in_memory().unwrap();
    let vault_root = PathBuf::from("/tmp/test-vault");

    insert_test_page(&store, &vault_root, "pages/a/title-page.md");
    insert_test_page(&store, &vault_root, "pages/b/title-page.md");
    insert_test_page(&store, &vault_root, "pages/unique.md");

    let fm = pkm_block::PageFrontmatter {
        title: Some("Foo".to_string()),
        ..Default::default()
    };
    let mut page = Page::new(vault_root.join("pages/a/title-page.md"), &vault_root);
    page.frontmatter = fm.clone();
    store.upsert_page(&page).unwrap();
    let mut page_b = Page::new(vault_root.join("pages/b/title-page.md"), &vault_root);
    page_b.frontmatter = fm;
    store.upsert_page(&page_b).unwrap();

    let meta = PageMetaIndex::from_store(&store).unwrap();

    // Ambiguous title → None, but candidates exposes both paths.
    assert!(
        meta.resolve_path("Foo").is_none(),
        "ambiguous title is unresolved"
    );
    assert_eq!(
        meta.resolve_slug_candidates("Foo").len(),
        2,
        "both paths are candidates"
    );

    // Unique page still resolves (regression).
    let unique = meta.resolve_path("Unique").unwrap();
    assert_eq!(unique, "pages/unique.md");
}
