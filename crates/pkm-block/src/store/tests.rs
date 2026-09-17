//! Tests for the block store.

use super::*;
use crate::block::{Block, BlockId, Priority, TaskMarker};
use crate::page::Page;

use tempfile::TempDir;
use uuid::Uuid;

#[test]
fn test_open_in_memory() {
    let store = BlockStore::open_in_memory().unwrap();
    assert_eq!(store.block_count().unwrap(), 0);
}

#[test]
fn test_open_disk() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("test.db");
    let store = BlockStore::open(&path).unwrap();
    assert_eq!(store.page_count().unwrap(), 0);
}

#[test]
fn test_insert_and_get_block() {
    let store = BlockStore::open_in_memory().unwrap();
    let id = Uuid::new_v4();
    let block = Block::new(id, "Hello world".into())
        .with_marker(TaskMarker::Todo)
        .with_priority(Priority::A);

    store.insert_block(&block, "pages/test.md").unwrap();

    let retrieved = store.get_block(id).unwrap();
    assert_eq!(retrieved.content, "Hello world");
    assert_eq!(retrieved.marker, Some(TaskMarker::Todo));
    assert_eq!(retrieved.priority, Some(Priority::A));
}

#[test]
fn test_update_block() {
    let store = BlockStore::open_in_memory().unwrap();
    let id = Uuid::new_v4();
    let mut block = Block::new(id, "Original".into());

    store.insert_block(&block, "pages/test.md").unwrap();

    block.content = "Updated".into();
    block.marker = Some(TaskMarker::Done);
    store.update_block(&block).unwrap();

    let retrieved = store.get_block(id).unwrap();
    assert_eq!(retrieved.content, "Updated");
    assert_eq!(retrieved.marker, Some(TaskMarker::Done));
}

#[test]
fn test_delete_block() {
    let store = BlockStore::open_in_memory().unwrap();
    let id = Uuid::new_v4();
    let block = Block::new(id, "Delete me".into());

    store.insert_block(&block, "pages/test.md").unwrap();
    assert_eq!(store.block_count().unwrap(), 1);

    store.delete_block(id).unwrap();
    assert_eq!(store.block_count().unwrap(), 0);

    // Getting deleted block should error
    assert!(store.get_block(id).is_err());
}

#[test]
fn test_get_blocks_by_page() {
    let store = BlockStore::open_in_memory().unwrap();
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let c = Uuid::new_v4();

    store
        .insert_block(&Block::new(a, "A".into()), "p/a.md")
        .unwrap();
    store
        .insert_block(&Block::new(b, "B".into()), "p/a.md")
        .unwrap();
    store
        .insert_block(&Block::new(c, "C".into()), "p/b.md")
        .unwrap();

    let page_a = store.get_blocks_by_page("p/a.md").unwrap();
    assert_eq!(page_a.len(), 2);
}

#[test]
fn test_upsert_and_get_page() {
    let store = BlockStore::open_in_memory().unwrap();
    let page = Page::new("pages/test.md".into(), std::path::Path::new("/vault"));

    store.upsert_page(&page).unwrap();

    let fm = store.get_page("pages/test.md").unwrap();
    assert!(fm.is_some());

    let pages = store.list_pages().unwrap();
    assert_eq!(pages.len(), 1);
}

#[test]
fn test_insert_and_get_links() {
    let store = BlockStore::open_in_memory().unwrap();
    let source = Uuid::new_v4();
    let target = Uuid::new_v4();

    store
        .insert_block(&Block::new(source, "Source".into()), "pages/src.md")
        .unwrap();
    store
        .insert_block(&Block::new(target, "Target".into()), "pages/tgt.md")
        .unwrap();

    store
        .insert_link(source, "block_ref", None, Some(target))
        .unwrap();
    store
        .insert_link(source, "page_ref", Some("Target Page"), None)
        .unwrap();

    let block_backlinks = store.get_backlinks_for_block(target).unwrap();
    assert_eq!(block_backlinks.len(), 1);
    assert_eq!(block_backlinks[0], source.to_string());

    let page_backlinks = store.get_backlinks_for_page("Target Page").unwrap();
    assert_eq!(page_backlinks.len(), 1);
}

#[test]
fn test_find_blocks_by_marker() {
    let store = BlockStore::open_in_memory().unwrap();
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();

    store
        .insert_block(
            &Block::new(a, "Task 1".into()).with_marker(TaskMarker::Todo),
            "pages/tasks.md",
        )
        .unwrap();
    store
        .insert_block(
            &Block::new(b, "Task 2".into()).with_marker(TaskMarker::Done),
            "pages/tasks.md",
        )
        .unwrap();

    let todos = store.find_blocks_by_marker("TODO").unwrap();
    assert_eq!(todos.len(), 1);
    assert_eq!(todos[0].id, a);
}

#[test]
fn test_find_blocks_by_markers() {
    let store = BlockStore::open_in_memory().unwrap();
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let c = Uuid::new_v4();

    store
        .insert_block(
            &Block::new(a, "Task 1".into()).with_marker(TaskMarker::Todo),
            "pages/tasks.md",
        )
        .unwrap();
    store
        .insert_block(
            &Block::new(b, "Task 2".into()).with_marker(TaskMarker::Doing),
            "pages/tasks.md",
        )
        .unwrap();
    store
        .insert_block(
            &Block::new(c, "Task 3".into()).with_marker(TaskMarker::Done),
            "pages/archive.md",
        )
        .unwrap();

    // Query for ["TODO", "DOING"] — returns 2 results with correct page_path
    let results = store.find_blocks_by_markers(&["TODO", "DOING"]).unwrap();
    assert_eq!(results.len(), 2);
    for (block, page_path) in &results {
        assert_eq!(page_path.as_str(), "pages/tasks.md");
        assert!(block.marker == Some(TaskMarker::Todo) || block.marker == Some(TaskMarker::Doing));
    }
    let result_ids: Vec<BlockId> = results.iter().map(|(b, _)| b.id).collect();
    assert!(result_ids.contains(&a));
    assert!(result_ids.contains(&b));
    assert!(!result_ids.contains(&c));

    // Query for ["NOW"] — returns empty
    let empty = store.find_blocks_by_markers(&["NOW"]).unwrap();
    assert_eq!(empty.len(), 0);

    // Query with empty markers — returns empty
    let empty2 = store.find_blocks_by_markers(&[]).unwrap();
    assert_eq!(empty2.len(), 0);
}

#[test]
fn test_delete_links_for_page() {
    let store = BlockStore::open_in_memory().unwrap();
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();

    store
        .insert_block(&Block::new(a, "links to [[x]]".into()), "pages/a.md")
        .unwrap();
    store
        .insert_block(&Block::new(b, "links to [[y]]".into()), "pages/b.md")
        .unwrap();
    store.insert_link(a, "page_ref", Some("x"), None).unwrap();
    store.insert_link(b, "page_ref", Some("y"), None).unwrap();

    assert_eq!(store.get_backlinks_for_page("x").unwrap().len(), 1);
    assert_eq!(store.get_backlinks_for_page("y").unwrap().len(), 1);

    // Deleting links for page a must clear only a's links, leaving b's intact.
    store.delete_links_for_page("pages/a.md").unwrap();
    assert_eq!(store.get_backlinks_for_page("x").unwrap().len(), 0);
    assert_eq!(store.get_backlinks_for_page("y").unwrap().len(), 1);
}

#[test]
fn test_delete_blocks_by_page() {
    let store = BlockStore::open_in_memory().unwrap();
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();

    store
        .insert_block(&Block::new(a, "A".into()), "pages/to_delete.md")
        .unwrap();
    store
        .insert_block(&Block::new(b, "B".into()), "pages/keep.md")
        .unwrap();

    let deleted = store.delete_blocks_by_page("pages/to_delete.md").unwrap();
    assert_eq!(deleted, 1);
    assert_eq!(store.block_count().unwrap(), 1);
}
