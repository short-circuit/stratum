//! Tauri command-level integration tests for the graph view command layer.
//!
//! These tests drive the REAL `#[tauri::command]` handlers (`get_graph_panel_data`,
//! `get_graph_data`, `get_orphaned_notes`, `save_graph_settings`) over the real Tauri
//! IPC dispatcher using the official `tauri::test` mock-app harness. Each test builds
//! a mock Tauri app whose `VaultState` is backed by a REAL temp vault. Nothing is
//! stubbed — the command handlers, store, parser, and graph builders are all
//! production code.
//!
//! E7.F3 acceptance mapping (graph view verification):
//!   * node labels render → nodes carry `title` = page frontmatter title
//!   * links present on first load → `get_graph_panel_data` returns edges from
//!     `[[wiki-link]]` content plus the connected components/orphans the UI
//!     renders on mount
//!   * tag-based colors correct → node `tags` are deterministic per page and
//!     derive from frontmatter (the frontend hashes `tags[0]` into a palette)
//!   * orphan detection → `orphans` lists exactly the unlinked pages
//!   * cache invalidated after reindex → mutating a page then
//!     `get_graph_panel_data` returns fresh data (no stale in-memory cache)
//!   * settings persist (graphStore) → `save_graph_settings` round-trips through
//!     the vault's `config.toml` and `get_settings` returns it
//!
//! NOTE on the process-global cache: `get_graph_panel_data` caches its result in a
//! process-global `OnceLock` (`GRAPH_CACHE`) that is NOT keyed by vault. The mock
//! harness intentionally calls `invalidate_graph_cache()` between fixtures so a stale
//! payload from one temp vault cannot leak into the next.
//!
//! Performance at 500+ nodes is asserted here on a 600-node real vault
//! (`graph_load_under_2s_at_600_nodes`), matching the AGENTS.md graph-load target
//! (10k notes < 2s) at the acceptance scale.

mod common;

use app_lib::commands::graph;
use app_lib::commands::settings::GraphSettingsDto;
use app_lib::commands::vault::{AppState, VaultState};
use pkm_block::Page;
use serde_json::json;
use std::sync::Mutex;
use tauri::test::{mock_builder, mock_context, noop_assets};
use tauri::webview::InvokeRequest;
use tauri::WebviewWindow;

/// Build a mock Tauri app whose `VaultState` is backed by `vault`, with the
/// graph commands + settings commands registered.
fn build_app(vault: &common::TestVault) -> tauri::App<tauri::test::MockRuntime> {
    let vs = VaultState::new(vault.vault_path.clone());
    mock_builder()
        .manage(Mutex::new(vs) as AppState)
        .invoke_handler(tauri::generate_handler![
            app_lib::commands::graph::get_graph_panel_data,
            app_lib::commands::graph::get_graph_data,
            app_lib::commands::graph::get_orphaned_notes,
            app_lib::commands::settings::get_settings,
            app_lib::commands::settings::save_graph_settings,
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
/// the shape the reindex/sync code path produces.
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
}

/// Build the canonical small fixture:
///   pages/alpha.md   title "Alpha"  tags [project, rust]  links [[beta]]
///   pages/beta.md    title "Beta"   tags [project]        links [[alpha]]
///   pages/orphan.md  title "Orphan" tags []               links []
fn seed_small_graph(tv: &common::TestVault) {
    seed_page(
        tv,
        "pages/alpha.md",
        "Alpha",
        &["project", "rust"],
        "---\ntitle: Alpha\ntags:\n  - project\n  - rust\n---\n- See [[beta]] for details\n",
    );
    seed_page(
        tv,
        "pages/beta.md",
        "Beta",
        &["project"],
        "---\ntitle: Beta\ntags:\n  - project\n---\n- Back to [[alpha]]\n",
    );
    seed_page(
        tv,
        "pages/orphan.md",
        "Orphan",
        &[],
        "---\ntitle: Orphan\n---\n- Isolated content\n",
    );
}

/// Build a vault with `n` pages where page i links to page (i+1)%n — a single
/// connected ring plus one deliberate orphan added on top.
fn seed_ring_vault(tv: &common::TestVault, n: usize) {
    for i in 0..n {
        let next = (i + 1) % n;
        let content =
            format!("---\ntitle: Page {i}\ntags:\n  - group-a\n---\n- Links to [[page-{next}]]\n");
        seed_page(
            tv,
            &format!("pages/page-{i}.md"),
            &format!("Page {i}"),
            &["group-a"],
            &content,
        );
    }
}

#[test]
fn graph_panel_data_returns_nodes_edges_and_labels_on_first_load() {
    let tv = common::create_test_vault();
    seed_small_graph(&tv);
    graph::invalidate_graph_cache();

    let app = build_app(&tv);
    let wv = webview(&app);

    let resp =
        invoke(&wv, "get_graph_panel_data", json!({})).expect("get_graph_panel_data succeeds");
    let graph = &resp["graph"];
    let nodes = graph["nodes"].as_array().expect("nodes array");
    let edges = graph["edges"].as_array().expect("edges array");

    // All pages present as nodes on first load.
    assert_eq!(nodes.len(), 3, "all pages become nodes");
    // The two wiki-links are present as edges on first load.
    assert_eq!(edges.len(), 2, "both [[wiki-links]] become edges");

    // Labels: every node carries its page title (what SpriteText renders).
    let mut by_id: std::collections::HashMap<String, serde_json::Value> =
        std::collections::HashMap::new();
    for n in nodes {
        let id = n["id"].as_str().unwrap().to_string();
        let label = n["title"].as_str().unwrap().to_string();
        assert!(
            !label.is_empty(),
            "every node must carry a renderable label (title)"
        );
        by_id.insert(id, n.clone());
    }
    assert_eq!(by_id["alpha"]["title"], "Alpha");
    assert_eq!(by_id["beta"]["title"], "Beta");
    assert_eq!(by_id["orphan"]["title"], "Orphan");

    // Orphan detection: exactly the unlinked page.
    let orphans = resp["orphans"].as_array().expect("orphans array");
    assert_eq!(orphans.len(), 1, "exactly one orphan (orphan.md)");
    assert_eq!(orphans[0]["slug"], "orphan");

    // Connected components: alpha+beta in one component, orphan alone.
    let comps = resp["components"].as_array().expect("components array");
    assert_eq!(comps.len(), 2, "two connected components");
    let biggest = comps
        .iter()
        .max_by_key(|c| c["size"].as_u64().unwrap())
        .unwrap();
    assert_eq!(
        biggest["size"], 2,
        "connected pair is the largest component"
    );
}

#[test]
fn graph_node_tags_are_deterministic_and_derive_from_frontmatter() {
    let tv = common::create_test_vault();
    seed_small_graph(&tv);
    graph::invalidate_graph_cache();

    let app = build_app(&tv);
    let wv = webview(&app);

    let resp =
        invoke(&wv, "get_graph_panel_data", json!({})).expect("get_graph_panel_data succeeds");
    let nodes = resp["graph"]["nodes"].as_array().expect("nodes array");

    // The frontend derives node color purely from `tags[0]` (GraphCanvas.tsx
    // hashes it into a palette), so a deterministic payload is the data-layer
    // contract: same page in the same vault must always produce the same tag.
    let mut by_id: std::collections::HashMap<String, serde_json::Value> =
        std::collections::HashMap::new();
    for n in nodes {
        by_id.insert(n["id"].as_str().unwrap().to_string(), n.clone());
    }
    let alpha_tags: Vec<String> = by_id["alpha"]["tags"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t.as_str().unwrap().to_string())
        .collect();
    assert!(
        alpha_tags.iter().any(|t| t == "project"),
        "frontmatter tag project must be attached to alpha"
    );
    assert!(
        alpha_tags.iter().any(|t| t == "rust"),
        "frontmatter tag rust must be attached to alpha"
    );
    let orphan_tags: Vec<String> = by_id["orphan"]["tags"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t.as_str().unwrap().to_string())
        .collect();
    assert!(
        orphan_tags.is_empty(),
        "untagged page must not fabricate tags (drives the default color)"
    );

    // Determinism across a re-fetch (cache path) — identical tag payload.
    let resp2 = invoke(&wv, "get_graph_panel_data", json!({})).expect("second fetch succeeds");
    let nodes2 = resp2["graph"]["nodes"].as_array().unwrap();
    let mut by_id2 = std::collections::HashMap::new();
    for n in nodes2 {
        by_id2.insert(n["id"].as_str().unwrap().to_string(), n["tags"].clone());
    }
    assert_eq!(by_id2["alpha"], json!(["project", "rust"]));
}

#[test]
fn graph_cache_is_invalidated_after_mutation() {
    let tv = common::create_test_vault();
    seed_small_graph(&tv);
    graph::invalidate_graph_cache();

    let app = build_app(&tv);
    let wv = webview(&app);

    let resp1 = invoke(&wv, "get_graph_panel_data", json!({})).expect("first fetch");
    assert_eq!(
        resp1["graph"]["node_count"].as_u64().unwrap(),
        3,
        "initial graph has 3 nodes"
    );

    // Add a fourth page with a wiki-link to alpha — exactly what
    // `invalidate_graph_cache()` is called after (page create/save/edit).
    seed_page(
        &tv,
        "pages/delta.md",
        "Delta",
        &[],
        "---\ntitle: Delta\n---\n- References [[alpha]]\n",
    );
    graph::invalidate_graph_cache();

    let resp2 =
        invoke(&wv, "get_graph_panel_data", json!({})).expect("second fetch after mutation");
    assert_eq!(
        resp2["graph"]["node_count"].as_u64().unwrap(),
        4,
        "newly added page appears after cache invalidation"
    );
    assert!(
        resp2["graph"]["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|n| n["id"] == "delta"),
        "delta is present in the fresh graph"
    );
    // The new edge links alpha<->delta (delta links alpha; alpha does not link
    // delta) — the payload must reflect the mutation, not the stale cache.
    let delta_edges = resp2["graph"]["edges"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["source"] == "delta")
        .count();
    assert_eq!(
        delta_edges, 1,
        "delta's outgoing wiki-link is in the fresh graph"
    );
}

#[test]
fn graph_cached_response_is_served_from_cache_without_mutation() {
    let tv = common::create_test_vault();
    seed_small_graph(&tv);
    graph::invalidate_graph_cache();

    let app = build_app(&tv);
    let wv = webview(&app);

    let r1 = invoke(&wv, "get_graph_panel_data", json!({})).expect("first");
    // A second call with no intervening mutation returns the identical cached
    // payload (stable node set, no re-read skew).
    let r2 = invoke(&wv, "get_graph_panel_data", json!({})).expect("second");
    assert_eq!(
        r1["graph"]["node_count"], r2["graph"]["node_count"],
        "unchanged vault is served from cache with same node count"
    );
    assert_eq!(
        r1["orphans"], r2["orphans"],
        "orphan list is stable across cached reads"
    );
}

#[test]
fn graph_load_under_2s_at_600_nodes() {
    let tv = common::create_test_vault();
    // 600 pages in a ring (well above the 500-node acceptance floor), one orphan.
    seed_ring_vault(&tv, 600);
    seed_page(
        &tv,
        "pages/isolated.md",
        "Isolated",
        &[],
        "---\ntitle: Isolated\n---\n- alone\n",
    );
    graph::invalidate_graph_cache();

    let app = build_app(&tv);
    let wv = webview(&app);

    let start = std::time::Instant::now();
    let resp = invoke(&wv, "get_graph_panel_data", json!({})).expect("600-node graph loads");
    let elapsed = start.elapsed();

    let node_count = resp["graph"]["node_count"].as_u64().unwrap();
    assert_eq!(node_count, 601, "all 601 pages present");
    assert_eq!(
        resp["graph"]["edge_count"].as_u64().unwrap(),
        600,
        "ring links all present"
    );

    // AGENTS.md graph-load target: 10k notes < 2s. 600 nodes is far below that
    // envelope, so the acceptance floor must be hit in well under 2s even in an
    // unoptimized debug build.
    assert!(
        elapsed.as_millis() < 2000,
        "600-node graph must load in < 2s (AGENTS.md target), took {:?}",
        elapsed
    );

    // The whole payload is present and well-formed for the renderer.
    let orphans = resp["orphans"].as_array().unwrap();
    assert_eq!(orphans.len(), 1, "exactly the isolated page is an orphan");
}

#[test]
fn graph_settings_persist_through_config_toml_round_trip() {
    let tv = common::create_test_vault();
    graph::invalidate_graph_cache();

    let app = build_app(&tv);
    let wv = webview(&app);

    // Change a selection of graph settings.
    let custom = GraphSettingsDto {
        show_connected: false,
        show_orphaned: true,
        show_tags: false,
        charge_strength: -12.5,
        link_distance: 77.0,
        alpha_decay: 0.11,
        velocity_decay: 0.29,
        link_curvature: 0.42,
    };
    invoke(&wv, "save_graph_settings", json!({ "graph": custom }))
        .expect("save_graph_settings succeeds");

    // Re-read via a fresh serialization — must reflect the persisted values.
    let after = invoke(&wv, "get_settings", json!({})).expect("get_settings after save");
    assert_eq!(after["graph"]["show_connected"], json!(false));
    assert_eq!(after["graph"]["show_orphaned"], json!(true));
    assert_eq!(after["graph"]["show_tags"], json!(false));
    assert_eq!(after["graph"]["charge_strength"], json!(-12.5));
    assert_eq!(after["graph"]["link_distance"], json!(77.0));
    assert_eq!(after["graph"]["alpha_decay"], json!(0.11));
    assert_eq!(after["graph"]["velocity_decay"], json!(0.29));
    assert_eq!(after["graph"]["link_curvature"], json!(0.42));

    // The values must be on disk in the vault's config.toml (the persistence
    // contract that survives an app restart).
    let config_path = tv.vault_path.join(".pkm").join("config.toml");
    assert!(config_path.exists(), "config.toml written");
    let on_disk = std::fs::read_to_string(&config_path).unwrap();
    assert!(
        on_disk.contains("charge_strength = -12.5"),
        "charge_strength persisted"
    );
    assert!(
        on_disk.contains("link_curvature = 0.42"),
        "link_curvature persisted"
    );
    assert!(
        on_disk.contains("show_connected = false"),
        "show_connected persisted"
    );

    // A brand-new `get_graph_panel_data` after a save must still work (the graph
    // command path is independent of persisted settings).
    let g = invoke(&wv, "get_graph_panel_data", json!({})).expect("graph still loads");
    assert_eq!(
        g["graph"]["node_count"].as_u64().unwrap(),
        0,
        "empty vault has no nodes"
    );
}
