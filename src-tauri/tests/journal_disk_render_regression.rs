//! QA regression: an on-disk journal `.md` file that is absent from SQLite
//! (or whose block rows are stale) MUST be rendered in the journal view without
//! an app restart.
//!
//! Reproduces t_12ffd5ff — user reported journal entries existing as files on
//! disk but not rendered in the journal view. The journal panel derives its
//! `pastDates` list exclusively from `list_pages` (SQLite `pages` table), and
//! `get_blocks`/`open_page` read blocks exclusively from SQLite with no disk
//! fallback. `sync_filesystem_to_db` only heals missing pages at startup, so a
//! file that lands on disk after boot (external edit, git pull, interrupted
//! save) is invisible until restart.
//!
//! Repro performed against a COPY of the live vault (`~/StratumVault`): the
//! live vault's journal files use the real BlockNote-serialized format
//! (`- bullet\n  .id: <uuid>`) and the DB was consistent at inspection, so the
//! drift shape (file on disk, page/blocks absent from SQLite) is manufactured
//! in a temp vault copy. The three read commands (`list_pages`, `get_blocks`,
//! `open_page`) are driven over the REAL Tauri IPC dispatcher (official
//! `tauri::test` mock harness) — nothing is stubbed.
//!
//! On the pre-fix code these assertions FAIL: the hidden journal file is absent
//! from `list_pages`, `get_blocks` returns zero rows, and `open_page` reports
//! block_count 0 — exactly the "not rendered" user symptom.
//!
//! Verified against the sibling fix branch `wt/t_c6e43085` (commit 6d341c3):
//! `get_blocks_heal_from_disk` (wired into `get_blocks`/`open_page`) makes
//! tests 2 and 3 pass, but `list_pages` is NOT reconciled there, so test 1
//! still FAILS — the disk-only journal is absent from the `pages` table and
//! remains invisible to the journal panel. The backend fix must also surface
//! on-disk pages through `list_pages` for this test to go fully green.

mod common;

use app_lib::commands::vault::{AppState, VaultState};
use serde_json::json;
use std::sync::Mutex;
use tauri::test::{mock_builder, mock_context, noop_assets};
use tauri::webview::InvokeRequest;
use tauri::WebviewWindow;

/// Build a mock Tauri app backed by `vault` with the page/block read commands
/// registered — the exact handlers the journal view calls.
fn build_app(vault: &common::TestVault) -> tauri::App<tauri::test::MockRuntime> {
    let vs = VaultState::new(vault.vault_path.clone());
    mock_builder()
        .manage(Mutex::new(vs) as AppState)
        .invoke_handler(tauri::generate_handler![
            app_lib::commands::page::list_pages,
            app_lib::commands::page::open_page,
            app_lib::commands::page::ensure_today_journal,
            app_lib::commands::block::get_blocks,
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

const HIDDEN_JOURNAL: &str = "journals/2026-05-15.md";

/// Body in the exact real format the live vault's journal files use
/// (BlockNote-serialized bullets with `.id:` lines), plus frontmatter.
const HIDDEN_JOURNAL_BODY: &str = "\
---
title: 2026-05-15
---

- Studying Systems Programming at depth. The advanced materials are rewarding
  .id: a83f41d8-90ca-4613-894f-ae54b1ec266f
- Pair programming session on Design Thinking
  .id: a55d7545-8af5-44da-be2a-c466bde1727e
- Refactored the Zettelkasten module
  .id: 9c5e4491-c9a7-4533-bc67-4165acba4895
";

/// Precondition guard: a journal page that exists on disk as a real file but is
/// entirely absent from SQLite (no `pages` row, no `blocks` rows). This is the
/// drift shape the user hit — a `git pull` or external edit created the file
/// after last boot, so `sync_filesystem_to_db` never saw it.
#[test]
fn hidden_journal_file_absent_from_db_renders_in_journal_view() {
    let tv = common::create_test_vault();
    // The journal exists on disk ONLY — SQLite knows nothing about it.
    let on_disk = tv.create_md_file(HIDDEN_JOURNAL, HIDDEN_JOURNAL_BODY);
    assert!(on_disk.exists(), "on-disk journal file must exist");

    // Precondition: the page is NOT in the SQLite pages table and has no blocks.
    let db_pages = tv.store.list_pages().unwrap();
    assert!(
        !db_pages.iter().any(|p| p == HIDDEN_JOURNAL),
        "precondition: hidden journal must be absent from SQLite pages"
    );
    let db_blocks = tv.store.get_blocks_by_page(HIDDEN_JOURNAL).unwrap();
    assert!(
        db_blocks.is_empty(),
        "precondition: hidden journal must have no SQLite blocks"
    );

    let app = build_app(&tv);
    let wv = webview(&app);

    // 1) list_pages — the journal panel's pastDates source (JournalPanel.shared.tsx
    //    filters pages to `journals/*.md`). The on-disk journal MUST be listed.
    let listed = invoke(&wv, "list_pages", json!({}))
        .unwrap_or_else(|e| panic!("list_pages should resolve, got rejection: {e}"));
    let pages = listed["pages"].as_array().expect("pages array");
    let paths: Vec<&str> = pages.iter().filter_map(|p| p["path"].as_str()).collect();
    assert!(
        paths.contains(&HIDDEN_JOURNAL),
        "list_pages must surface the on-disk journal, got paths: {paths:?}"
    );
    // No duplicate of the hidden journal may appear.
    assert_eq!(
        paths.iter().filter(|p| *p == &HIDDEN_JOURNAL).count(),
        1,
        "the on-disk journal must appear exactly once"
    );

    // 2) get_blocks — renders the entry content. The on-disk journal's real
    //    blocks MUST be returned (not zero rows).
    let all = invoke(&wv, "get_blocks", json!({ "pagePath": HIDDEN_JOURNAL }))
        .unwrap_or_else(|e| panic!("get_blocks should resolve, got rejection: {e}"));
    let blocks = all["blocks"].as_array().expect("blocks array");
    assert_eq!(
        blocks.len(),
        3,
        "get_blocks must return the 3 on-disk blocks, got {blocks:?}"
    );
    assert!(
        blocks.iter().any(|b| b["content"]
            .as_str()
            .map(|c| c.contains("Systems Programming"))
            == Some(true)),
        "returned blocks must carry the on-disk content: {blocks:?}"
    );

    // 3) open_page — the header metadata source. block_count must reflect disk.
    let page = invoke(&wv, "open_page", json!({ "path": HIDDEN_JOURNAL }))
        .unwrap_or_else(|e| panic!("open_page should resolve, got rejection: {e}"));
    assert_eq!(
        page["block_count"].as_u64(),
        Some(3),
        "open_page must report the on-disk block count, got: {page:?}"
    );
    assert!(
        page["path"].as_str().map(|p| p.ends_with(HIDDEN_JOURNAL)) == Some(true),
        "open_page must return the journal page: {page:?}"
    );
}

/// Same drift class one step further along: the page IS registered in SQLite
/// (a stale `pages` row) but its `blocks` rows are missing while the on-disk
/// file has real content — the exact shape found in the live vault previously
/// ("Borrowing.md"). The read path must self-heal / fall back to disk and the
/// DB must not end up with duplicates.
#[test]
fn stale_page_with_missing_block_rows_renders_blocks_from_disk() {
    let tv = common::create_test_vault();
    tv.create_md_file(HIDDEN_JOURNAL, HIDDEN_JOURNAL_BODY);
    // SQLite: page row EXISTS but has ZERO block rows (the decayed shape).
    tv.add_page(HIDDEN_JOURNAL);

    // Precondition: stale DB — page listed, zero blocks.
    let db_blocks = tv.store.get_blocks_by_page(HIDDEN_JOURNAL).unwrap();
    assert!(
        db_blocks.is_empty(),
        "precondition: stale DB has no block rows"
    );

    let app = build_app(&tv);
    let wv = webview(&app);

    // get_blocks must fall back to disk and return the real blocks.
    let all = invoke(&wv, "get_blocks", json!({ "pagePath": HIDDEN_JOURNAL }))
        .unwrap_or_else(|e| panic!("get_blocks should resolve, got rejection: {e}"));
    let blocks = all["blocks"].as_array().expect("blocks array");
    assert_eq!(
        blocks.len(),
        3,
        "get_blocks must return the 3 on-disk blocks when DB rows are stale, got {blocks:?}"
    );

    // The read path must converge the DB (no duplication): a second read is
    // stable, and the DB now holds rows.
    let after = tv.store.get_blocks_by_page(HIDDEN_JOURNAL).unwrap();
    assert_eq!(
        after.len(),
        3,
        "DB must be healed from disk, not left stale"
    );

    // open_page must report the disk block count.
    let page = invoke(&wv, "open_page", json!({ "path": HIDDEN_JOURNAL }))
        .unwrap_or_else(|e| panic!("open_page should resolve, got rejection: {e}"));
    assert_eq!(
        page["block_count"].as_u64(),
        Some(3),
        "open_page must report the on-disk block count, got: {page:?}"
    );
}

/// Regression guard against over-correction: an existing DB-backed entry must
/// be unaffected and no duplicate may be introduced. A page that is fully
/// healthy in SQLite (row + blocks) must read back exactly its DB blocks with
/// no disk round-trip changing anything.
#[test]
fn existing_db_entry_is_unaffected_and_no_duplicates() {
    let tv = common::create_test_vault();
    // Seeded DB page with real blocks (existing, healthy).
    let body = "\
---
title: 2026-09-17
---

- Morning planning
  .id: 11111111-1111-4111-8111-111111111111
";
    let full = tv.vault_path.join("journals/2026-09-17.md");
    std::fs::create_dir_all(full.parent().unwrap()).unwrap();
    std::fs::write(&full, body).unwrap();
    let (_fm, _, blocks) = pkm_markdown::block_parser::parse_document(body);
    let mut page = pkm_block::Page::new(full, &tv.vault_path);
    page.set_blocks(&blocks);
    tv.store.upsert_page(&page).unwrap();
    for b in &blocks {
        tv.store.insert_block(b, "journals/2026-09-17.md").unwrap();
    }

    let db_before = tv.store.list_pages().unwrap();
    assert_eq!(db_before.len(), 1);

    let app = build_app(&tv);
    let wv = webview(&app);

    // Read paths on the healthy entry.
    let all = invoke(
        &wv,
        "get_blocks",
        json!({ "pagePath": "journals/2026-09-17.md" }),
    )
    .unwrap_or_else(|e| panic!("get_blocks should resolve, got rejection: {e}"));
    let blocks = all["blocks"].as_array().expect("blocks array");
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0]["content"].as_str(), Some("Morning planning"));

    // No duplicates appear in list_pages.
    let listed = invoke(&wv, "list_pages", json!({}))
        .unwrap_or_else(|e| panic!("list_pages should resolve, got rejection: {e}"));
    let pages = listed["pages"].as_array().expect("pages array");
    assert_eq!(
        pages
            .iter()
            .filter(|p| p["path"].as_str() == Some("journals/2026-09-17.md"))
            .count(),
        1,
        "healthy DB entry must appear exactly once"
    );

    // DB unchanged after reads.
    let db_after = tv.store.list_pages().unwrap();
    assert_eq!(db_after.len(), 1);
    let blocks_after = tv
        .store
        .get_blocks_by_page("journals/2026-09-17.md")
        .unwrap();
    assert_eq!(blocks_after.len(), 1);
}
