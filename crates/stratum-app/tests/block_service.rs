//! Headless tests for the block service commands:
//! `build_markdown`, `save_blocks`, `get_blocks`, `update_block`,
//! `delete_block`, `insert_block`, `toggle_block_marker`,
//! `clear_block_marker`.

mod common;

use common::{block_dto, create_test_vault};

#[tokio::test]
async fn build_markdown_serializes_blocks_and_title() {
    let vault = create_test_vault();

    let md = vault
        .service
        .build_markdown(vec![block_dto("hello world")], Some("Doc".into()))
        .await
        .unwrap();

    assert!(md.contains("title: Doc"), "frontmatter title: {md}");
    assert!(md.contains("hello world"), "block body: {md}");
}

#[tokio::test]
async fn save_blocks_writes_file_and_database_rows() {
    let vault = create_test_vault();

    vault
        .service
        .save_blocks(
            "pages/blocks.md".into(),
            vec![block_dto("alpha"), block_dto("beta")],
            Some("Blocks".into()),
        )
        .await
        .unwrap();

    let content = vault.read_file("pages/blocks.md");
    assert!(content.contains("alpha") && content.contains("beta"));
    let blocks = vault
        .service
        .get_blocks("pages/blocks.md".into())
        .await
        .unwrap();
    assert_eq!(blocks.blocks.len(), 2);
}

#[tokio::test]
async fn get_blocks_returns_persisted_blocks() {
    let vault = create_test_vault();
    vault
        .service
        .save_blocks("pages/read.md".into(), vec![block_dto("read me")], None)
        .await
        .unwrap();

    let blocks = vault
        .service
        .get_blocks("pages/read.md".into())
        .await
        .unwrap();
    assert_eq!(blocks.blocks.len(), 1);
    assert_eq!(blocks.blocks[0].content, "read me");
}

#[tokio::test]
async fn update_block_persists_new_content() {
    let vault = create_test_vault();
    let block = block_dto("before");
    let id = block.id.clone();
    vault
        .service
        .save_blocks("pages/update.md".into(), vec![block], None)
        .await
        .unwrap();

    let mut updated = block_dto("after");
    updated.id = id.clone();
    vault
        .service
        .update_block("pages/update.md".into(), updated)
        .await
        .unwrap();

    let blocks = vault
        .service
        .get_blocks("pages/update.md".into())
        .await
        .unwrap();
    assert_eq!(blocks.blocks[0].content, "after");
}

#[tokio::test]
async fn delete_block_removes_the_row() {
    let vault = create_test_vault();
    let block = block_dto("delete me");
    let id = block.id.clone();
    vault
        .service
        .save_blocks("pages/delete.md".into(), vec![block], None)
        .await
        .unwrap();

    vault.service.delete_block(id.clone()).await.unwrap();

    // `delete_block` is SQLite-only (the outliner follows it with `save_blocks`,
    // which rewrites the file); `get_blocks` would heal from the still-present
    // disk file, so assert against the database row directly.
    let remaining = vault.store().get_blocks_by_page("pages/delete.md").unwrap();
    assert!(
        remaining.iter().all(|b| b.id.to_string() != id),
        "block row must be deleted"
    );
}

#[tokio::test]
async fn insert_block_creates_a_new_block() {
    let vault = create_test_vault();
    vault
        .service
        .save_blocks("pages/insert.md".into(), vec![block_dto("existing")], None)
        .await
        .unwrap();

    let inserted = vault
        .service
        .insert_block("pages/insert.md".into(), "inserted".into(), None, None)
        .await
        .unwrap();

    assert_eq!(inserted.content, "inserted");
    assert!(!inserted.id.is_empty());
    let blocks = vault
        .service
        .get_blocks("pages/insert.md".into())
        .await
        .unwrap();
    assert!(blocks.blocks.iter().any(|b| b.content == "inserted"));
}

#[tokio::test]
async fn toggle_block_marker_sets_todo() {
    let vault = create_test_vault();
    let block = block_dto("task");
    let id = block.id.clone();
    vault
        .service
        .save_blocks("pages/task.md".into(), vec![block], None)
        .await
        .unwrap();

    let marker = vault
        .service
        .toggle_block_marker("pages/task.md".into(), id)
        .await
        .unwrap();

    assert_eq!(marker.as_deref(), Some("TODO"));
}

#[tokio::test]
async fn clear_block_marker_removes_the_marker() {
    let vault = create_test_vault();
    let mut block = block_dto("marked task");
    block.marker = Some("TODO".into());
    let id = block.id.clone();
    vault
        .service
        .save_blocks("pages/marked.md".into(), vec![block], None)
        .await
        .unwrap();

    vault
        .service
        .clear_block_marker("pages/marked.md".into(), id)
        .await
        .unwrap();

    let blocks = vault
        .service
        .get_blocks("pages/marked.md".into())
        .await
        .unwrap();
    assert_eq!(blocks.blocks[0].marker, None);
}
