//! Headless tests for the reminder service commands:
//! `list_reminders`, `snooze_reminder`, `clear_reminder`.

mod common;

use common::{block_dto, create_test_vault};

fn due_block(minutes_ahead: i64) -> stratum_app::block::BlockDto {
    let mut block = block_dto("remind me");
    let due = (chrono::Utc::now() + chrono::Duration::minutes(minutes_ahead)).to_rfc3339();
    block.properties = vec![("due".into(), due)];
    block
}

#[tokio::test]
async fn list_reminders_returns_due_blocks() {
    let vault = create_test_vault();
    let block = due_block(60);
    let block_id = block.id.clone();
    vault
        .service
        .save_blocks("pages/reminders.md".into(), vec![block], None)
        .await
        .unwrap();

    let reminders = vault.service.list_reminders().await.unwrap();

    assert_eq!(reminders.len(), 1);
    assert_eq!(reminders[0].block_id, block_id);
    assert_eq!(reminders[0].page_path, "pages/reminders.md");
    assert_eq!(reminders[0].content, "remind me");
}

#[tokio::test]
async fn snooze_reminder_pushes_due_time_forward() {
    let vault = create_test_vault();
    let block = due_block(10);
    let block_id = block.id.clone();
    let original_due = block.properties[0].1.clone();
    vault
        .service
        .save_blocks("pages/snooze.md".into(), vec![block], None)
        .await
        .unwrap();

    vault
        .service
        .snooze_reminder("pages/snooze.md".into(), block_id.clone(), 60)
        .await
        .unwrap();

    let reminders = vault.service.list_reminders().await.unwrap();
    assert_eq!(reminders.len(), 1);
    assert!(
        reminders[0].due > original_due,
        "snoozed due must move forward: {} > {}",
        reminders[0].due,
        original_due
    );
}

#[tokio::test]
async fn clear_reminder_removes_the_due_property() {
    let vault = create_test_vault();
    let block = due_block(30);
    let block_id = block.id.clone();
    vault
        .service
        .save_blocks("pages/clear.md".into(), vec![block], None)
        .await
        .unwrap();

    vault
        .service
        .clear_reminder("pages/clear.md".into(), block_id)
        .await
        .unwrap();

    assert!(vault.service.list_reminders().await.unwrap().is_empty());
}
