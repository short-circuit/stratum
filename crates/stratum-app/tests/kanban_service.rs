//! Headless tests for the kanban service commands:
//! `get_kanban_blocks`, `create_kanban_block`.

mod common;

use common::create_test_vault;

#[tokio::test]
async fn create_kanban_block_writes_todays_journal() {
    let vault = create_test_vault();

    let block = vault
        .service
        .create_kanban_block("ship the feature".into(), "TODO".into())
        .await
        .unwrap();

    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    assert_eq!(block.page_path, format!("journals/{today}.md"));
    assert_eq!(block.marker.as_deref(), Some("TODO"));
    assert_eq!(block.content, "ship the feature");
    assert!(vault.exists(&block.page_path));
}

#[tokio::test]
async fn get_kanban_blocks_returns_marked_blocks() {
    let vault = create_test_vault();
    let created = vault
        .service
        .create_kanban_block("doing work".into(), "DOING".into())
        .await
        .unwrap();

    let data = vault.service.get_kanban_blocks().await.unwrap();

    assert!(
        data.blocks.iter().any(|b| b.id == created.id),
        "created card must appear on the board"
    );
    let found = data.blocks.iter().find(|b| b.id == created.id).unwrap();
    assert_eq!(found.marker.as_deref(), Some("DOING"));
}
