mod common;

use common::create_test_vault;
use pkm_index::block_search::BlockIndex;
use uuid::Uuid;

fn make_block(content: &str) -> pkm_block::Block {
    pkm_block::Block::new(Uuid::new_v4(), content.to_string())
}

#[test]
fn test_index_and_search() {
    let tv = create_test_vault();
    let indexPath = tv.vault_path.join(".pkm").join("search");
    std::fs::create_dir_all(&indexPath).unwrap();
    let mut index = BlockIndex::create(&indexPath).unwrap();

    let b1 = make_block("Machine learning and neural networks");
    tv.store.insert_block(&b1, "pages/ai.md").unwrap();
    index.index_block(&b1, "pages/ai.md").unwrap();

    let b2 = make_block("Cooking recipes for pasta carbonara");
    tv.store.insert_block(&b2, "pages/cooking.md").unwrap();
    index.index_block(&b2, "pages/cooking.md").unwrap();

    index.flush().unwrap();

    let results = index.search("machine learning", 10).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].page_path, "pages/ai.md");

    let results = index.search("pasta", 10).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].page_path, "pages/cooking.md");

    let results = index.search("nonexistent", 10).unwrap();
    assert!(results.is_empty());
}

#[test]
fn test_remove_from_index() {
    let tv = create_test_vault();
    let indexPath = tv.vault_path.join(".pkm").join("search");
    std::fs::create_dir_all(&indexPath).unwrap();
    let mut index = BlockIndex::create(&indexPath).unwrap();

    let b = make_block("This will be removed");
    index.index_block(&b, "pages/test.md").unwrap();
    index.flush().unwrap();

    assert_eq!(index.search("removed", 10).unwrap().len(), 1);

    index.delete_block(b.id).unwrap();
    index.flush().unwrap();

    assert_eq!(index.search("removed", 10).unwrap().len(), 0);
}

#[test]
fn test_index_multiple_blocks_same_page() {
    let tv = create_test_vault();
    let indexPath = tv.vault_path.join(".pkm").join("search");
    std::fs::create_dir_all(&indexPath).unwrap();
    let mut index = BlockIndex::create(&indexPath).unwrap();

    for i in 0..3 {
        let b = make_block(&format!("Block number {}", i));
        tv.store.insert_block(&b, "pages/combo.md").unwrap();
        index.index_block(&b, "pages/combo.md").unwrap();
    }
    index.flush().unwrap();

    let results = index.search("Block", 10).unwrap();
    assert_eq!(results.len(), 3);
    for r in &results {
        assert_eq!(r.page_path, "pages/combo.md");
    }
}

#[test]
fn test_rebuild_index_with_clean_dir() {
    let tv = create_test_vault();
    let indexPath = tv.vault_path.join(".pkm").join("search");
    std::fs::create_dir_all(&indexPath).unwrap();

    // Create index and add a block
    let mut index = BlockIndex::create(&indexPath).unwrap();
    let b = make_block("Initial content");
    index.index_block(&b, "pages/test.md").unwrap();
    index.flush().unwrap();
    drop(index);

    // Remove index directory and recreate (simulating full rebuild)
    std::fs::remove_dir_all(&indexPath).unwrap();
    std::fs::create_dir_all(&indexPath).unwrap();

    let mut rebuilt = BlockIndex::create(&indexPath).unwrap();
    let b2 = make_block("New content");
    rebuilt.index_block(&b2, "pages/test.md").unwrap();
    rebuilt.flush().unwrap();

    // Should only find new content (old was cleared by directory removal)
    let results = rebuilt.search("Initial", 10).unwrap();
    assert_eq!(results.len(), 0);
    let results = rebuilt.search("New", 10).unwrap();
    assert_eq!(results.len(), 1);
}

#[test]
fn test_search_limit() {
    let tv = create_test_vault();
    let indexPath = tv.vault_path.join(".pkm").join("search");
    std::fs::create_dir_all(&indexPath).unwrap();
    let mut index = BlockIndex::create(&indexPath).unwrap();

    for i in 0..5 {
        let b = make_block(&format!("Searchable item {}", i));
        index.index_block(&b, "pages/test.md").unwrap();
    }
    index.flush().unwrap();

    let all = index.search("Searchable", 10).unwrap();
    assert_eq!(all.len(), 5);

    let limited = index.search("Searchable", 2).unwrap();
    assert_eq!(limited.len(), 2);
}
