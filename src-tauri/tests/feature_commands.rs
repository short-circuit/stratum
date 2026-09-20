//! E7.F4 — Verify journal auto-create, template variables, SM-2 review,
//! kanban marker mapping + edit dialog, and whiteboard save/load acceptance
//! criteria through the REAL Tauri command layer.
//!
//! These tests drive the REAL `#[tauri::command]` handlers over the real Tauri
//! IPC dispatcher (official `tauri::test` mock-app harness) against a REAL temp
//! vault on disk. Nothing is stubbed: store, serializer, and filesystem are
//! production code.
//!
//! The acceptance contract is that each flow completes end-to-end ON DISK:
//!   * JN — `ensure_today_journal` creates `journals/YYYY-MM-DD.md` (idempotent)
//!   * TM — `save_template` + `apply_template` substitute `{{var}}` and write the
//!     target page to disk
//!   * FC — `generate_flashcards` produces Q/A cards; `review_card` runs the SM-2
//!     scheduler and persists the new schedule
//!   * KN — `get_kanban_blocks` returns marker rows with page provenance; the
//!     edit-dialog save path (`update_block` → `save_blocks`) persists marker +
//!     content to the `.md` file
//!   * WB — `save_whiteboard` writes `<name>.excalidraw`; `load_whiteboard`
//!     reads it back; `list_whiteboards` enumerates saved boards

mod common;

use app_lib::commands::vault::{AppState, VaultState};
use pkm_block::Page;
use serde_json::json;
use std::sync::Mutex;
use tauri::test::{mock_builder, mock_context, noop_assets};
use tauri::webview::InvokeRequest;
use tauri::WebviewWindow;

/// Build a mock Tauri app whose `VaultState` is backed by `vault`, with the
/// feature command handlers registered.
fn build_app(vault: &common::TestVault) -> tauri::App<tauri::test::MockRuntime> {
    let vs = VaultState::new(vault.vault_path.clone());
    mock_builder()
        .manage(Mutex::new(vs) as AppState)
        .invoke_handler(tauri::generate_handler![
            app_lib::commands::page::ensure_today_journal,
            app_lib::commands::template::save_template,
            app_lib::commands::template::apply_template,
            app_lib::commands::flashcards::generate_flashcards,
            app_lib::commands::flashcards::review_card,
            app_lib::commands::kanban::get_kanban_blocks,
            app_lib::commands::kanban::create_kanban_block,
            app_lib::commands::block::update_block,
            app_lib::commands::block::save_blocks,
            app_lib::commands::block::get_blocks,
            app_lib::commands::whiteboard::save_whiteboard,
            app_lib::commands::whiteboard::load_whiteboard,
            app_lib::commands::whiteboard::list_whiteboards,
            app_lib::commands::whiteboard::delete_whiteboard,
        ])
        .build(mock_context(noop_assets()))
        .expect("app build")
}

/// Drive one command invocation through the real IPC dispatcher and return the
/// deserialized JSON response or the rejection value.
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

/// Seed a page into the store with blocks deserialized from its on-disk body,
/// mirroring what `sync_filesystem_to_db` does on app boot.
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

// ---------------------------------------------------------------------------
// JN — daily journal auto-create
// ---------------------------------------------------------------------------

#[test]
fn command_ensure_today_journal_creates_file_on_disk() {
    let tv = common::create_test_vault();
    let app = build_app(&tv);
    let wv = webview(&app);

    let res = invoke(&wv, "ensure_today_journal", json!({})).expect("resolve");
    let path = res["path"].as_str().expect("path");
    assert!(
        path.starts_with("journals/") && path.ends_with(".md"),
        "journal must live under journals/: {path}"
    );
    let full = tv.vault_path.join(path);
    assert!(full.exists(), "journal file must exist on disk");
    let content = std::fs::read_to_string(&full).unwrap();
    assert!(
        content.contains("title:"),
        "journal must have frontmatter title"
    );
}

#[test]
fn command_ensure_today_journal_is_idempotent() {
    let tv = common::create_test_vault();
    let app = build_app(&tv);
    let wv = webview(&app);

    let first = invoke(&wv, "ensure_today_journal", json!({})).expect("resolve");
    let path = first["path"].as_str().unwrap().to_string();
    let full = tv.vault_path.join(&path);
    let before = std::fs::read_to_string(&full).unwrap();

    let second = invoke(&wv, "ensure_today_journal", json!({})).expect("resolve");
    assert_eq!(second["path"].as_str().unwrap(), path);
    let after = std::fs::read_to_string(&full).unwrap();
    assert_eq!(before, after, "re-ensure must not rewrite the journal");
}

// ---------------------------------------------------------------------------
// TM — template save + apply with variable substitution
// ---------------------------------------------------------------------------

#[test]
fn command_template_apply_writes_rendered_target_page() {
    let tv = common::create_test_vault();
    std::fs::create_dir_all(tv.vault_path.join("templates")).unwrap();
    std::fs::write(
        tv.vault_path.join("templates/meeting.md"),
        "# {{title}}\n\nDate: {{date}}\nTopic: {{topic}}",
    )
    .unwrap();

    let app = build_app(&tv);
    let wv = webview(&app);

    let res = invoke(
        &wv,
        "apply_template",
        json!({
            "templateName": "meeting",
            "targetPage": "pages/2026-09-19-standup.md",
            "variables": [["topic", "roadmap"]]
        }),
    )
    .expect("apply must resolve");

    let target = tv.vault_path.join("pages/2026-09-19-standup.md");
    assert!(target.exists(), "target page must be written to disk");
    let on_disk = std::fs::read_to_string(&target).unwrap();
    // Rendered content returned by the command == what landed on disk.
    let rendered = res.as_str().unwrap().to_string();
    assert_eq!(on_disk, rendered);
    assert!(
        on_disk.contains("Topic: roadmap"),
        "user variable substituted"
    );
    assert!(
        on_disk.contains("Date: 20"),
        "built-in date substituted: {:?}",
        on_disk
    );
}

#[test]
fn command_save_template_persists_to_disk_and_apply_is_idempotent() {
    let tv = common::create_test_vault();
    let app = build_app(&tv);
    let wv = webview(&app);

    invoke(
        &wv,
        "save_template",
        json!({ "name": "note", "content": "# {{title}}\n\nBody {{body}}" }),
    )
    .expect("save must resolve");

    let tpl = tv.vault_path.join("templates/note.md");
    assert!(tpl.exists(), "template file must be on disk");
    assert!(std::fs::read_to_string(&tpl).unwrap().contains("{{body}}"));

    // Apply twice — the second write must overwrite deterministically.
    invoke(
        &wv,
        "apply_template",
        json!({
            "templateName": "note",
            "targetPage": "pages/x.md",
            "variables": [["body", "one"]]
        }),
    )
    .expect("apply 1");
    invoke(
        &wv,
        "apply_template",
        json!({
            "templateName": "note",
            "targetPage": "pages/x.md",
            "variables": [["body", "two"]]
        }),
    )
    .expect("apply 2");
    let on_disk = std::fs::read_to_string(tv.vault_path.join("pages/x.md")).unwrap();
    assert!(on_disk.contains("Body two"));
    assert!(!on_disk.contains("Body one"));
}

// ---------------------------------------------------------------------------
// FC — flashcards generate + SM-2 review persistence
// ---------------------------------------------------------------------------

#[test]
fn command_flashcard_generate_and_review_uses_real_content() {
    let tv = common::create_test_vault();
    // A block whose CONTENT is the question and whose properties carry the
    // answer (documented contract — block content = question).
    let body = "---\ntitle: fc-note\n---\n\n- What is a monad?\n  .question: true\n  .answer: A design pattern.\n";
    seed_disk_page(&tv, "pages/fc-note.md", body);

    let app = build_app(&tv);
    let wv = webview(&app);

    let cards = invoke(&wv, "generate_flashcards", json!({})).expect("generate");
    let arr = cards.as_array().expect("array");
    assert_eq!(arr.len(), 1, "one card expected from the fixture");
    assert_eq!(
        arr[0]["front"].as_str().unwrap(),
        "What is a monad?",
        "front must be the block content, not the property value"
    );
    assert_eq!(arr[0]["back"].as_str().unwrap(), "A design pattern.");

    // SM-2 review on the generated card with a Good rating (q=3).
    let id = arr[0]["id"].as_str().unwrap().to_string();
    let page_path = arr[0]["page_path"].as_str().unwrap().to_string();
    let reviewed = invoke(
        &wv,
        "review_card",
        json!({ "cardId": id, "quality": 3, "pagePath": page_path }),
    )
    .expect("review must resolve");
    assert_eq!(
        reviewed["repetitions"].as_u64().unwrap(),
        1,
        "reps must increment"
    );
    assert_eq!(
        reviewed["interval_days"].as_u64().unwrap(),
        1,
        "first correct → 1 day"
    );
    // SM-2: a Good (q=3) rating slightly lowers ease below 2.5 but stays above
    // the 1.3 floor. Rejecting a nonsensical reset (<=0) double-checks the update.
    let ease = reviewed["ease_factor"].as_f64().unwrap();
    assert!(
        (1.3..2.5).contains(&ease),
        "q=3 must decrease ease toward the floor, got {ease}"
    );
    assert_eq!(
        reviewed["next_review"].as_str().unwrap(),
        (chrono::Utc::now() + chrono::Duration::days(1))
            .format("%Y-%m-%d")
            .to_string(),
        "next review = today + 1 day"
    );
}

// ---------------------------------------------------------------------------
// KN — kanban marker query + edit-dialog persistence (the DnD/edit contract)
// ---------------------------------------------------------------------------

#[test]
fn command_kanban_get_blocks_returns_marker_rows_with_page() {
    let tv = common::create_test_vault();
    let body = "---\ntitle: tasks\n---\n\n- ship MVP\n  .marker: TODO\n- write docs\n  .marker: DOING\n- sign off\n  .marker: DONE\n";
    seed_disk_page(&tv, "pages/tasks.md", body);

    let app = build_app(&tv);
    let wv = webview(&app);

    let res = invoke(&wv, "get_kanban_blocks", json!({})).expect("resolve");
    let blocks = res["blocks"].as_array().expect("blocks");
    assert_eq!(blocks.len(), 3);
    let markers: Vec<_> = blocks
        .iter()
        .map(|b| b["marker"].as_str().unwrap_or("").to_string())
        .collect();
    assert!(markers.contains(&"TODO".to_string()));
    assert!(markers.contains(&"DOING".to_string()));
    assert!(markers.contains(&"DONE".to_string()));
    // Provenance must survive.
    for b in blocks {
        assert_eq!(b["page_path"].as_str().unwrap(), "pages/tasks.md");
        assert!(b["content"].as_str().is_some());
    }
}

#[test]
fn command_kanban_create_block_writes_today_journal_to_disk() {
    let tv = common::create_test_vault();
    let app = build_app(&tv);
    let wv = webview(&app);

    let res = invoke(
        &wv,
        "create_kanban_block",
        json!({ "content": "new task", "marker": "TODO" }),
    )
    .expect("create must resolve");
    assert_eq!(res["marker"].as_str().unwrap(), "TODO");
    assert_eq!(res["content"].as_str().unwrap(), "new task");

    let page = res["page_path"].as_str().unwrap();
    let full = tv.vault_path.join(page);
    assert!(full.exists(), "today's journal must be on disk");
    let on_disk = std::fs::read_to_string(&full).unwrap();
    assert!(
        on_disk.contains("new task") && on_disk.contains(".marker: TODO"),
        "created task must serialize to the journal .md: {on_disk}"
    );
}

#[test]
fn command_kanban_edit_dialog_save_persists_marker_and_content_to_disk() {
    let tv = common::create_test_vault();
    let body = "---\ntitle: tasks\n---\n\n- ship MVP\n  .marker: TODO\n";
    seed_disk_page(&tv, "pages/tasks.md", body);

    // Resolve the real block id from the store.
    let blocks = tv.store.get_blocks_by_page("pages/tasks.md").unwrap();
    assert_eq!(blocks.len(), 1);
    let id = blocks[0].id.to_string();

    let app = build_app(&tv);
    let wv = webview(&app);

    // The edit dialog path (see KanbanEditDialog + KanbanPanel.handleEditSave):
    // update_block applies the new marker/content to SQLite, then save_blocks
    // reserializes the whole page to disk.
    invoke(
        &wv,
        "update_block",
        json!({
            "pagePath": "pages/tasks.md",
            "block": {
                "id": id,
                "content": "ship MVP (edited)",
                "parentId": null,
                "leftId": null,
                "properties": [],
                "marker": "DOING",
                "priority": "A",
                "collapsed": false,
                "headingLevel": null
            }
        }),
    )
    .expect("update must resolve");

    let all = invoke(&wv, "get_blocks", json!({ "pagePath": "pages/tasks.md" }))
        .expect("get must resolve");
    invoke(
        &wv,
        "save_blocks",
        json!({ "pagePath": "pages/tasks.md", "blocks": all["blocks"], "title": "tasks" }),
    )
    .expect("save must resolve");

    let on_disk = std::fs::read_to_string(tv.vault_path.join("pages/tasks.md")).unwrap();
    assert!(
        on_disk.contains("ship MVP (edited)"),
        "content persists: {on_disk}"
    );
    assert!(
        on_disk.contains(".marker: DOING"),
        "marker change persists to disk: {on_disk}"
    );
    assert!(!on_disk.contains(".marker: TODO"));
}

#[test]
fn command_kanban_dnd_update_without_save_does_not_persist_to_disk() {
    // KN — drag-and-drop marker change. The frontend `handleDragEnd` persists
    // via `updateBlock` (SQLite-only) and does NOT call `saveBlocks`, so the
    // marker change never reaches the `.md` file. On restart the disk sync
    // rebuilds the DB from disk and the change is lost. This test pins the
    // CURRENT contract so the gap is visible; fixing it requires issuing a
    // `save_blocks` after the DnD drop (see docs/acceptance-defects.md E7.F4).
    let tv = common::create_test_vault();
    let body = "---\ntitle: tasks\n---\n\n- ship MVP\n  .marker: TODO\n";
    seed_disk_page(&tv, "pages/tasks.md", body);
    let before = std::fs::read_to_string(tv.vault_path.join("pages/tasks.md")).unwrap();

    let block = &tv.store.get_blocks_by_page("pages/tasks.md").unwrap()[0];
    let id = block.id.to_string();

    let app = build_app(&tv);
    let wv = webview(&app);

    // DnD drop from todo → in_progress: frontend calls update_block, marker
    // DOING. This is exactly what handleDragEnd does (KanbanPanel.shared.tsx).
    invoke(
        &wv,
        "update_block",
        json!({
            "pagePath": "pages/tasks.md",
            "block": {
                "id": id,
                "content": "ship MVP",
                "parentId": null,
                "leftId": null,
                "properties": [],
                "marker": "DOING",
                "priority": null,
                "collapsed": false,
                "headingLevel": null
            }
        }),
    )
    .expect("update must resolve");

    // The store has moved (DB-only).
    let db_block = tv.store.get_block(id.parse().unwrap()).unwrap();
    assert_eq!(db_block.marker.map(|m| m.as_str()), Some("DOING"));
    // But the on-disk file still says TODO.
    let after = std::fs::read_to_string(tv.vault_path.join("pages/tasks.md")).unwrap();
    assert_eq!(before, after, "DnD marker change must NOT reach disk today");

    // Simulate the restart disk sync: re-sync the .md into a fresh store view.
    // The disk still has TODO, so the DOING marker is lost.
    let re_synced = pkm_markdown::block_parser::parse_document(&after);
    assert_eq!(re_synced.2[0].marker.map(|m| m.as_str()), Some("TODO"));
}

#[test]
fn command_review_card_persists_schedule_to_disk() {
    // FC — SM-2 review must persist the scheduling metadata (ease/interval/
    // reps/next_review) to the `.md` file so it survives an app restart.
    // (Regression: this was DB-only before E7.F4 and the boot-time disk sync
    // wiped the schedule on every restart.)
    let tv = common::create_test_vault();
    let body = "---\ntitle: fc-note\n---\n\n- What is a monad?\n  .question: true\n  .answer: A design pattern.\n";
    seed_disk_page(&tv, "pages/fc-note.md", body);
    let before = std::fs::read_to_string(tv.vault_path.join("pages/fc-note.md")).unwrap();

    let block = &tv.store.get_blocks_by_page("pages/fc-note.md").unwrap()[0];
    let id = block.id.to_string();

    let app = build_app(&tv);
    let wv = webview(&app);

    invoke(
        &wv,
        "review_card",
        json!({ "cardId": id, "quality": 3, "pagePath": "pages/fc-note.md" }),
    )
    .expect("review must resolve");

    // DB now holds the updated schedule.
    let db_block = tv.store.get_block(id.parse().unwrap()).unwrap();
    assert!(db_block.properties.contains_key("interval"));
    assert!(db_block.properties.contains_key("next_review"));

    // The schedule must also reach the .md file.
    let after = std::fs::read_to_string(tv.vault_path.join("pages/fc-note.md")).unwrap();
    assert_ne!(
        before, after,
        "review must rewrite the .md with the schedule"
    );
    assert!(
        after.contains(".interval: 1"),
        "interval must persist to disk: {after}"
    );
    assert!(
        after.contains(".reps: 1"),
        "reps must persist to disk: {after}"
    );
    // Frontmatter preserved through the rewrite.
    assert!(after.contains("title: fc-note"), "frontmatter preserved");

    // Restart sync would now converge to the persisted schedule, not lose it.
    let re_synced = pkm_markdown::block_parser::parse_document(&after);
    assert_eq!(
        re_synced.2[0]
            .properties
            .get("interval")
            .map(|s| s.as_str()),
        Some("1")
    );
}

// ---------------------------------------------------------------------------
// WB — whiteboard save / load / list
// ---------------------------------------------------------------------------

#[test]
fn command_whiteboard_save_load_list_roundtrip() {
    let tv = common::create_test_vault();
    let app = build_app(&tv);
    let wv = webview(&app);

    let scene = r##"{"type":"excalidraw","version":2,"elements":[{"id":"e1","type":"rectangle","x":10,"y":20,"width":100,"height":80}],"appState":{"viewBackgroundColor":"#ffffff"}}"##;

    invoke(
        &wv,
        "save_whiteboard",
        json!({ "name": "brainstorm", "content": scene }),
    )
    .expect("save must resolve");

    let wb_file = tv.vault_path.join("whiteboards/brainstorm.excalidraw");
    assert!(wb_file.exists(), "whiteboard file must be on disk");
    assert_eq!(
        std::fs::read_to_string(&wb_file).unwrap(),
        scene,
        "saved content must match on disk"
    );

    let loaded = invoke(&wv, "load_whiteboard", json!({ "name": "brainstorm" }))
        .expect("load must resolve")
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(loaded, scene, "load must return exactly what was saved");

    let listed = invoke(&wv, "list_whiteboards", json!({})).expect("list must resolve");
    let boards = listed.as_array().expect("array");
    assert_eq!(boards.len(), 1);
    assert_eq!(boards[0]["name"].as_str().unwrap(), "brainstorm");
    assert_eq!(boards[0]["content"].as_str().unwrap(), scene);
}

#[test]
fn command_whiteboard_load_missing_returns_empty_scene_and_delete_removes_file() {
    let tv = common::create_test_vault();
    let app = build_app(&tv);
    let wv = webview(&app);

    let loaded = invoke(&wv, "load_whiteboard", json!({ "name": "ghost" }))
        .expect("load missing must resolve (not reject)");
    assert!(
        loaded.as_str().unwrap_or("").contains("elements"),
        "missing board returns an empty scene, not an error"
    );

    // Save then delete → file gone + list empty.
    invoke(
        &wv,
        "save_whiteboard",
        json!({ "name": "temp", "content": "{\"elements\":[]}" }),
    )
    .expect("save");
    let wb_file = tv.vault_path.join("whiteboards/temp.excalidraw");
    assert!(wb_file.exists());
    invoke(&wv, "delete_whiteboard", json!({ "name": "temp" })).expect("delete");
    assert!(!wb_file.exists(), "delete must remove the file on disk");
    let listed = invoke(&wv, "list_whiteboards", json!({})).expect("list");
    assert_eq!(listed.as_array().unwrap().len(), 0);
}
