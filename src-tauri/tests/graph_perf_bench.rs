//! Performance profiling harness for the graph view pipeline.
//!
//! This is NOT an acceptance test — it is a reproducible micro-benchmark for
//! profiling the exact backend code path behind `get_graph_panel_data`:
//!
//!   1. `BlockStore::open`                (fresh SQLite connection per command)
//!   2. `PageMetaIndex::from_store`        (list_pages + get_pages)
//!   3. `build_adjacency_list`            (ONE read: `get_page_ref_edges`)
//!   4. connected-components BFS           (get_connected_components_from_meta)
//!   5. DTO build + `serde_json` serialization of the full `GraphPanelDataDto`
//!
//! Because the store-query internals live in `pkm-block` and the graph
//! derivation lives in `src-tauri/src/commands/graph.rs` (private functions),
//! this harness drives the REAL Tauri IPC command (`get_graph_panel_data`)
//! through the `tauri::test` mock-app harness — the same approach the existing
//! `graph_commands.rs` acceptance suite uses — plus direct `BlockStore` reads
//! for per-phase attribution and `EXPLAIN QUERY PLAN` for SQLite introspection.
//!
//! Run (release, for honest numbers):
//!   cargo test -p stratum-tauri --release --test graph_perf_bench -- --nocapture
//!
//! The benchmark cases are `#[ignore]` gated (ignored by default in `cargo test`,
//! and the project's CI). The seed-generation helpers are shared with gate:
//!   cargo test -p stratum-tauri --test graph_perf_bench
//! (#[ignore] tests are still compiled, so the seed code stays covered.)

mod common;

use app_lib::commands::graph;
use app_lib::commands::vault::{AppState, VaultState};
use pkm_block::{Block, BlockStore, Page};
use std::sync::Mutex;
use tauri::test::{mock_builder, mock_context, noop_assets};
use tauri::webview::InvokeRequest;
use tauri::WebviewWindow;

// ── Synthetic vault seeding ─────────────────────────────────────────────
// The seeds mirror the shapes the parser + `reconcile_page_links` produce:
//   * a page row per note (upsert_page),
//   * one body block per note holding `- starts with [[target]]`,
//   * one `links` row per wiki-link (the shape `reconcile_page_links` writes).

/// Seed `n` pages in a ring: page i links to page (i+1)%n, so every node is
/// connected (a single giant component — the worst case for BFS/serialization
/// sizes) plus one deliberate isolated page as an orphan.
fn seed_ring_vault(tv: &common::TestVault, n: usize) -> (usize, usize) {
    let mut block_ids = Vec::with_capacity(n);
    let mut edges = 0usize;
    for i in 0..n {
        let path = format!("pages/page-{i}.md");
        let full = tv.vault_path.join(&path);
        std::fs::write(&full, format!("title: Page {i}\n")).unwrap();
        let mut page = Page::new(full, &tv.vault_path);
        page.frontmatter.title = Some(format!("Page {i}"));
        tv.store.upsert_page(&page).unwrap();

        let next = (i + 1) % n;
        let content = format!("- starts with [[page-{next}]]\n");
        let block = Block::new(uuid::Uuid::new_v4(), content);
        tv.store.insert_block(&block, &path).unwrap();
        block_ids.push(block.id);
        edges += 1;
    }
    for (i, bid) in block_ids.iter().enumerate() {
        let next = (i + 1) % n;
        tv.store
            .insert_link(*bid, "page_ref", Some(&format!("pages/page-{next}.md")), None)
            .unwrap();
    }
    // One isolated page (orphan).
    let iso = "pages/isolated.md";
    let full = tv.vault_path.join(iso);
    std::fs::write(&full, "title: Isolated\n").unwrap();
    let mut page = Page::new(full, &tv.vault_path);
    page.frontmatter.title = Some("Isolated".to_string());
    tv.store.upsert_page(&page).unwrap();
    (n + 1, edges)
}

/// Seed `n` pages with NO links at all — every node is an orphan. Exercises
/// the same store reads but the minimal adjacency/BFS/serialization work, to
/// isolate where time actually goes.
fn seed_orphan_vault(tv: &common::TestVault, n: usize) {
    for i in 0..n {
        let path = format!("pages/disconnected-{i}.md");
        let full = tv.vault_path.join(&path);
        std::fs::write(&full, format!("title: Page {i}\n")).unwrap();
        let mut page = Page::new(full, &tv.vault_path);
        page.frontmatter.title = Some(format!("Page {i}"));
        tv.store.upsert_page(&page).unwrap();
        let block = Block::new(uuid::Uuid::new_v4(), "- no links here".to_string());
        tv.store.insert_block(&block, &path).unwrap();
    }
}

/// Mid-sized vault with many connected components (a chain) plus orphan pages,
/// to look at the BFS component-count scalability separate from a single giant
/// component. Chain: page i links page i+1 (chain_links components of size 1
/// + one component of size chain_len).
fn seed_chain_vault(tv: &common::TestVault, n: usize) -> usize {
    let mut block_ids = Vec::with_capacity(n);
    for i in 0..n {
        let path = format!("pages/chain-{i}.md");
        let full = tv.vault_path.join(&path);
        std::fs::write(&full, format!("title: Chain {i}\n")).unwrap();
        let mut page = Page::new(full, &tv.vault_path);
        page.frontmatter.title = Some(format!("Chain {i}"));
        tv.store.upsert_page(&page).unwrap();

        let content = if i + 1 < n {
            format!("- next [[chain-{}]]\n", i + 1)
        } else {
            "- tail".to_string()
        };
        let block = Block::new(uuid::Uuid::new_v4(), content);
        tv.store.insert_block(&block, &path).unwrap();
        block_ids.push(block.id);
        if i + 1 < n {
            tv.store
                .insert_link(
                    block_ids[i],
                    "page_ref",
                    Some(&format!("pages/chain-{}.md", i + 1)),
                    None,
                )
                .unwrap();
        }
    }
    n
}

// ── IPC harness (mirrors graph_commands.rs) ─────────────────────────────

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

// ── Measurement helpers ─────────────────────────────────────────────────

/// Current total resident set size of the process in MiB (best-effort).
fn rss_mib() -> f64 {
    let path = format!("/proc/{}/status", std::process::id());
    let Ok(status) = std::fs::read_to_string(path) else {
        return 0.0;
    };
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            if let Some(kb) = rest.trim().strip_suffix(" kB") {
                if let Ok(kb) = kb.trim().parse::<f64>() {
                    return kb / 1024.0;
                }
            }
        }
    }
    0.0
}

// ── EXPLAIN QUERY PLAN (SQLite introspection) ────────────────────────────

fn explain_query_plan(store: &BlockStore, label: &str, sql: &str, params: &[String]) {
    println!("\n  -- {label} EXPLAIN QUERY PLAN --");
    let conn_owned = rusqlite::Connection::open_in_memory().unwrap();
    // Copy the vault schema so the planner sees the same indexes.
    let _ = conn_owned.execute_batch(
        "CREATE TABLE IF NOT EXISTS blocks (
            id TEXT PRIMARY KEY, page_path TEXT NOT NULL, content TEXT NOT NULL DEFAULT '',
            parent_id TEXT, left_id TEXT, properties TEXT NOT NULL DEFAULT '{}',
            marker TEXT, priority TEXT, collapsed INTEGER NOT NULL DEFAULT 0,
            heading_level INTEGER, created_at TEXT NOT NULL, modified_at TEXT NOT NULL);
         CREATE INDEX IF NOT EXISTS idx_blocks_page ON blocks(page_path);
         CREATE INDEX IF NOT EXISTS idx_blocks_parent ON blocks(parent_id);
         CREATE INDEX IF NOT EXISTS idx_blocks_marker ON blocks(marker);
         CREATE TABLE IF NOT EXISTS pages (
            path TEXT PRIMARY KEY, title TEXT, frontmatter TEXT NOT NULL DEFAULT '{}',
            block_count INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL, modified_at TEXT NOT NULL);
         CREATE TABLE IF NOT EXISTS links (
            id INTEGER PRIMARY KEY AUTOINCREMENT, source_block TEXT NOT NULL,
            link_type TEXT NOT NULL, target_page TEXT COLLATE NOCASE, target_block TEXT,
            FOREIGN KEY (source_block) REFERENCES blocks(id) ON DELETE CASCADE);
         CREATE INDEX IF NOT EXISTS idx_links_source ON links(source_block);
         CREATE INDEX IF NOT EXISTS idx_links_target_page ON links(target_page);
         CREATE INDEX IF NOT EXISTS idx_links_target_block ON links(target_block);",
    );
    {
        let mut stmt = conn_owned
            .prepare(&format!("EXPLAIN QUERY PLAN {}", sql))
            .unwrap();
        let mut rows = stmt.query(rusqlite::params_from_iter(params.iter())).unwrap();
        while let Ok(Some(row)) = rows.next() {
            let detail: String = row.get(3).unwrap();
            println!("      {label}: {detail}");
        }
    }
    let _ = store;
}

// ── Phase-timed, in-process measurement of the real code path ────────────

/// Replicate the real command-path timings (same store methods the command
/// uses) in isolation. Phase breakdown attribution is the deliverable.
fn store_phase_breakdown(store: &BlockStore, n: usize) {
    use std::time::Instant;

    let t0 = Instant::now();
    let paths = store.list_pages().unwrap();
    let t_list = t0.elapsed();

    let t1 = Instant::now();
    let _pages = store.get_pages(&paths).unwrap();
    let t_pages = t1.elapsed();

    // The optimized graph path does NOT re-read/re-parse block content; edges
    // come in a single read from the `links` table (the authoritative source),
    // so this is the only per-command block-level store read left.
    let t2 = Instant::now();
    let edges = store.get_page_ref_edges().unwrap();
    let t_edges = t2.elapsed();

    println!(
        "  store-read phases ({n} pages): list_pages={:.1?}  get_pages={:.1?}  get_page_ref_edges={:.1?}  (link rows: {})",
        t_list,
        t_pages,
        t_edges,
        edges.len()
    );

    // SQLite query plans for the exact queries the command layer runs.
    let placeholders: Vec<String> = vec!["?".to_string(); paths.len()];
    let slug_plan = "SELECT path FROM pages ORDER BY modified_at DESC".to_string();
    let pages_plan = format!(
        "SELECT path, frontmatter FROM pages WHERE path IN ({})",
        placeholders.join(", ")
    );
    let edges_plan = "SELECT src_page.path, dst.path \
         FROM links l \
         JOIN blocks src ON l.source_block = src.id \
         JOIN pages src_page ON src.page_path = src_page.path \
         JOIN pages dst ON l.target_page = dst.path \
         WHERE l.link_type = 'page_ref' \
         ORDER BY src.rowid, l.rowid"
        .to_string();
    explain_query_plan(store, "list_pages", &slug_plan, &[]);
    explain_query_plan(store, "get_pages", &pages_plan, &paths);
    explain_query_plan(store, "get_page_ref_edges", &edges_plan, &[]);
}

// ── Benchmark cases (all #[ignore] gated) ───────────────────────────────

#[test]
#[ignore]
fn bench_small_vault_100_pages() {
    let tv = common::create_test_vault();
    seed_ring_vault(&tv, 100);
    println!("\n=== get_graph_panel_data: 100-page ring ===");
    let app = build_app(&tv);
    let wv = webview(&app);
    graph::invalidate_graph_cache();

    let start = std::time::Instant::now();
    let (_peak, resp) = {
        let mut peak = rss_mib();
        let out = invoke(&wv, "get_graph_panel_data", serde_json::json!({})).unwrap();
        peak = peak.max(rss_mib());
        (peak, out)
    };
    let elapsed = start.elapsed();
    println!("  total IPC round-trip: {elapsed:?}");
    println!("  nodes={} edges={} components={} orphans={}",
        resp["graph"]["node_count"], resp["graph"]["edge_count"],
        resp["components"].as_array().map(|a| a.len()).unwrap_or(0),
        resp["orphans"].as_array().map(|a| a.len()).unwrap_or(0));

    let ser = serde_json::to_string(&resp).unwrap();
    println!("  serialized payload size: {} bytes", ser.len());
}

#[test]
#[ignore]
fn bench_scale_sweep_100_1000_10000() {
    // Full command-path timings at 100 / 1k / 10k notes (ring + one orphan).
    for n in [100usize, 1000, 10_000] {
        let tv = common::create_test_vault();
        let (total_pages, edges) = seed_ring_vault(&tv, n);
        let app = build_app(&tv);
        let wv = webview(&app);
        graph::invalidate_graph_cache();

        // COLD: measure Peak RSS across the whole in-process command.
        let start = std::time::Instant::now();
        let mut peak = rss_mib();
        let resp = invoke(&wv, "get_graph_panel_data", serde_json::json!({})).unwrap();
        peak = peak.max(rss_mib());
        let elapsed = start.elapsed();

        let nodes = resp["graph"]["node_count"].as_u64().unwrap();
        let edges_out = resp["graph"]["edge_count"].as_u64().unwrap();
        let ser = serde_json::to_string(&resp).unwrap();
        println!(
            "RING n={n:>6} (pages={total_pages} edges={edges}): COLD total {elapsed:?} | \
             peak-rss~{peak:.0}MiB | payload {}B | nodes={nodes} rtn-edges={edges_out}",
            ser.len()
        );

        // WARM: second call is served from the in-process cache (no DB).
        let start = std::time::Instant::now();
        let _ = invoke(&wv, "get_graph_panel_data", serde_json::json!({})).unwrap();
        let warm = start.elapsed();
        println!("    warm (cached) second call: {warm:?}");

        // Store-phase attribution on the same seeded DB.
        store_phase_breakdown(&tv.store, total_pages);
    }
}

#[test]
#[ignore]
fn bench_all_orphans_10000() {
    // No links anywhere — isolates store-read + node-build cost from BFS.
    let tv = common::create_test_vault();
    seed_orphan_vault(&tv, 10_000);
    let app = build_app(&tv);
    let wv = webview(&app);
    graph::invalidate_graph_cache();

    let start = std::time::Instant::now();
    let mut peak = rss_mib();
    let resp = invoke(&wv, "get_graph_panel_data", serde_json::json!({})).unwrap();
    peak = peak.max(rss_mib());
    let elapsed = start.elapsed();

    let orphans = resp["orphans"].as_array().map(|a| a.len()).unwrap_or(0);
    println!("\n=== 10k all-orphan ring-0: total {elapsed:?} | peak-rss~{peak:.0}MiB | orphans={orphans} ===");
    store_phase_breakdown(&tv.store, 10_000);
}

#[test]
#[ignore]
fn bench_chain_10000_components() {
    // 1 long chain → many BFS iterations but each component has size ~1..N/2.
    // Focuses the BFS component enumeration cost.
    let tv = common::create_test_vault();
    seed_chain_vault(&tv, 10_000);
    let app = build_app(&tv);
    let wv = webview(&app);
    graph::invalidate_graph_cache();

    let start = std::time::Instant::now();
    let mut peak = rss_mib();
    let resp = invoke(&wv, "get_graph_panel_data", serde_json::json!({})).unwrap();
    peak = peak.max(rss_mib());
    let elapsed = start.elapsed();

    let comps = resp["components"].as_array().map(|a| a.len()).unwrap_or(0);
    println!("\n=== 10k chain: total {elapsed:?} | peak-rss~{peak:.0}MiB | components={comps} ===");
    store_phase_breakdown(&tv.store, 10_000);
}

/// Confirm the benchmark seed machinery is correct (non-ignored), so `cargo
/// test` (CI) compiles and lightly exercises the helper code without running
/// the slow cases.
#[test]
fn seed_machinery_roundtrips() {
    let tv = common::create_test_vault();
    let (pages, edges) = seed_ring_vault(&tv, 10);
    assert_eq!(pages, 11, "10 ring pages + 1 orphan");
    assert_eq!(edges, 10, "ring has 10 outgoing links");
    let all = tv.store.list_pages().unwrap();
    assert_eq!(all.len(), 11);

    let paths = tv.store.list_pages().unwrap();
    let pages_fm = tv.store.get_pages(&paths).unwrap();
    assert!(pages_fm.contains_key("pages/page-0.md"));

    let blocks = tv.store
        .get_blocks_by_pages(&["pages/page-0.md".to_string()])
        .unwrap();
    assert_eq!(blocks["pages/page-0.md"].len(), 1);
    assert!(blocks["pages/page-0.md"][0].content.contains("[[page-1]]"));
}
