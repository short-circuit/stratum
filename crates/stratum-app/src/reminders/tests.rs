//! Tests for arbitrary reminder due setters and the due helpers.
//!
//! These drive the real SQLite block store in a temporary vault; no Tauri
//! runtime or filesystem watcher is involved.

use crate::reminders::due::{parse_due, ReminderBuckets};
use crate::state::StratumService;
use chrono::{Local, Utc};
use pkm_block::Block;
use std::path::Path;
use uuid::Uuid;

fn block_on<F: std::future::Future>(fut: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime")
        .block_on(fut)
}

fn service_for(vault: &Path) -> StratumService {
    std::fs::create_dir_all(vault.join(".pkm")).expect("create .pkm");
    StratumService::new(vault.to_path_buf())
}

fn seed_block(service: &StratumService, page_path: &str) -> (String, Uuid) {
    let id = Uuid::new_v4();
    let store = service.lock().expect("lock").get_store().expect("store");
    store
        .insert_block(&Block::new(id, "Reminder test".to_string()), page_path)
        .expect("insert block");
    (id.to_string(), id)
}

fn read_block(service: &StratumService, id: Uuid) -> Block {
    let store = service.lock().expect("lock").get_store().expect("store");
    store.get_block(id).expect("block")
}

#[test]
fn set_block_reminder_persists_an_rfc3339_due() {
    let dir = tempfile::tempdir().expect("tempdir");
    let service = service_for(dir.path());
    let page_path = "pages/reminders.md";
    let (block_id, id) = seed_block(&service, page_path);

    let due = "2026-11-12T09:30:00Z";
    block_on(service.set_block_reminder(page_path.to_string(), block_id, Some(due.to_string())))
        .expect("set reminder");

    let block = read_block(&service, id);
    assert_eq!(block.due(), parse_due(due));
}

#[test]
fn set_block_reminder_accepts_a_naive_local_value() {
    let dir = tempfile::tempdir().expect("tempdir");
    let service = service_for(dir.path());
    let page_path = "pages/reminders.md";
    let (block_id, id) = seed_block(&service, page_path);

    let due = "2026-11-12T09:30";
    block_on(service.set_block_reminder(page_path.to_string(), block_id, Some(due.to_string())))
        .expect("set reminder");

    let block = read_block(&service, id);
    let expected = parse_due(due).expect("parsed");
    assert_eq!(block.due(), Some(expected));
}

#[test]
fn set_block_reminder_with_none_removes_the_due_property() {
    let dir = tempfile::tempdir().expect("tempdir");
    let service = service_for(dir.path());
    let page_path = "pages/reminders.md";
    let (block_id, id) = seed_block(&service, page_path);

    block_on(service.set_block_reminder(
        page_path.to_string(),
        block_id.clone(),
        Some("2026-11-12T09:30:00Z".to_string()),
    ))
    .expect("set reminder");

    block_on(service.set_block_reminder(page_path.to_string(), block_id, None))
        .expect("clear reminder");

    let block = read_block(&service, id);
    assert_eq!(block.due(), None);
}

#[test]
fn set_block_reminder_rejects_an_invalid_timestamp() {
    let dir = tempfile::tempdir().expect("tempdir");
    let service = service_for(dir.path());
    let page_path = "pages/reminders.md";
    let (block_id, id) = seed_block(&service, page_path);

    let error = block_on(service.set_block_reminder(
        page_path.to_string(),
        block_id,
        Some("not-a-date".to_string()),
    ))
    .expect_err("invalid timestamp");
    assert!(error.contains("Invalid due timestamp"));
    assert_eq!(read_block(&service, id).due(), None);
}

#[test]
fn bucket_helper_matches_the_expected_shape() {
    let now = Local::now();
    let dues = vec![
        (now - chrono::Duration::hours(1)).with_timezone(&Utc),
        now.with_timezone(&Utc),
        (now + chrono::Duration::days(2)).with_timezone(&Utc),
    ];
    let buckets: ReminderBuckets = crate::reminders::due::bucket(&dues, now);
    assert_eq!(buckets.overdue, vec![0]);
    assert_eq!(buckets.today, vec![1]);
    assert_eq!(buckets.upcoming, vec![2]);
}
