//! Tauri command-level integration tests for the boot-time `links` heal and
//! bracket-wrapped wiki-link target normalization (t_f2344ad9).
//!
//! Regression: corrupted legacy content (`[[[[Docker]]]]`) leaks bracket-wrapping
//! into extracted link targets; the graph then drops the edge because resolution
//! never matches a slug. These tests drive the REAL `#[tauri::command]` handler
//! (`get_graph_panel_data`) over the real Tauri IPC dispatcher plus the pure
//! command-layer heal function, asserting:
//!   * `extract_links` yields a clean target (unit-tested in pkm-markdown),
//!   * the boot-time `heal_missing_page_links` restores the missing `links` row
//!     with a CLEAN target,
//!   * the heal is idempotent (a converged vault returns 0 on re-run),
//!   * the graph command returns the healed edge with clean slugs.
//!
//! NOTE on the process-global cache: `get_graph_panel_data` caches its result in a
//! process-global `OnceLock` (`GRAPH_CACHE`) that is NOT keyed by vault. The mock
//! harness intentionally calls `invalidate_graph_cache()` between fixtures so a
//! stale payload from one temp vault cannot leak into the next.

mod common;

use app_lib::commands::graph;
use app_lib::commands::vault::{AppState, VaultState};
use pkm_block::Page;
use serde_json::json;
use std::sync::Mutex;
use tauri::test::{mock_builder, mock_context, noop_assets};
use tauri::webview::InvokeRequest;
use tauri::WebviewWindow;

/// Build a mock Tauri app whose `VaultState` is backed by `vault`, with the
/// graph panel command registered.
fn build_app(vault: &common::TestVault) -> tauri::App<tauri::test::MockRuntime> {
    let vs = VaultState::new(vault.vault_path.clone());
    mock_builder()
        .manage(Mutex::new(vs) as AppState)
        .invoke_handler(tauri::generate_handler![
            app_lib::commands::graph::get_graph_panel_data,
        ])
        .build(mock_context(noop_assets()))
        .expect("app build")
}

/// Drive one command invocation through the real IPC dispatcher and return the
/// deserialized JSON response.
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

/// Seed a page (with optional frontmatter tags) plus a body block in SQLite —
/// the shape the reindex/sync code path produces. Body-block wiki-links are
/// reconciled into the `links` table (as the production write paths do) so graph
/// tests exercise the real read path — edges come from `links`, and the graph
/// command no longer heals `links` at read time.
fn seed_page(tv: &common::TestVault, rel_path: &str, title: &str, tags: &[&str], content: &str) {
    let full = tv.vault_path.join(rel_path);
    std::fs::write(&full, content).unwrap();
    let mut page = Page::new(full, &tv.vault_path);
    page.frontmatter.title = Some(title.to_string());
    page.frontmatter.tags = tags.iter().map(|s| s.to_string()).collect();
    tv.store.upsert_page(&page).unwrap();
    let (_fm, _, blocks) = pkm_markdown::block_parser::parse_document(content);
    for b in &blocks {
        tv.store.insert_block(b, rel_path).unwrap();
    }
    // Reproduce the production write path: reconcile this page's wiki-links into
    // the `links` table (source of the graph's authoritative edge reads).
    app_lib::commands::page::reconcile_page_links(&tv.store, rel_path, &blocks).unwrap();
}

/// Regression (t_f2344ad9): corrupted legacy content `[[[[Docker]]]]` must extract
/// to clean target `Docker`; the boot-time heal must restore the `links` row; the
/// graph must resolve the edge to clean slugs; the heal must be idempotent.
#[test]
fn sync_heals_bracket_wrapped_links_and_graph_returns_edge() {
    let tv = common::create_test_vault();
    seed_page(
        &tv,
        "pages/docker.md",
        "Docker",
        &[],
        "---\ntitle: Docker\n---\n- Docker basics\n",
    );
    // Corrupted legacy page: quadruple-bracket target, as found in live vaults.
    seed_page(
        &tv,
        "pages/source.md",
        "Source",
        &[],
        "---\ntitle: Source\n---\n- See [[[[Docker]]]] for details\n",
    );

    // Simulate a stale pre-heal DB: seed_page reconciled rows already, so delete
    // them to reproduce the "blocks exist but links table empty" state the boot
    // heal is meant to repair.
    tv.store.delete_links_for_page("pages/source.md").unwrap();

    // Boot-time heal restores exactly one clean row.
    let healed = app_lib::commands::page::heal_missing_page_links(&tv.store).unwrap();
    assert_eq!(
        healed, 1,
        "source page owns an extractable link and zero rows -> healed"
    );

    // Idempotency: a second run must do nothing.
    let healed2 = app_lib::commands::page::heal_missing_page_links(&tv.store).unwrap();
    assert_eq!(healed2, 0, "already-healed page must not be re-counted");

    let rows = tv.store.get_page_ref_edges().unwrap();
    assert_eq!(rows.len(), 1, "exactly one edge row");
    assert_eq!(rows[0].0, "pages/source.md");
    assert_eq!(rows[0].1, "Docker", "target stored clean, no brackets");

    // The graph command returns the edge with clean slugs.
    graph::invalidate_graph_cache();
    let app = build_app(&tv);
    let wv = webview(&app);
    let resp = invoke(&wv, "get_graph_panel_data", json!({})).expect("graph loads");
    let edges = resp["graph"]["edges"].as_array().expect("edges array");
    assert_eq!(edges.len(), 1, "graph must return the healed edge");
    assert_eq!(edges[0]["source"], "source");
    assert_eq!(edges[0]["target"], "docker");
}

/// Full boot-path regression: the production `sync_filesystem_to_db` (startup
/// sync) must heal a vault whose `links` table is empty for a page containing
/// bracket-wrapped `[[[[Docker]]]]` content, writing CLEAN targets — the exact
/// "expected outcome" of t_f2344ad9. Also asserts the count converges on re-run.
#[test]
fn sync_filesystem_to_db_heals_bracket_wrapped_links_clean() {
    let tv = common::create_test_vault();
    // Seed on-disk files only — the DB starts empty, mirroring a fresh import of
    // a legacy vault with corrupted content.
    tv.create_md_file(
        "pages/docker.md",
        "---\ntitle: Docker\n---\n- Docker basics\n",
    );
    tv.create_md_file(
        "pages/source.md",
        "---\ntitle: Source\n---\n- See [[[[Docker]]]] for details\n",
    );

    // First run: the startup sync parses both files and reconciles the links
    // table through the fixed extractor — the bracket-wrapped target must be
    // stored clean ("Docker", not "[[Docker]]").
    let first = app_lib::commands::page::sync_filesystem_to_db(&tv.vault_path, &tv.db_path)
        .expect("startup sync succeeds");
    assert_eq!(first, 2, "both pages synced from disk");

    let mut rows = tv.store.get_page_ref_edges().unwrap();
    assert_eq!(rows.len(), 1, "one edge row from the corrupted page");
    assert_eq!(rows[0].1, "Docker", "target stored clean, no brackets");

    // Second run: converged vault — nothing new to heal (idempotency). The
    // page with no wiki-links (docker.md) is legitimately zero-link and skipped.
    let second = app_lib::commands::page::sync_filesystem_to_db(&tv.vault_path, &tv.db_path)
        .expect("second sync succeeds");
    assert_eq!(second, 0, "converged vault reports zero new pages");

    rows = tv.store.get_page_ref_edges().unwrap();
    assert_eq!(rows.len(), 1, "row count stable across re-sync");
    assert_eq!(rows[0].1, "Docker");
}
