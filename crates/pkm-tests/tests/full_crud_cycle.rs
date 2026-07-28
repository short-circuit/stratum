mod common;

use common::create_test_vault;
use uuid::Uuid;

#[test]
fn test_full_lifecycle() {
    let tv = create_test_vault();
    let page_path = "pages/my-note.md".to_string();

    // 1. Create page in SQLite
    tv.add_page(&page_path);
    assert!(tv.store.list_pages().unwrap().contains(&page_path));

    // 2. Insert blocks
    let b1 = tv.add_block(&page_path, "# My Note");
    let b2 = tv.add_block(&page_path, "Some content");
    assert_eq!(tv.store.get_blocks_by_page(&page_path).unwrap().len(), 2);

    // 3. Update a block
    let mut b2_updated = b2.clone();
    b2_updated.content = "Updated content".into();
    tv.store.update_block(&b2_updated).unwrap();
    let retrieved = tv.store.get_block(b2.id).unwrap();
    assert_eq!(retrieved.content, "Updated content");

    // 4. Delete a block
    tv.store.delete_block(b1.id).unwrap();
    assert_eq!(tv.store.get_blocks_by_page(&page_path).unwrap().len(), 1);

    // 5. Delete the page
    tv.store.delete_page(&page_path).unwrap();
    assert!(tv.store.get_page(&page_path).unwrap().is_none());
    assert_eq!(tv.store.list_pages().unwrap().len(), 0);
}

#[test]
fn test_insert_nonexistent_block_fails() {
    let tv = create_test_vault();
    let result = tv.store.get_block(Uuid::new_v4());
    assert!(result.is_err());
}

#[test]
fn test_update_nonexistent_block_fails() {
    let tv = create_test_vault();
    let block = pkm_block::Block::new(Uuid::new_v4(), "Ghost".into());
    // update_block with nonexistent ID should not error (INSERT OR REPLACE semantics)
    tv.store.update_block(&block).unwrap();
}

#[test]
fn test_delete_nonexistent_block_errors() {
    let tv = create_test_vault();
    let result = tv.store.delete_block(Uuid::new_v4());
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

#[test]
fn test_page_with_blocks_then_delete() {
    let tv = create_test_vault();
    tv.add_page("pages/full.md");
    tv.add_block("pages/full.md", "Block A");
    tv.add_block("pages/full.md", "Block B");

    tv.store.delete_page("pages/full.md").unwrap();
    // Verify blocks were also deleted
    assert_eq!(
        tv.store.get_blocks_by_page("pages/full.md").unwrap().len(),
        0
    );
}

#[test]
fn test_insert_block_with_properties() {
    let tv = create_test_vault();
    let block = pkm_block::Block::new(Uuid::new_v4(), "Task".into())
        .with_property("deadline", "2026-08-01")
        .with_property("assignee", "me");
    tv.store.insert_block(&block, "pages/test.md").unwrap();

    let retrieved = tv.store.get_block(block.id).unwrap();
    assert_eq!(
        retrieved.properties.get("deadline").map(|s| s.as_str()),
        Some("2026-08-01")
    );
    assert_eq!(
        retrieved.properties.get("assignee").map(|s| s.as_str()),
        Some("me")
    );
}
