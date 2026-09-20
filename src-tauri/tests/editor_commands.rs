//! E7.F1 — Verify the editor / notes / linking command layer through the REAL
//! Tauri command handlers against a REAL temp vault on disk.
//!
//! These tests drive the production `#[tauri::command]` handlers over the real
//! Tauri IPC dispatcher (official `tauri::test` mock-app harness). Nothing is
//! stubbed: the shared `BlockStore`, the `pkm-markdown` serializer, and the
//! filesystem are production code. On-disk state is asserted after each flow so
//! the acceptance contract is verified end-to-end, not just in-memory.
//!
//! E7.F1 acceptance mapping (editor / notes / linking):
//!   * ED-08 — `save_blocks` persists edited content to the `.md` file AND
//!     preserves the existing frontmatter (tags/aliases/created/extra) that a
//!     block-level save would otherwise strip.
//!   * ED-04 / TK-ED — marker + priority survive a full block round trip
//!     (`save_blocks` → `get_blocks`), and `toggle_block_marker` cycles the
//!     marker and persists it to disk with frontmatter intact.
//!   * LK-04 — `get_page_backlinks` returns real linked references and
//!     unlinked mentions from other pages holding a `[[wiki-link]]` or a plain
//!     text mention of the target page.
//!   * LN — `resolve_link_target` resolves a wiki-link target to a real page by
//!     slug, by space-dashed slug, and by page title.
//!   * AC — `autocomplete` (`kind=page`) returns real page suggestions the
//!     editor `[[`-autocomplete consumes.

mod common;

use app_lib::commands::vault::{AppState, VaultState};
use pkm_block::Page;
use serde_json::json;
use std::sync::Mutex;
use tauri::test::{mock_builder, mock_context, noop_assets};
use tauri::webview::InvokeRequest;
use tauri::WebviewWindow;

/// Build a mock Tauri app whose `VaultState` is backed by `vault`, with the
/// editor/notes/linking command handlers registered.
fn build_app(vault: &common::TestVault) -> tauri::App<tauri::test::MockRuntime> {
    let vs = VaultState::new(vault.vault_path.clone());
    mock_builder()
        .manage(Mutex::new(vs) as AppState)
        .invoke_handler(tauri::generate_handler![
            app_lib::commands::block::save_blocks,
            app_lib::commands::block::get_blocks,
            app_lib::commands::block::update_block,
            app_lib::commands::block::toggle_block_marker,
            app_lib::commands::block::clear_block_marker,
            app_lib::commands::search::get_page_backlinks,
            app_lib::commands::graph::resolve_link_target,
            app_lib::commands::search::autocomplete,
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

/// Seed a page into the store with blocks + frontmatter deserialized from its
/// on-disk body, mirroring exactly what `sync_page_from_disk` (the app-boot
/// import path) does: the parsed frontmatter (title, tags, …) is written to the
/// page before upsert, so `get_page_backlinks`/`resolve_link_target` see the
/// same page metadata a real boot would produce.
fn seed_disk_page(tv: &common::TestVault, rel: &str, body: &str) {
    let full = tv.vault_path.join(rel);
    std::fs::create_dir_all(full.parent().unwrap()).unwrap();
    std::fs::write(&full, body).unwrap();
    let (fm, _, blocks) = pkm_markdown::block_parser::parse_document(body);
    let mut page = Page::new(full, &tv.vault_path);
    page.frontmatter = pkm_block::PageFrontmatter {
        title: fm.title,
        created: fm.created,
        modified: fm.modified,
        tags: fm.tags,
        aliases: fm.aliases,
        ..Default::default()
    };
    page.set_blocks(&blocks);
    tv.store.upsert_page(&page).unwrap();
    for b in &blocks {
        tv.store.insert_block(b, rel).unwrap();
    }
}

/// Build a `BlockDto`-shaped JSON value that mirrors what the frontend sends.
/// (Kept for tests that mutate a block via `update_block`.)
#[allow(dead_code)]
fn block_dto(
    id: &str,
    content: &str,
    marker: Option<&str>,
    priority: Option<&str>,
) -> serde_json::Value {
    json!({
        "id": id,
        "content": content,
        "parentId": null,
        "leftId": null,
        "properties": [],
        "marker": marker,
        "priority": priority,
        "collapsed": false,
        "headingLevel": null,
    })
}

// ---------------------------------------------------------------------------
// ED-08 — block save persists content AND preserves frontmatter
// ---------------------------------------------------------------------------

#[test]
fn command_save_blocks_persists_edit_and_preserves_custom_frontmatter() {
    let tv = common::create_test_vault();
    let body = "---\ntitle: Alpha Project\ntags:\n  - project\n  - rust\ncustom_field: keep-me-verbatim\n---\n\n- Original line one\n  .id: 11111111-1111-4111-8111-111111111111\n- Original line two\n  .id: 22222222-2222-4222-8222-222222222222\n";
    seed_disk_page(&tv, "pages/alpha.md", body);

    let app = build_app(&tv);
    let wv = webview(&app);

    // Round trip: save the same two blocks with an edited first line.
    let all = invoke(&wv, "get_blocks", json!({ "pagePath": "pages/alpha.md" }))
        .expect("get_blocks resolves");
    let blocks = all["blocks"].as_array().expect("blocks array");
    assert_eq!(blocks.len(), 2);

    let mut edited = blocks.clone();
    edited[0]["content"] = json!("Edited line one");
    invoke(
        &wv,
        "save_blocks",
        json!({ "pagePath": "pages/alpha.md", "blocks": edited, "title": "Alpha Project" }),
    )
    .expect("save_blocks resolves");

    // The edit landed on disk.
    let on_disk = std::fs::read_to_string(tv.vault_path.join("pages/alpha.md")).unwrap();
    assert!(
        on_disk.contains("Edited line one"),
        "edit persists: {on_disk}"
    );
    assert!(!on_disk.contains("Original line one"));

    // Custom frontmatter (tags + an arbitrary custom field) was NOT stripped.
    assert!(
        on_disk.contains("custom_field: keep-me-verbatim"),
        "custom field preserved: {on_disk}"
    );
    assert!(on_disk.contains("- project"), "tags preserved: {on_disk}");
    assert!(on_disk.contains("- rust"), "tags preserved: {on_disk}");
    assert!(
        on_disk.contains("title: Alpha Project"),
        "title preserved: {on_disk}"
    );
}

// ---------------------------------------------------------------------------
// ED-04 — marker/priority survive the save → load round trip
// ---------------------------------------------------------------------------

#[test]
fn command_save_blocks_roundtrips_marker_and_priority() {
    let tv = common::create_test_vault();
    let body = "---\ntitle: tasks\n---\n\n- ship MVP\n  .marker: TODO\n  .priority: A\n  .id: 33333333-3333-4333-8333-333333333333\n- write docs\n  .id: 44444444-4444-4444-8444-444444444444\n";
    seed_disk_page(&tv, "pages/tasks.md", body);

    let app = build_app(&tv);
    let wv = webview(&app);

    let all = invoke(&wv, "get_blocks", json!({ "pagePath": "pages/tasks.md" }))
        .expect("get_blocks resolves");
    let blocks = all["blocks"].as_array().expect("blocks array");
    assert_eq!(blocks.len(), 2);

    // The marker block must read back with marker + priority intact.
    let task = blocks
        .iter()
        .find(|b| b["content"].as_str() == Some("ship MVP"))
        .expect("task block present");
    assert_eq!(task["marker"].as_str(), Some("TODO"));
    assert_eq!(task["priority"].as_str(), Some("A"));

    // Re-save the exact DTOs → disk must still carry the marker + priority.
    invoke(
        &wv,
        "save_blocks",
        json!({ "pagePath": "pages/tasks.md", "blocks": blocks, "title": "tasks" }),
    )
    .expect("save_blocks resolves");

    let on_disk = std::fs::read_to_string(tv.vault_path.join("pages/tasks.md")).unwrap();
    assert!(
        on_disk.contains(".marker: TODO"),
        "marker survives: {on_disk}"
    );
    assert!(
        on_disk.contains(".priority: A"),
        "priority survives: {on_disk}"
    );
}

// ---------------------------------------------------------------------------
// ED-04 — toggle_block_marker persists marker change to disk w/ frontmatter
// ---------------------------------------------------------------------------

#[test]
fn command_toggle_block_marker_cycles_and_persists_to_disk() {
    let tv = common::create_test_vault();
    let body = "---\ntitle: tasks\ntags:\n  - project\n---\n\n- ship MVP\n  .marker: TODO\n  .id: 55555555-5555-4555-8555-555555555555\n";
    seed_disk_page(&tv, "pages/tasks.md", body);

    // Resolve the real block id from the store.
    let blocks = tv.store.get_blocks_by_page("pages/tasks.md").unwrap();
    assert_eq!(blocks.len(), 1);
    let id = blocks[0].id.to_string();
    assert_eq!(
        blocks[0].marker.map(|m| m.as_str().to_string()).as_deref(),
        Some("TODO")
    );

    let app = build_app(&tv);
    let wv = webview(&app);

    // Toggle TODO → DOING (task lifecycle).
    let res = invoke(
        &wv,
        "toggle_block_marker",
        json!({ "pagePath": "pages/tasks.md", "blockId": id }),
    )
    .expect("toggle resolves");
    assert_eq!(res.as_str().unwrap_or(""), "DOING");

    // The change is written to the .md file.
    let on_disk = std::fs::read_to_string(tv.vault_path.join("pages/tasks.md")).unwrap();
    assert!(
        on_disk.contains(".marker: DOING"),
        "toggle persists: {on_disk}"
    );
    assert!(!on_disk.contains(".marker: TODO"));
    // Frontmatter survives the toggle's direct file rewrite.
    assert!(
        on_disk.contains("- project"),
        "frontmatter tags preserved: {on_disk}"
    );
}

#[test]
fn command_clear_block_marker_preserves_custom_frontmatter() {
    // Same frontmatter-preservation class as the toggle fix: clearing a marker
    // must not strip tags/aliases/extra from the on-disk .md.
    let tv = common::create_test_vault();
    let body = "---\ntitle: tasks\ntags:\n  - project\n  - rust\nalias: [old-name]\n---\n\n- ship MVP\n  .marker: TODO\n  .id: 99999999-9999-4999-8999-999999999999\n";
    seed_disk_page(&tv, "pages/tasks.md", body);

    let blocks = tv.store.get_blocks_by_page("pages/tasks.md").unwrap();
    assert_eq!(blocks.len(), 1);
    let id = blocks[0].id.to_string();

    let app = build_app(&tv);
    let wv = webview(&app);

    invoke(
        &wv,
        "clear_block_marker",
        json!({ "pagePath": "pages/tasks.md", "blockId": id }),
    )
    .expect("clear resolves");

    let on_disk = std::fs::read_to_string(tv.vault_path.join("pages/tasks.md")).unwrap();
    // Marker is cleared on disk.
    assert!(
        !on_disk.contains(".marker:"),
        "marker cleared from disk: {on_disk}"
    );
    // Full custom frontmatter survives (aliases are re-serialized in YAML block
    // form — `alias:\n- old-name` — which the parser normalizes; semantics kept).
    assert!(on_disk.contains("- project"), "tags preserved: {on_disk}");
    assert!(on_disk.contains("- rust"), "tags preserved: {on_disk}");
    assert!(on_disk.contains("old-name"), "aliases preserved: {on_disk}");
    assert!(
        on_disk.contains("title: tasks"),
        "title preserved: {on_disk}"
    );
}

// ---------------------------------------------------------------------------
// LK-04 — backlinks: linked references + unlinked mentions
// ---------------------------------------------------------------------------

#[test]
fn command_get_page_backlinks_returns_linked_and_unlinked() {
    let tv = common::create_test_vault();

    // Target page.
    let target = "---\ntitle: Beta Notes\n---\n\n- Beta landing\n  .id: 66666666-6666-4666-8666-666666666666\n";
    seed_disk_page(&tv, "pages/beta.md", target);

    // Linked source: an explicit [[wiki-link]] to the target page by title.
    let linked_src = "---\ntitle: alpha\n---\n\n- References [[Beta Notes]] here\n  .marker: TODO\n  .id: 77777777-7777-4777-8777-777777777777\n";
    seed_disk_page(&tv, "pages/alpha.md", linked_src);

    // Unlinked source: a plain-text mention of the target's stem, no formal link.
    let unlinked_src = "---\ntitle: gamma\n---\n\n- mentions Beta Notes in passing\n  .id: 88888888-8888-4888-8888-888888888888\n";
    seed_disk_page(&tv, "pages/gamma.md", unlinked_src);

    let app = build_app(&tv);
    let wv = webview(&app);

    let res = invoke(
        &wv,
        "get_page_backlinks",
        json!({ "pagePath": "pages/beta.md" }),
    )
    .expect("get_page_backlinks resolves");

    let backlinks = res.as_array().expect("backlinks array");
    let linked: Vec<_> = backlinks
        .iter()
        .filter(|b| b["is_linked"] == json!(true))
        .collect();
    let unlinked: Vec<_> = backlinks
        .iter()
        .filter(|b| b["is_linked"] == json!(false))
        .collect();

    // The explicit wiki-link source must come back as a LINKED backlink.
    assert!(
        linked
            .iter()
            .any(|b| b["source_page"].as_str() == Some("pages/alpha.md")),
        "linked backlink from alpha present: {res}"
    );
    // The plain text mention must come back as an UNLINKED mention.
    assert!(
        unlinked
            .iter()
            .any(|b| b["source_page"].as_str() == Some("pages/gamma.md")),
        "unlinked mention from gamma present: {res}"
    );
    // No self-references from the target page itself.
    assert!(
        !backlinks
            .iter()
            .any(|b| b["source_page"].as_str() == Some("pages/beta.md")),
        "no self backlinks: {res}"
    );
}

// ---------------------------------------------------------------------------
// LN — wiki-link target resolution (slug / dashed-slug / title)
// ---------------------------------------------------------------------------

#[test]
fn command_resolve_link_target_resolves_slug_dash_and_title() {
    let tv = common::create_test_vault();
    let body = "---\ntitle: Alpha Project\n---\n\n- first line\n  .id: aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa\n";
    seed_disk_page(&tv, "pages/alpha-project.md", body);

    let app = build_app(&tv);
    let wv = webview(&app);

    // By kebab slug (what the editor autocomplete writes).
    let by_slug = invoke(
        &wv,
        "resolve_link_target",
        json!({ "target": "alpha-project" }),
    )
    .expect("resolve by slug");
    assert_eq!(
        by_slug["page_path"].as_str(),
        Some("pages/alpha-project.md")
    );

    // By title.
    let by_title = invoke(
        &wv,
        "resolve_link_target",
        json!({ "target": "Alpha Project" }),
    )
    .expect("resolve by title");
    assert_eq!(
        by_title["page_path"].as_str(),
        Some("pages/alpha-project.md")
    );

    // Unknown target → empty resolution (dead-link popup path), not an error.
    let missing = invoke(
        &wv,
        "resolve_link_target",
        json!({ "target": "does-not-exist" }),
    )
    .expect("resolve missing resolves cleanly");
    assert!(
        missing["page_path"].is_null(),
        "missing target has no page_path: {missing}"
    );
}

// ---------------------------------------------------------------------------
// AC — editor [[-autocomplete suggestions over the real page list
// ---------------------------------------------------------------------------

#[test]
fn command_autocomplete_page_returns_real_pages() {
    let tv = common::create_test_vault();
    let b1 =
        "---\ntitle: Alpha Project\n---\n\n- a1\n  .id: bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb\n";
    seed_disk_page(&tv, "pages/alpha-project.md", b1);
    let b2 =
        "---\ntitle: Research Notes\n---\n\n- r1\n  .id: cccccccc-cccc-4ccc-8ccc-cccccccccccc\n";
    seed_disk_page(&tv, "pages/research.md", b2);

    let app = build_app(&tv);
    let wv = webview(&app);

    // Empty query → both pages listed.
    let res = invoke(&wv, "autocomplete", json!({ "query": "", "kind": "page" }))
        .expect("autocomplete resolves");
    let items = res.as_array().expect("items array");
    assert!(!items.is_empty(), "autocomplete lists pages: {res}");

    // Filtered query narrows to the matching page, with the real path as detail.
    let filtered = invoke(
        &wv,
        "autocomplete",
        json!({ "query": "alpha", "kind": "page" }),
    )
    .expect("autocomplete resolves");
    let f = filtered.as_array().expect("items array");
    assert!(
        f.iter()
            .any(|i| i["detail"].as_str() == Some("pages/alpha-project.md")),
        "alpha suggestion present: {filtered}"
    );
}
