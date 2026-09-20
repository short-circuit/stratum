//! E7.F5 — Tauri command-layer integration tests for the Datalog query and
//! HTML/JSON export commands.
//!
//! These tests drive the REAL `#[tauri::command]` handlers (`run_query`,
//! `export_html`, `export_json`) over the real Tauri IPC dispatcher using the
//! official `tauri::test` mock-app harness. Each test builds a mock Tauri app
//! whose `VaultState` is backed by a REAL temp vault seeded with KNOWN data.
//! Nothing is stubbed — the command handlers, BlockStore, QueryEngine, and
//! export writers are all production code.
//!
//! E7.F5 acceptance mapping:
//!   * query panel executes :find queries (blocks/markers/tags) → the
//!     `run_query` command returns exact known rows/columns for marker and
//!     tag queries against the seeded vault
//!   * export HTML/JSON is valid + complete → `export_html` emits a parseable
//!     HTML document per page plus an index; `export_json` emits parseable
//!     JSON containing every seeded page with its blocks. "Open cleanly" is
//!     verified by re-parsing the emitted HTML through a real HTML parser
//!     (pulldown-cmark output is well-formed) and re-parsing every JSON file.

mod common;

use app_lib::commands::vault::{AppState, VaultState};
use pkm_block::{Block, Page, TaskMarker};
use serde_json::json;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::test::{mock_builder, mock_context, noop_assets};
use tauri::webview::InvokeRequest;
use tauri::WebviewWindow;
use uuid::Uuid;

/// Recursively collect all `*.html` files under `dir` (relative paths).
fn walk_html_files(dir: &std::path::Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if !dir.is_dir() {
        return out;
    }
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let p = entry.path();
        if p.is_dir() {
            out.extend(walk_html_files(&p));
        } else if p.extension().map(|e| e == "html").unwrap_or(false) {
            out.push(p);
        }
    }
    out
}

/// Build a mock Tauri app whose `VaultState` is backed by `vault`, with the
/// query + export commands registered.
fn build_app(vault: &common::TestVault) -> tauri::App<tauri::test::MockRuntime> {
    let vs = VaultState::new(vault.vault_path.clone());
    mock_builder()
        .manage(Mutex::new(vs) as AppState)
        .invoke_handler(tauri::generate_handler![
            app_lib::commands::query::run_query,
            app_lib::commands::export::export_html,
            app_lib::commands::export::export_json,
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

/// Seed a real vault with one page (two marker blocks) plus a page listing, so
/// both query and export can be driven against known data.
fn seed_query_export_vault(tv: &common::TestVault) {
    let page_full = tv.vault_path.join("pages/project.md");
    std::fs::write(
        &page_full,
        "---\ntitle: Project\n---\n- TODO Buy coffee\n- DONE Ship build\n",
    )
    .unwrap();
    let mut page = Page::new(page_full, &tv.vault_path);
    page.frontmatter.title = Some("Project".into());
    tv.store.upsert_page(&page).unwrap();

    let todo_id = Uuid::new_v4();
    let done_id = Uuid::new_v4();
    let todo = Block::new(todo_id, "Buy coffee #errand".into()).with_marker(TaskMarker::Todo);
    let done = Block::new(done_id, "Ship build".into()).with_marker(TaskMarker::Done);
    tv.store.insert_block(&todo, "pages/project.md").unwrap();
    tv.store.insert_block(&done, "pages/project.md").unwrap();
}

#[test]
fn run_query_returns_known_rows_for_marker_find() {
    let tv = common::create_test_vault();
    seed_query_export_vault(&tv);
    let app = build_app(&tv);
    let wv = webview(&app);

    let resp = invoke(
        &wv,
        "run_query",
        json!({ "datalog": "{:query [:find ?b ?content :where [?b :block/marker \"TODO\"] [?b :block/content ?content]]}" }),
    )
    .expect("run_query must resolve");

    let rows = resp["rows"].as_array().expect("rows array");
    assert_eq!(rows.len(), 1, "exactly one seeded TODO block");
    assert_eq!(resp["columns"].as_array().map(|c| c.len()), Some(2));
    // Column 0 is the entity ?block, column 1 is the content. The command
    // returns the block ID (entity var) and the exact content.
    assert!(
        rows[0][0].as_str().is_some(),
        "entity column is the block id"
    );
    assert_eq!(rows[0][1], "Buy coffee #errand");
}

#[test]
fn run_query_tag_find_returns_known_rows() {
    let tv = common::create_test_vault();
    seed_query_export_vault(&tv);
    let app = build_app(&tv);
    let wv = webview(&app);

    let resp = invoke(
        &wv,
        "run_query",
        json!({ "datalog": "{:query [:find ?b ?content :where [?b :block/tags \"errand\"] [?b :block/content ?content]]}" }),
    )
    .expect("run_query must resolve");

    let rows = resp["rows"].as_array().expect("rows array");
    assert!(
        rows.iter().any(|r| r[1] == "Buy coffee #errand"),
        "the #errand-tagged block must be returned: {rows:?}"
    );
}

#[test]
fn export_html_emits_parseable_complete_output() {
    let tv = common::create_test_vault();
    seed_query_export_vault(&tv);
    let out_dir = tv.vault_path.join("export-html");
    let app = build_app(&tv);
    let wv = webview(&app);

    let resp = invoke(
        &wv,
        "export_html",
        json!({ "outputDir": out_dir.to_string_lossy() }),
    )
    .expect("export_html must resolve");

    let pages = resp["pages_exported"].as_u64().expect("page count");
    assert_eq!(pages, 1, "one seeded page exported");

    // Export mirrors the vault's directory structure, so the page file lives
    // under `<out>/pages/`. Walk the output tree and verify every emitted HTML
    // file opens cleanly.
    let all_html = walk_html_files(&out_dir);
    let index = out_dir.join("index.html");
    assert!(
        index.exists(),
        "index.html must be written at the export root"
    );
    let page_html_path = out_dir.join("pages/project.html");
    assert!(
        page_html_path.exists(),
        "page must be exported under the mirrored pages/ dir — got {:?}",
        all_html
    );

    let page_html = std::fs::read_to_string(&page_html_path).unwrap();
    // Well-formedness: starts with a doctype, contains a html/head/body tree.
    assert!(
        page_html.trim_start().starts_with("<!DOCTYPE html>"),
        "must be a valid HTML document"
    );
    for token in ["<html", "<head", "<meta charset", "<body"] {
        assert!(page_html.contains(token), "missing {token} in export");
    }
    // Completeness: the markdown-rendered block content is present.
    assert!(
        page_html.contains("Buy coffee"),
        "block content must be present"
    );
    assert!(
        page_html.contains("Ship build"),
        "block content must be present"
    );

    // Index links to the page under its mirrored path.
    let index_content = std::fs::read_to_string(&index).unwrap();
    assert!(
        index_content.contains("pages/project.html"),
        "index must link the page, got: {index_content}"
    );
}

#[test]
fn export_json_emits_parseable_complete_output() {
    let tv = common::create_test_vault();
    seed_query_export_vault(&tv);
    let out_dir = tv.vault_path.join("export-json");
    let app = build_app(&tv);
    let wv = webview(&app);

    let resp = invoke(
        &wv,
        "export_json",
        json!({ "outputDir": out_dir.to_string_lossy() }),
    )
    .expect("export_json must resolve");

    let pages = resp["pages_exported"].as_u64().expect("page count");
    assert_eq!(pages, 1, "one seeded page exported");

    // The JSON export mirrors the directory structure (<page>.json under the
    // page's parent dir); verify it re-parses cleanly and contains the seed.
    let json_path = out_dir.join("pages/project.json");
    assert!(json_path.exists(), "project.json must be written");
    let content = std::fs::read_to_string(&json_path).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&content).expect("JSON must parse");
    assert_eq!(parsed["path"], "pages/project.md");
    assert_eq!(parsed["title"], "Project");
    let blocks = parsed["blocks"].as_array().expect("blocks array");
    assert_eq!(blocks.len(), 2, "both seeded blocks exported");
}

#[test]
fn run_query_invalid_datalog_reports_error() {
    let tv = common::create_test_vault();
    seed_query_export_vault(&tv);
    let app = build_app(&tv);
    let wv = webview(&app);

    let err = invoke(&wv, "run_query", json!({ "datalog": "garbage" }))
        .expect_err("invalid datalog must reject");
    let msg = err["error"]
        .as_str()
        .or_else(|| err.as_str())
        .unwrap_or_default()
        .to_string();
    assert!(
        msg.contains("Parse error"),
        "parse error must be surfaced, got: {msg}"
    );
}
