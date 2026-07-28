mod common;

use common::create_test_vault;
use pkm_block::{Block, Priority, TaskMarker};

#[test]
fn test_insert_and_retrieve_block() {
    let tv = create_test_vault();
    let block = Block::new(uuid::Uuid::new_v4(), "Hello world".into())
        .with_marker(TaskMarker::Todo)
        .with_priority(Priority::A)
        .with_property("key", "value");

    tv.store.insert_block(&block, "pages/test.md").unwrap();
    let retrieved = tv.store.get_block(block.id).unwrap();

    assert_eq!(retrieved.content, "Hello world");
    assert_eq!(retrieved.marker, Some(TaskMarker::Todo));
    assert_eq!(
        retrieved.properties.get("key").map(|s| s.as_str()),
        Some("value")
    );
}

#[test]
fn test_get_nonexistent_block_errors() {
    let tv = create_test_vault();
    let id = uuid::Uuid::new_v4();
    let result = tv.store.get_block(id);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

#[test]
fn test_update_block() {
    let tv = create_test_vault();
    let mut block = tv.add_block("pages/test.md", "Original content");
    block.content = "Updated content".into();
    tv.store.update_block(&block).unwrap();

    let retrieved = tv.store.get_block(block.id).unwrap();
    assert_eq!(retrieved.content, "Updated content");
}

#[test]
fn test_delete_block() {
    let tv = create_test_vault();
    let block = tv.add_block("pages/test.md", "To be deleted");
    tv.store.delete_block(block.id).unwrap();

    let result = tv.store.get_block(block.id);
    assert!(result.is_err());
}

#[test]
fn test_blocks_by_page() {
    let tv = create_test_vault();
    tv.add_block("pages/test.md", "Block 1");
    tv.add_block("pages/test.md", "Block 2");
    tv.add_block("pages/other.md", "Block 3");

    let page1 = tv.store.get_blocks_by_page("pages/test.md").unwrap();
    assert_eq!(page1.len(), 2);

    let page2 = tv.store.get_blocks_by_page("pages/other.md").unwrap();
    assert_eq!(page2.len(), 1);
}

#[test]
fn test_page_crud() {
    let tv = create_test_vault();
    tv.add_page("pages/test.md");
    assert!(tv.store.get_page("pages/test.md").unwrap().is_some());

    let pages = tv.store.list_pages().unwrap();
    assert!(pages.contains(&"pages/test.md".to_string()));

    tv.store.delete_page("pages/test.md").unwrap();
    assert!(tv.store.get_page("pages/test.md").unwrap().is_none());
}

#[test]
fn test_page_count() {
    let tv = create_test_vault();
    tv.add_page("pages/a.md");
    tv.add_page("pages/b.md");
    assert_eq!(tv.store.page_count().unwrap(), 2);
}

#[test]
fn test_block_count() {
    let tv = create_test_vault();
    tv.add_page("pages/test.md");
    tv.add_block("pages/test.md", "Block 1");
    tv.add_block("pages/test.md", "Block 2");
    assert_eq!(tv.store.block_count().unwrap(), 2);
}

#[test]
fn test_insert_link_and_backlinks() {
    let tv = create_test_vault();
    let source = tv.add_block("pages/a.md", "Links to [[other-page]]");
    tv.store
        .insert_link(source.id, "page_ref", Some("pages/other.md"), None)
        .unwrap();

    let backlinks = tv.store.get_backlinks_for_page("pages/other.md").unwrap();
    assert_eq!(backlinks.len(), 1);
    assert_eq!(backlinks[0], source.id.to_string());
}

#[test]
fn test_find_blocks_by_marker() {
    let tv = create_test_vault();
    tv.add_block_with_marker("pages/test.md", "Task 1", TaskMarker::Todo);
    tv.add_block_with_marker("pages/test.md", "Task 2", TaskMarker::Doing);
    tv.add_block("pages/test.md", "Not a task");

    let todos = tv.store.find_blocks_by_marker("TODO").unwrap();
    assert_eq!(todos.len(), 1);
    assert!(todos[0].content.contains("Task 1"));
}

#[test]
fn test_find_blocks_by_multiple_markers() {
    let tv = create_test_vault();
    tv.add_block_with_marker("pages/test.md", "Todo task", TaskMarker::Todo);
    tv.add_block_with_marker("pages/test.md", "Doing task", TaskMarker::Doing);
    tv.add_block_with_marker("pages/test.md", "Done task", TaskMarker::Done);

    let open = tv.store.find_blocks_by_markers(&["TODO", "DOING"]).unwrap();
    assert_eq!(open.len(), 2);

    let done = tv.store.find_blocks_by_markers(&["DONE"]).unwrap();
    assert_eq!(done.len(), 1);
}

#[test]
fn test_empty_markers_returns_empty() {
    let tv = create_test_vault();
    let result = tv.store.find_blocks_by_markers(&[]).unwrap();
    assert!(result.is_empty());
}

#[test]
fn test_delete_links_on_block_delete() {
    let tv = create_test_vault();
    let source = tv.add_block("pages/a.md", "Links to b");
    tv.add_page("pages/b.md");
    tv.store
        .insert_link(source.id, "page_ref", Some("pages/b.md"), None)
        .unwrap();

    // Delete the source block (CASCADE should remove the link)
    tv.store.delete_block(source.id).unwrap();
    let backlinks = tv.store.get_backlinks_for_page("pages/b.md").unwrap();
    assert!(backlinks.is_empty());
}
