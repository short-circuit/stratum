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

/// Task acceptance (t_03247cfb): corrupted quad-bracket wiki-links must heal into
/// clean `links` rows AND produce real graph edges through the actual Tauri IPC
/// command path. This is the end-to-end shape the acceptance card demands:
/// A.md with `See [[[[Docker]]]]` + Docker.md page → sync → heal → graph command
/// returns source == "a" / target == "docker", with idempotent re-heal.
#[test]
fn corrupted_quad_bracket_links_produce_graph_edges() {
    let tv = common::create_test_vault();
    seed_page(
        &tv,
        "pages/docker.md",
        "Docker",
        &[],
        "---\ntitle: Docker\n---\n- Docker basics\n",
    );
    seed_page(
        &tv,
        "pages/a.md",
        "A",
        &[],
        "---\ntitle: A\n---\n- See [[[[Docker]]]] for details\n",
    );

    // Corrupt state: blocks exist but the links table has no rows for the source
    // page (simulates a pre-fix vault where reconcile never ran).
    tv.store.delete_links_for_page("pages/a.md").unwrap();
    let before = tv.store.get_page_ref_edges().unwrap();
    assert_eq!(before.len(), 0, "pre-heal: no page_ref rows at all");

    // Boot-time heal restores the row with a CLEAN target.
    let healed = app_lib::commands::page::heal_missing_page_links(&tv.store).unwrap();
    assert_eq!(healed, 1, "exactly one page owns a recoverable link");

    let rows = tv.store.get_page_ref_edges().unwrap();
    assert_eq!(rows.len(), 1, "links table: exactly 1 page_ref row");
    assert_eq!(rows[0].0, "pages/a.md");
    assert_eq!(rows[0].1, "Docker", "target stored clean — no brackets");

    // Idempotency: re-run must heal nothing and change no rows.
    let healed2 = app_lib::commands::page::heal_missing_page_links(&tv.store).unwrap();
    assert_eq!(healed2, 0, "second heal run: nothing new to heal");
    let rows2 = tv.store.get_page_ref_edges().unwrap();
    assert_eq!(rows2.len(), 1, "row count unchanged after re-heal");
    assert_eq!(rows2[0].1, "Docker");

    // The graph command (real IPC dispatch) returns the healed edge with clean slugs.
    graph::invalidate_graph_cache();
    let app = build_app(&tv);
    let wv = webview(&app);
    let resp = invoke(&wv, "get_graph_panel_data", json!({})).expect("graph loads");
    let edges = resp["graph"]["edges"].as_array().expect("edges array");
    assert_eq!(edges.len(), 1, "graph must expose the healed edge");
    assert_eq!(
        edges[0]["source"], "a",
        "source slug resolved from pages/a.md"
    );
    assert_eq!(edges[0]["target"], "docker", "target slug resolved cleanly");
}

/// Acceptance smoke (t_03247cfb job 2): drive the REAL `sync_filesystem_to_db`
/// and `get_page_ref_edges` path against a copy of the live vault at
/// `/home/shrtcrct/StratumVault`. Gated `#[ignore]`; run manually with the test
/// command `cargo test -p stratum-tauri --test graph_link_healing -- --ignored
/// real_vault_copy_heals_corrupted_links_and_restores_edges --nocapture`.
///
/// The live vault is ALREADY healed (its `links` table has 5,082 page_ref rows
/// from a prior boot), so to reproduce the CORRUPT state the harness copies the
/// vault, nulls the `links` table (the pre-fix state), then runs the sync/heal
/// path. It asserts:
///   * before: 0 page_ref rows (simulated corruption),
///   * after:   ≥ 500 rows, exact count reported,
///   * the known edge `Networking.md → Docker` (Networking.md:15) resolves with
///     clean target `Docker` (no brackets),
///   * idempotent re-run: 0 new rows.
#[test]
#[ignore]
fn real_vault_copy_heals_corrupted_links_and_restores_edges() {
    let mut src = std::path::PathBuf::from("/home/shrtcrct/StratumVault");
    if let Ok(alt) = std::env::var("STRATUM_REAL_VAULT") {
        src = alt.into();
    }
    assert!(src.exists(), "live vault must exist");

    let tmp = tempfile::tempdir().expect("temp dir");
    let copy = tmp.path().join("vault-copy");

    // Copy the LIVE vault's blocks.db (it already holds the page/block rows) and
    // the .md tree. Then clear the `links` table to reproduce the PRE-FIX
    // corrupted state (blocks exist but no link rows).
    std::fs::create_dir_all(copy.join(".pkm")).expect("create .pkm");
    std::fs::copy(
        src.join(".pkm").join("blocks.db"),
        copy.join(".pkm").join("blocks.db"),
    )
    .expect("copy blocks.db");
    copy_tree(&src, &copy);

    let db_path = copy.join(".pkm").join("blocks.db");
    // Reproduce the corrupted state: pre-fix DBs have empty `links` tables.
    {
        let store = pkm_block::BlockStore::open(&db_path).expect("open fresh db");
        store
            .execute_batch("DELETE FROM links")
            .expect("clear links");
        let before = store.get_page_ref_edges().expect("edges pre-heal");
        assert_eq!(before.len(), 0, "corrupted state: no edges");
        println!("BEFORE: 0 page_ref rows (links table cleared to pre-fix state)");
    }

    let synced = app_lib::commands::page::sync_filesystem_to_db(&copy, &db_path)
        .expect("sync_filesystem_to_db");
    println!("SYNCED pages/blocks count: {synced}");

    let store = pkm_block::BlockStore::open(&db_path).expect("reopen db");
    let edges = store.get_page_ref_edges().expect("edges post-heal");
    println!("AFTER: {} page_ref rows", edges.len());
    assert!(
        edges.len() >= 500,
        "expected >= 500 recovered page_ref edges for the real vault, got {}",
        edges.len()
    );

    // Known edge: Networking.md:15 has `[[Docker]]` (in the real vault the file
    // uses clean `[[Docker]]`; the corrupting path is the pre-fix extractor that
    // stored `[[Docker]]`). What we assert here is the row resolves with a CLEAN
    // target so the graph slug resolution can match `Docker.md`.
    let net_docker: Vec<_> = edges
        .iter()
        .filter(|(s, t)| s.ends_with("Networking.md") && t == "Docker")
        .collect();
    assert_eq!(
        net_docker.len(),
        1,
        "exactly one Networking.md -> Docker edge, got {:?}",
        net_docker
    );
    println!("KNOWN EDGE: Networking.md -> Docker present, clean target");

    // Idempotency: re-running sync must not inflate the count.
    let synced2 =
        app_lib::commands::page::sync_filesystem_to_db(&copy, &db_path).expect("second sync");
    println!("SYNCED (2nd run): {synced2} (expect 0 new)");
    let edges2 = store.get_page_ref_edges().expect("edges after 2nd sync");
    assert_eq!(
        edges2.len(),
        edges.len(),
        "row count must be stable across re-sync"
    );
}

/// Copy a directory tree (files + subdirs) preserving relative structure.
/// Skips `.git` and Tantivy `search` dirs to keep the copy light; `MdCollector`
/// only reads `.md` files, so the search index is irrelevant for the sync path.
fn copy_tree(src: &std::path::Path, dst: &std::path::Path) {
    std::fs::create_dir_all(dst).expect("create dst");
    for entry in std::fs::read_dir(src).expect("read src dir") {
        let entry = entry.expect("entry");
        let name = entry.file_name();
        let name_str = name.to_string_lossy().to_string();
        if name_str == ".git" || name_str == "search" || name_str == "search.idx" {
            continue;
        }
        let from = entry.path();
        let to = dst.join(&name);
        if from.is_dir() {
            copy_tree(&from, &to);
        } else {
            if let Some(parent) = to.parent() {
                std::fs::create_dir_all(parent).ok();
            }
            std::fs::copy(&from, &to).expect("copy file");
        }
    }
}
