//! Headless tests for the tray service commands:
//! `get_tray_status`, `quick_capture_save`.

mod common;

use common::create_test_vault;

#[tokio::test]
async fn get_tray_status_reports_default_configuration() {
    let vault = create_test_vault();

    let status = vault.service.get_tray_status().unwrap();

    assert_eq!(status.capture_target, "journal");
    assert_eq!(status.shortcut, "CmdOrCtrl+Shift+Space");
    assert_eq!(status.inbox_path, "Inbox.md");
    let desktop = !(cfg!(target_os = "android") || cfg!(target_os = "ios"));
    assert_eq!(status.enabled, desktop);
}

#[tokio::test]
async fn quick_capture_save_appends_to_todays_journal() {
    let vault = create_test_vault();

    let result = vault
        .service
        .quick_capture_save("captured thought".into())
        .await
        .unwrap();

    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    assert_eq!(result.page_path, format!("journals/{today}.md"));
    assert!(!result.block_id.is_empty());
    let content = vault.read_file(&result.page_path);
    assert!(
        content.contains("captured thought"),
        "capture must be persisted: {content}"
    );

    let blocks = vault
        .service
        .get_blocks(result.page_path.clone())
        .await
        .unwrap();
    assert!(
        blocks.blocks.iter().any(|b| b.id == result.block_id),
        "captured block must be stored"
    );
}
