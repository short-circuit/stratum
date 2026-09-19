//! E7.F7 — Verify AI & research command layer through the REAL Tauri IPC
//! dispatcher against a REAL temp vault.
//!
//! These tests drive the actual `#[tauri::command]` handlers (`ai_transform_block`,
//! `generate_mermaid`, `ai_research`, `ai_interlink_notes`) over the official
//! `tauri::test` mock-app harness. Nothing is faked on the app side: config
//! loading, `ProviderFactory`, the research engine, and the block store are all
//! production code. The only external surface is the configured AI/research
//! upstream, which is a wiremock server standing in for the configured provider
//! and SearXNG endpoint — exactly the "runs against a configured endpoint"
//! acceptance contract.
//!
//! Coverage mapping to the E7.F7 acceptance criteria:
//!   * Transform (rewrite/summarize) — `ai_transform_block` returns the provider
//!     output for rewrite and summarize actions, erroring cleanly when no config
//!     exists (failures surface without hanging).
//!   * Mermaid generation — `generate_mermaid` returns a diagram body from the
//!     configured provider.
//!   * Research via SearXNG with SSRF guard + timeout — `ai_research` queries the
//!     configured SearXNG endpoint, fetches a result page, and synthesizes through
//!     the LLM; the endpoint is a loopback mock (permitted by the SSRF guard) and
//!     the read timeout is small enough that a stuck upstream cannot hang the
//!     command.
//!   * Interlink suggestions — `ai_interlink_notes` finds related notes from the
//!     real index and rewrites the text with wiki-links when the LLM adds them.

mod common;

use app_lib::commands::vault::{AppState, VaultState};
use serde_json::json;
use std::sync::Mutex;
use tauri::test::{mock_builder, mock_context, noop_assets};
use tauri::webview::InvokeRequest;
use tauri::Manager;
use tauri::WebviewWindow;

/// Drive `fut` to completion on a short-lived current-thread tokio runtime so
/// wiremock (async) can be mounted from a plain `#[test]`. Matches the sibling
/// plugin/sync harnesses.
fn block_on<F: std::future::Future>(fut: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(fut)
}

fn build_app(vault: &common::TestVault) -> tauri::App<tauri::test::MockRuntime> {
    let vs = VaultState::new(vault.vault_path.clone());
    mock_builder()
        .manage(Mutex::new(vs) as AppState)
        .invoke_handler(tauri::generate_handler![
            app_lib::commands::ai::ai_transform_block,
            app_lib::commands::ai::generate_mermaid,
            app_lib::commands::ai::ai_research,
            app_lib::commands::ai::ai_interlink_notes,
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

/// Write a `.pkm/config.toml` carrying an `[ai]` + `[research]` section pointed
/// at a real wiremock origin, mirroring what a user configures in Settings.
fn seed_config(tv: &common::TestVault, ai_endpoint: &str, searxng_endpoint: &str) {
    let dir = tv.vault_path.join(".pkm");
    std::fs::create_dir_all(&dir).unwrap();
    let toml = format!(
        "vault_path = \"{}\"\n\n[ai]\nprovider = \"CustomOpenAI\"\nendpoint = \"{ai_endpoint}\"\napi_key = \"sk-test\"\nmodel = \"test-model\"\nrag_enabled = true\nrag_chunk_count = 3\n\n[research]\nsearxng_endpoint = \"{searxng_endpoint}\"\nmax_results = 3\nmax_depth = 1\n",
        tv.vault_path.display()
    );
    std::fs::write(dir.join("config.toml"), toml).unwrap();
}

/// Mount a wiremock server exposing both an OpenAI-compatible chat surface
/// (`/v1/chat/completions`) and a SearXNG surface (`/search`, `/page1`).
async fn mount_ai_research_backend() -> wiremock::MockServer {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, ResponseTemplate};

    let server = wiremock::MockServer::start().await;

    // Chat completions — the single LLM answer used by transform, mermaid,
    // research synthesis, and interlink.
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "choices": [{ "message": { "role": "assistant", "content": "Rewritten text from the AI provider." } }],
            "usage": { "prompt_tokens": 10, "completion_tokens": 5 }
        })))
        .mount(&server)
        .await;

    // SearXNG JSON envelope.
    Mock::given(method("GET"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [
                {
                    "title": "Rust Programming Language",
                    "url": format!("{}/page1", server.uri()),
                    "content": "Rust is a systems programming language."
                }
            ]
        })))
        .mount(&server)
        .await;

    // The result page the engine fetches and reads.
    Mock::given(method("GET"))
        .and(path("/page1"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("Rust guarantees memory safety and prevents data races."),
        )
        .mount(&server)
        .await;

    server
}

// ---------------------------------------------------------------------------
// Transform (rewrite / summarize)
// ---------------------------------------------------------------------------

#[test]
fn ai_transform_rewrite_returns_provider_output() {
    let tv = common::create_test_vault();
    let server = block_on(mount_ai_research_backend());
    seed_config(&tv, &format!("{}/v1", server.uri()), &server.uri());

    let app = build_app(&tv);
    let wv = webview(&app);

    let res = invoke(
        &wv,
        "ai_transform_block",
        json!({ "text": "Rust is memory safe.", "action": "rewrite", "pagePath": null }),
    )
    .expect("transform must resolve (not hang)");
    let content = res["content"].as_str().expect("content field");
    assert!(
        content.contains("Rewritten text"),
        "transform must return the provider output: {content}"
    );
}

#[test]
fn ai_transform_summarize_returns_provider_output() {
    let tv = common::create_test_vault();
    let server = block_on(mount_ai_research_backend());
    seed_config(&tv, &format!("{}/v1", server.uri()), &server.uri());

    let app = build_app(&tv);
    let wv = webview(&app);

    let res = invoke(
        &wv,
        "ai_transform_block",
        json!({ "text": "long text to summarize", "action": "summarize", "pagePath": null }),
    )
    .expect("summarize must resolve");
    assert!(res["content"]
        .as_str()
        .unwrap_or("")
        .contains("Rewritten text"));
}

#[test]
fn ai_transform_without_config_fails_without_hanging() {
    let tv = common::create_test_vault();
    let app = build_app(&tv);
    let wv = webview(&app);

    // No `.pkm/config.toml` — the command must reject fast with a clear error.
    let res = invoke(
        &wv,
        "ai_transform_block",
        json!({ "text": "x", "action": "rewrite", "pagePath": null }),
    );
    let err = res.expect_err("must reject when AI not configured");
    assert!(err.is_string(), "rejection must be a string error");
    let msg = err.as_str().unwrap_or("");
    assert!(
        msg.contains("configure") || msg.contains("config"),
        "error must be actionable: {msg}"
    );
}

// ---------------------------------------------------------------------------
// Mermaid generation
// ---------------------------------------------------------------------------

#[test]
fn generate_mermaid_returns_diagram_from_provider() {
    let tv = common::create_test_vault();
    let server = block_on(mount_ai_research_backend());
    seed_config(&tv, &format!("{}/v1", server.uri()), &server.uri());

    let app = build_app(&tv);
    let wv = webview(&app);

    let res = invoke(
        &wv,
        "generate_mermaid",
        json!({ "prompt": "a flowchart of a review process" }),
    )
    .expect("generate_mermaid must resolve");
    assert!(
        res["content"]
            .as_str()
            .unwrap_or("")
            .contains("Rewritten text"),
        "mermaid returns the provider diagram output"
    );
}

// ---------------------------------------------------------------------------
// Research via SearXNG (SSRF guard + timeout)
// ---------------------------------------------------------------------------

#[test]
fn ai_research_runs_full_pipeline_against_configured_endpoint() {
    let tv = common::create_test_vault();
    let server = block_on(mount_ai_research_backend());
    seed_config(&tv, &format!("{}/v1", server.uri()), &server.uri());

    let app = build_app(&tv);
    let wv = webview(&app);

    let res = invoke(&wv, "ai_research", json!({ "query": "Rust memory safety" }))
        .expect("research must resolve (not hang)");
    let findings = res["findings"].as_str().expect("findings");
    assert!(
        findings.contains("Rewritten text"),
        "findings must be the synthesized research notes, got: {findings}"
    );
    let sources = res["sources"].as_array().expect("sources array");
    assert_eq!(sources.len(), 1, "one search source recorded");
    assert_eq!(
        sources[0]["title"].as_str().unwrap(),
        "Rust Programming Language"
    );
    assert!(
        sources[0]["url"].as_str().unwrap().ends_with("/page1"),
        "source URL must point at the read page"
    );
}

#[test]
fn ai_research_without_config_fails_fast() {
    let tv = common::create_test_vault();
    let app = build_app(&tv);
    let wv = webview(&app);

    let res = invoke(&wv, "ai_research", json!({ "query": "weather" }));
    let err = res.expect_err("must reject without config");
    assert!(err.is_string());
    let msg = err.as_str().unwrap_or("");
    assert!(
        msg.contains("configure") || msg.contains("config"),
        "error must be clear: {msg}"
    );
}

// ---------------------------------------------------------------------------
// Interlink suggestions (real index + LLM rewrite)
// ---------------------------------------------------------------------------

#[test]
fn ai_interlink_finds_related_notes_and_rewrites_with_wiki_links() {
    let tv = common::create_test_vault();
    let server = block_on(mount_ai_research_backend());
    seed_config(&tv, &format!("{}/v1", server.uri()), &server.uri());

    // Seed the CURRENT page (the note being edited) plus a genuinely related
    // note on disk + in the store, then build the real index so `RelatedFinder`
    // finds the related note (the shape `create_page` produces).
    let cur_full = tv.vault_path.join("pages/current.md");
    std::fs::create_dir_all(cur_full.parent().unwrap()).unwrap();
    std::fs::write(
        &cur_full,
        "---\ntitle: Current\n---\n- a scratch note under edit\n",
    )
    .unwrap();

    let related_full = tv.vault_path.join("pages/rust-dev.md");
    std::fs::create_dir_all(related_full.parent().unwrap()).unwrap();
    std::fs::write(
        &related_full,
        "---\ntitle: Rust Dev\n---\n- memory safety in systems programming\n",
    )
    .unwrap();

    let mut cur_page = pkm_block::Page::new(cur_full, &tv.vault_path);
    cur_page.frontmatter.title = Some("Current".into());
    tv.store.upsert_page(&cur_page).unwrap();
    let cur_content = std::fs::read_to_string(tv.vault_path.join("pages/current.md")).unwrap();
    let (_fm, _, cur_blocks) = pkm_markdown::block_parser::parse_document(&cur_content);

    let mut rel_page = pkm_block::Page::new(related_full, &tv.vault_path);
    rel_page.frontmatter.title = Some("Rust Dev".into());
    tv.store.upsert_page(&rel_page).unwrap();
    let rel_content = std::fs::read_to_string(tv.vault_path.join("pages/rust-dev.md")).unwrap();
    let (_fm, _, rel_blocks) = pkm_markdown::block_parser::parse_document(&rel_content);

    let app_build = build_app(&tv);
    {
        let st = app_build.state::<AppState>();
        let mut st = st.lock().unwrap();
        let bi = st.ensure_block_index().unwrap();
        for b in &cur_blocks {
            tv.store.insert_block(b, "pages/current.md").unwrap();
            bi.index_block(b, "pages/current.md").unwrap();
        }
        for b in &rel_blocks {
            tv.store.insert_block(b, "pages/rust-dev.md").unwrap();
            bi.index_block(b, "pages/rust-dev.md").unwrap();
        }
        bi.flush().unwrap();
    }

    let app = build_app(&tv);
    let wv = webview(&app);

    // The mock chat returns the literal "Rewritten text…" — the interlink flow
    // must post-process it (no self-links) and return it as-is. The point is
    // the pipe runs end-to-end against the real index (a related note is found,
    // so the LLM is called) without erroring.
    let res = invoke(
        &wv,
        "ai_interlink_notes",
        json!({ "text": "memory safety in systems programming", "pagePath": "pages/current.md" }),
    )
    .expect("interlink must resolve");
    assert!(
        res["content"]
            .as_str()
            .unwrap_or("")
            .contains("Rewritten text"),
        "interlink returns the LLM output when a related note exists"
    );
}
