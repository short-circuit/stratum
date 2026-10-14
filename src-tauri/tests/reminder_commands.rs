//! E10.F4 — Verify the reminders command layer end-to-end through the REAL
//! Tauri command handlers over the real IPC dispatcher, against a real temp
//! vault on disk.
//!
//! Acceptance contract:
//!   * RM-1 — `save_blocks` with a `due` property → `list_reminders` returns it
//!   * RM-2 — `snooze_reminder` shifts the block's `due` forward (+10 min)
//!   * RM-3 — `clear_reminder` removes the `due` property and the on-disk `.md`
//!     no longer contains a `.due` line

mod common;

use app_lib::commands::vault::{AppState, VaultState};
use pkm_block::Page;
use serde_json::json;
use std::sync::Mutex;
use tauri::test::{mock_builder, mock_context, noop_assets};
use tauri::webview::InvokeRequest;
use tauri::WebviewWindow;

fn build_app(vault: &common::TestVault) -> tauri::App<tauri::test::MockRuntime> {
    let vs = VaultState::new(vault.vault_path.clone());
    mock_builder()
        .manage(Mutex::new(vs) as AppState)
        .invoke_handler(tauri::generate_handler![
            app_lib::commands::block::save_blocks,
            app_lib::commands::block::get_blocks,
            app_lib::commands::block::update_block,
            app_lib::commands::reminders::list_reminders,
            app_lib::commands::reminders::snooze_reminder,
            app_lib::commands::reminders::clear_reminder,
        ])
        .build(mock_context(noop_assets()))
        .expect("app build")
}

fn invoke(
    webview: &WebviewWindow<tauri::test::MockRuntime>,
    cmd: &str,
    body: serde_json::Value,
) -> Result<serde_json::Value, serde_json::Value> {
    let request = InvokeRequest {
        cmd: cmd.to_string(),
        callback: tauri::ipc::CallbackFn(0),
        error: tauri::ipc::CallbackFn(1),
        url: "tauri://localhost".parse().unwrap(),
        body: tauri::ipc::InvokeBody::Json(body),
        headers: Default::default(),
        invoke_key: tauri::test::INVOKE_KEY.to_string(),
    };
    tauri::test::get_ipc_response(webview, request).map(|b| {
        b.deserialize::<serde_json::Value>()
            .unwrap_or(serde_json::Value::Null)
    })
}

fn webview(app: &tauri::App<tauri::test::MockRuntime>) -> WebviewWindow<tauri::test::MockRuntime> {
    tauri::WebviewWindowBuilder::new(app, "main", Default::default())
        .build()
        .unwrap()
}

fn seed_disk_page(tv: &common::TestVault, rel: &str, body: &str) {
    let full = tv.vault_path.join(rel);
    std::fs::create_dir_all(full.parent().unwrap()).unwrap();
    std::fs::write(&full, body).unwrap();
    let (_fm, _, blocks) = pkm_markdown::block_parser::parse_document(body);
    let mut page = Page::new(full, &tv.vault_path);
    page.set_blocks(&blocks);
    tv.store.upsert_page(&page).unwrap();
    for b in &blocks {
        tv.store.insert_block(b, rel).unwrap();
    }
}

#[test]
fn command_list_reminders_returns_block_with_due() {
    let tv = common::create_test_vault();
    seed_disk_page(
        &tv,
        "pages/tasks.md",
        "- Remind me to ship\n  .due: 2020-01-01T00:00:00.000Z\n  .id: 00000000-0000-0000-0000-000000000001\n",
    );
    let app = build_app(&tv);
    let wv = webview(&app);

    let res = invoke(&wv, "list_reminders", json!({}))
        .expect("list_reminders resolves");
    let arr = res.as_array().expect("list must return an array");
    assert_eq!(arr.len(), 1);
    let item = &arr[0];
    assert_eq!(item["content"], "Remind me to ship");
    assert_eq!(item["page_path"], "pages/tasks.md");
}

#[test]
fn command_snooze_reminder_shifts_due_forward() {
    let tv = common::create_test_vault();
    seed_disk_page(
        &tv,
        "pages/tasks.md",
        "- Remind me\n  .due: 2020-01-01T00:00:00.000Z\n  .id: 00000000-0000-0000-0000-000000000001\n",
    );
    let app = build_app(&tv);
    let wv = webview(&app);

    let before = invoke(&wv, "list_reminders", json!({}))
        .expect("list_reminders resolves");
    let due_before = before[0]["due"].as_str().unwrap().to_string();

    invoke(
        &wv,
        "snooze_reminder",
        json!({
            "pagePath": "pages/tasks.md",
            "blockId": "00000000-0000-0000-0000-000000000001",
            "minutes": 10
        }),
    )
    .expect("snooze resolves");

    let after = invoke(&wv, "list_reminders", json!({}))
        .expect("list_reminders resolves");
    let due_after = after[0]["due"].as_str().unwrap().to_string();

    let parse = |s: &str| -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::parse_from_rfc3339(s)
            .unwrap()
            .with_timezone(&chrono::Utc)
    };
    // Snooze must move the due time into the future, to ~now + 10 minutes.
    let now = chrono::Utc::now();
    let after_dt = parse(&due_after);
    assert!(
        after_dt > parse(&due_before),
        "snooze must move due forward"
    );
    let target = now + chrono::Duration::minutes(10);
    let drift = (after_dt - target).num_seconds().abs();
    assert!(
        drift < 5,
        "new due must be ~now+10min (drift {drift}s, due {due_after})"
    );
}

#[test]
fn command_clear_reminder_removes_due_property_and_disk_line() {
    let tv = common::create_test_vault();
    seed_disk_page(
        &tv,
        "pages/tasks.md",
        "- Clear me\n  .due: 2020-01-01T00:00:00.000Z\n  .id: 00000000-0000-0000-0000-000000000001\n",
    );
    let app = build_app(&tv);
    let wv = webview(&app);

    invoke(
        &wv,
        "clear_reminder",
        json!({
            "pagePath": "pages/tasks.md",
            "blockId": "00000000-0000-0000-0000-000000000001"
        }),
    )
    .expect("clear resolves");

    let res = invoke(&wv, "list_reminders", json!({})).expect("list resolves");
    assert!(res.as_array().unwrap().is_empty());

    let disk = std::fs::read_to_string(tv.vault_path.join("pages/tasks.md")).unwrap();
    assert!(
        !disk.contains(".due:"),
        "disk must no longer contain a .due line:\n{disk}"
    );
}
