//! Regression coverage for AI-action error surfacing and config handling
//! (audit failure modes FM-2, FM-8/S3, FM-14 plus the RAG + STT command
//! surface).
//!
//! Drives the REAL `#[tauri::command]` handlers (`ai_transform_block`,
//! `ai_rag_query`, `stt_test_connection`) over the official `tauri::test`
//! mock-app harness against a REAL temp vault. Nothing on the app side is
//! faked — config loading, `ProviderFactory`, the RAG engine and the STT
//! probe are all production code. The only external surface is the configured
//! AI/STT upstream, which is a wiremock server standing in for the provider
//! endpoint.
//!
//! Coverage mapped to the audit's failure modes:
//!   * FM-2 — a missing `[ai]` config yields an actionable "no config file
//!     found … Open Settings → AI" message; a malformed MOST config parse
//!     error is a distinct "Failed to load AI configuration" error (never
//!     masked behind a generic "not configured").
//!   * FM-8/S3 — a non-2xx endpoint response surfaces the provider's HTTP
//!     error (401/5xx), not "AI not configured".
//!   * FM-14 — `OPENAI_API_KEY` env fallback authenticates the real request
//!     even when the config file carries no key; whitespace-only env values
//!     are ignored.
//!   * RAG — `ai_rag_query` succeeds end-to-end against the validated
//!     endpoint (embeddings + chat) and returns an answer with citations.
//!   * STT — `stt_test_connection` lists models from a validated endpoint and
//!     surfaces a 200-then-bad-body parse failure instead of swallowing it.
//!
//! These tests build on the patterns established in `ai_command_flows.rs` /
//! `dictation_commands.rs`; they deliberately exercise the config-error paths
//! the earlier happy-path suite does not assert on.

#![allow(clippy::needless_return)]

mod common;

use app_lib::commands::vault::{AppState, VaultState};
use serde_json::json;
use std::sync::Mutex;
use tauri::test::{mock_builder, mock_context, noop_assets};
use tauri::webview::InvokeRequest;
use tauri::WebviewWindow;

/// Drive `fut` to completion on a short-lived current-thread tokio runtime so
/// wiremock (async) can be mounted from a plain `#[test]`.
fn block_on<F: std::future::Future>(fut: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(fut)
}

/// Build a mock Tauri app whose `VaultState` is backed by `vault`, with the
/// command handlers under test registered.
fn build_app(vault: &common::TestVault) -> tauri::App<tauri::test::MockRuntime> {
    let vs = VaultState::new(vault.vault_path.clone());
    mock_builder()
        .manage(Mutex::new(vs) as AppState)
        .invoke_handler(tauri::generate_handler![
            app_lib::commands::ai::ai_transform_block,
            app_lib::commands::ai::ai_rag_query,
            app_lib::commands::dictation::stt_test_connection,
        ])
        .build(mock_context(noop_assets()))
        .expect("app build")
}

/// Drive one command invocation through the real IPC dispatcher.
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

/// Write a `.pkm/config.toml` carrying a `[ai]` section pointed at a real
/// wiremock origin, mirroring what a user configures in Settings.
///
/// The endpoint must carry the `/v1` prefix because `effective_endpoint` for
/// `CustomOpenAI` uses the configured value verbatim (no `/v1` is appended),
/// matching how `ai_command_flows.rs` seeds its config.
fn seed_ai_config(tv: &common::TestVault, ai_endpoint_base: &str) {
    let ai_endpoint = format!("{ai_endpoint_base}/v1");
    let dir = tv.vault_path.join(".pkm");
    std::fs::create_dir_all(&dir).unwrap();
    let toml = format!(
        "vault_path = \"{}\"\n\n[ai]\nprovider = \"CustomOpenAI\"\nendpoint = \"{ai_endpoint}\"\napi_key = \"sk-test\"\nmodel = \"test-model\"\nrag_enabled = true\nrag_chunk_count = 3\n",
        tv.vault_path.display()
    );
    std::fs::write(dir.join("config.toml"), toml).unwrap();
}

/// Seed the block-search index at `<vault>/.pkm/search` (the same location a
/// fresh `IndexEngine` reads from) with one content block, then flush to disk.
fn seed_vault_index(vault: &common::TestVault) {
    let mut engine = VaultState::new(vault.vault_path.clone());
    let idx = engine.ensure_block_index().expect("block index created");
    idx.index_block(
        &pkm_block::Block::new(
            uuid::Uuid::new_v4(),
            "Rust is a systems programming language focused on safety.".to_string(),
        ),
        "notes/rust.md",
    )
    .expect("indexed");
    idx.flush().expect("index flushed to disk");
}

// ---------------------------------------------------------------------------
// FM-2: config error surfacing (not masked as "AI not configured")
// ---------------------------------------------------------------------------

/// A genuinely missing `.pkm/config.toml` must produce the actionable
/// "no config file found … Open Settings → AI" message — the FM-2 contract.
#[test]
fn missing_ai_config_surfaces_actionable_message() {
    let vault = common::create_test_vault();
    // Note: no config.toml is written.
    let app = build_app(&vault);
    let wv = webview(&app);

    let res = invoke(
        &wv,
        "ai_transform_block",
        json!({
            "text": "Rephrase this",
            "action": "rewrite",
            "pagePath": "pages/test.md",
        }),
    );

    match res {
        Err(msg) => {
            let msg = msg.as_str().unwrap_or_default().to_string();
            // The FM-2 contract: the message tells the user what to fix and
            // does NOT pretend the feature is unconfigured in a generic way.
            assert!(
                msg.contains("AI is not configured yet"),
                "missing config message is actionable, got: {msg}"
            );
            assert!(
                msg.contains("Open Settings → AI"),
                "missing config message points at Settings → AI, got: {msg}"
            );
            assert!(
                !msg.contains("Failed to load AI configuration"),
                "missing-file case must not be reported as a parse failure: {msg}"
            );
        }
        Ok(v) => panic!("expected error for missing config, got {v:?}"),
    }
}

/// A malformed (unparseable) `.pkm/config.toml` must surface the parse error —
/// NOT the "AI not configured" early return — so a corrupted file is not
/// masked as "no config".
#[test]
fn invalid_ai_config_surfaces_parse_error_not_masked() {
    let vault = common::create_test_vault();
    let dir = vault.vault_path.join(".pkm");
    std::fs::create_dir_all(&dir).unwrap();
    // Deliberately malformed TOML.
    std::fs::write(
        dir.join("config.toml"),
        "[ai]\nprovider = \"CustomOpenAI\"\nendpoint = [not closed\n",
    )
    .unwrap();

    let app = build_app(&vault);
    let wv = webview(&app);

    let res = invoke(
        &wv,
        "ai_transform_block",
        json!({
            "text": "Rephrase this",
            "action": "rewrite",
            "pagePath": "pages/test.md",
        }),
    );

    match res {
        Err(msg) => {
            let msg = msg.as_str().unwrap_or_default().to_string();
            assert!(
                msg.contains("Failed to load AI configuration"),
                "parse error must be surfaced, got: {msg}"
            );
            assert!(
                !msg.contains("AI is not configured yet"),
                "parse error must not be masked as missing config: {msg}"
            );
        }
        Ok(v) => panic!("expected error for malformed config, got {v:?}"),
    }
}

// ---------------------------------------------------------------------------
// FM-8/S3: invalid endpoint / non-2xx is NOT masked as "AI not configured"
// ---------------------------------------------------------------------------

/// A configured endpoint that answers 401 (invalid key) must surface the
/// provider's HTTP error body — not "AI not configured".
#[test]
fn endpoint_401_is_not_masked_as_ai_not_configured() {
    block_on(async {
        let server = wiremock::MockServer::start().await;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, ResponseTemplate};

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
                "error": { "message": "Incorrect API key provided.", "type": "invalid_request_error" }
            })))
            .mount(&server)
            .await;

        let vault = common::create_test_vault();
        seed_ai_config(&vault, &server.uri());

        let app = build_app(&vault);
        let wv = webview(&app);

        let res = invoke(
            &wv,
            "ai_transform_block",
            json!({
                "text": "Rephrase this",
                "action": "rewrite",
                "pagePath": "pages/test.md",
            }),
        );

        match res {
            Err(msg) => {
                let msg = msg.as_str().unwrap_or_default().to_string();
                assert!(
                    msg.contains("401") || msg.contains("Incorrect API key"),
                    "HTTP error body must be surfaced, got: {msg}"
                );
                assert!(
                    !msg.contains("AI is not configured"),
                    "endpoint failure must not be masked as 'AI not configured': {msg}"
                );
            }
            Ok(v) => panic!("expected error for 401 endpoint, got {v:?}"),
        }
    });
}

/// A 2xx response that fails to parse into the OpenAI shape must surface as a
/// parse error — not "AI not configured".
#[test]
fn endpoint_garbage_2xx_surfaces_parse_error() {
    block_on(async {
        let server = wiremock::MockServer::start().await;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, ResponseTemplate};

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            // 2xx but not the documented shape (missing `choices`).
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "unexpected": true
            })))
            .mount(&server)
            .await;

        let vault = common::create_test_vault();
        seed_ai_config(&vault, &server.uri());

        let app = build_app(&vault);
        let wv = webview(&app);

        let res = invoke(
            &wv,
            "ai_transform_block",
            json!({
                "text": "Rephrase this",
                "action": "rewrite",
                "pagePath": "pages/test.md",
            }),
        );

        match res {
            Err(msg) => {
                let msg = msg.as_str().unwrap_or_default().to_string();
                assert!(
                    msg.contains("parse") || msg.contains("OpenAI"),
                    "parse failure must be surfaced, got: {msg}"
                );
                assert!(
                    !msg.contains("AI is not configured"),
                    "parse error must not be masked as 'AI not configured': {msg}"
                );
            }
            Ok(v) => panic!("expected error for unparseable 2xx, got {v:?}"),
        }
    });
}

// ---------------------------------------------------------------------------
// FM-14: env api_key fallback
// ---------------------------------------------------------------------------

/// Serializes tests that mutate the process-wide `OPENAI_API_KEY` env var —
/// parallel tests would otherwise clobber each other's env state.
static ENV_KEY_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// With no API key in the config file, a non-empty `OPENAI_API_KEY` env var
/// must be used to authenticate the outbound request. The wiremock asserts the
/// exact bearer token came from the environment.
#[test]
fn env_api_key_fallback_authenticates_request() {
    let _guard = ENV_KEY_MUTEX.lock().unwrap();
    block_on(async {
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, ResponseTemplate};

        // Isolate from any ambient value; restore afterwards.
        unsafe { std::env::remove_var("OPENAI_API_KEY") };
        unsafe { std::env::set_var("OPENAI_API_KEY", "sk-envsecret-12345") };

        const EXPECTED_BEARER: &str = "Bearer sk-envsecret-12345";
        let server = wiremock::MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .and(header("authorization", EXPECTED_BEARER))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "choices": [{ "message": { "role": "assistant", "content": "env fallback worked" } }],
                "usage": { "prompt_tokens": 5, "completion_tokens": 3 }
            })))
            .expect(1)
            .mount(&server)
            .await;

        let vault = common::create_test_vault();
        // Config file WITHOUT an api_key — only the env fallback can satisfy
        // the OpenAI provider's auth requirement. The endpoint carries the
        // `/v1` prefix because the OpenAI provider appends `/chat/completions`
        // verbatim (no `/v1` is added for it, matching `ai_command_flows.rs`).
        let dir = vault.vault_path.join(".pkm");
        std::fs::create_dir_all(&dir).unwrap();
        let toml = format!(
            "vault_path = \"{}\"\n\n[ai]\nprovider = \"OpenAI\"\nendpoint = \"{}/v1\"\nmodel = \"test-model\"\n",
            vault.vault_path.display(),
            server.uri()
        );
        std::fs::write(dir.join("config.toml"), toml).unwrap();

        let app = build_app(&vault);
        let wv = webview(&app);

        let res = invoke(
            &wv,
            "ai_transform_block",
            json!({
                "text": "Rephrase this",
                "action": "rewrite",
                "pagePath": "pages/test.md",
            }),
        )
        .expect("env-key-authenticated request succeeds");

        let content = res["content"].as_str().unwrap_or_default();
        assert_eq!(content, "env fallback worked");

        unsafe { std::env::remove_var("OPENAI_API_KEY") };
    });
}

/// A whitespace-only `OPENAI_API_KEY` env var must be ignored (treated as
/// unset), so the configured key in the file wins. Using `CustomOpenAI` here
/// guarantees the config key is present; the wiremock asserts the configured
/// key was sent, proving the whitespace env did not override it.
#[test]
fn env_api_key_whitespace_only_ignored_config_key_wins() {
    let _guard = ENV_KEY_MUTEX.lock().unwrap();
    block_on(async {
        let server = wiremock::MockServer::start().await;
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, ResponseTemplate};

        unsafe { std::env::set_var("OPENAI_API_KEY", "   ") };

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .and(header("authorization", "Bearer sk-test"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "choices": [{ "message": { "role": "assistant", "content": "config key used" } }],
                "usage": { "prompt_tokens": 1, "completion_tokens": 1 }
            })))
            .expect(1)
            .mount(&server)
            .await;

        let vault = common::create_test_vault();
        seed_ai_config(&vault, &server.uri()); // api_key = "sk-test"

        let app = build_app(&vault);
        let wv = webview(&app);

        let res = invoke(
            &wv,
            "ai_transform_block",
            json!({
                "text": "Rephrase this",
                "action": "rewrite",
                "pagePath": "pages/test.md",
            }),
        )
        .expect("config key is used when env is whitespace-only");

        let content = res["content"].as_str().unwrap_or_default();
        assert_eq!(content, "config key used");

        unsafe { std::env::remove_var("OPENAI_API_KEY") };
    });
}

// ---------------------------------------------------------------------------
// RAG: ai_rag_query success against the validated endpoint
// ---------------------------------------------------------------------------

/// `ai_rag_query` runs the full retrieva-augmented pipeline against the
/// configured endpoint: the vault search index is queried, embeddings are
/// fetched from the (mocked) provider, and the LLM answer is returned with
/// citations.
#[test]
fn rag_query_succeeds_against_validated_endpoint() {
    block_on(async {
        let server = wiremock::MockServer::start().await;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, ResponseTemplate};

        // Embeddings endpoint (`POST {endpoint}/embeddings` for CustomOpenAI
        // whose base URL already carries /v1... the client appends /embeddings).
        // The RAG engine sends [question, snippet...] and re-ranks via cosine
        // similarity; a deterministic unit vector keeps ordering well-defined.
        Mock::given(method("POST"))
            .and(path("/v1/embeddings"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [
                    { "index": 0, "object": "embedding", "embedding": [1.0, 0.0, 0.0, 0.0] },
                    { "index": 1, "object": "embedding", "embedding": [0.9, 0.1, 0.0, 0.0] }
                ]
            })))
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "choices": [{ "message": { "role": "assistant", "content": "Rust is a systems programming language focused on safety (Source 1)." } }],
                "usage": { "prompt_tokens": 20, "completion_tokens": 8 }
            })))
            .mount(&server)
            .await;

        let vault = common::create_test_vault();
        seed_ai_config(&vault, &server.uri());
        seed_vault_index(&vault);

        let app = build_app(&vault);
        let wv = webview(&app);

        let res = invoke(
            &wv,
            "ai_rag_query",
            json!({
                "question": "What is Rust?",
            }),
        )
        .expect("rag query succeeds against mocked endpoint");

        let answer = res["answer"].as_str().unwrap_or_default();
        assert!(!answer.is_empty(), "RAG returns an answer, got: {res:?}");
        assert!(
            answer.contains("Rust"),
            "answer references the retrieved source, got: {answer}"
        );
        // Citations map to the seeded note; embedding re-ranking keeps at
        // least the single indexed block in the top-k.
        let citations = res["citations"].as_array().expect("citations array");
        assert!(
            !citations.is_empty(),
            "RAG returns citations for a seeded source, got: {res:?}"
        );
        assert_eq!(res["total_tokens"].as_u64(), Some(28), "usage surfaced");
    });
}

// ---------------------------------------------------------------------------
// STT: stt_test_connection against the validated endpoint
// ---------------------------------------------------------------------------

/// `stt_test_connection` lists models from a validated endpoint and reports
/// `ok=true` — the happy path for the dictation Settings probe.
#[test]
fn stt_test_connection_lists_models_from_validated_endpoint() {
    block_on(async {
        let server = wiremock::MockServer::start().await;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, ResponseTemplate};

        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": [
                    { "id": "whisper-1", "object": "model" },
                    { "id": "whisper-large-v3", "object": "model" }
                ]
            })))
            .mount(&server)
            .await;

        let vault = common::create_test_vault();
        let dir = vault.vault_path.join(".pkm");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("config.toml"),
            format!(
                "[stt]\nendpoint = \"{}\"\nmodel = \"whisper-1\"\n",
                server.uri()
            ),
        )
        .unwrap();

        let app = build_app(&vault);
        let wv = webview(&app);

        let res = invoke(&wv, "stt_test_connection", json!({})).expect("connection ok");
        assert_eq!(res["ok"], json!(true), "validated endpoint reports ok");
        let models = res["models"].as_array().expect("models list");
        assert!(
            models.iter().any(|m| m == &json!("whisper-1")),
            "exposes transcription models, got: {models:?}"
        );
    });
}

/// A 200 response whose body is not valid JSON must be surfaced as a parse
/// failure — the audit's "STT /v1/models parse failure surfaced not swallowed"
/// regression guard.
#[test]
fn stt_test_connection_surfaces_models_parse_failure() {
    block_on(async {
        let server = wiremock::MockServer::start().await;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, ResponseTemplate};

        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .respond_with(ResponseTemplate::new(200).set_body_string("not-json-{{{"))
            .mount(&server)
            .await;

        let vault = common::create_test_vault();
        let dir = vault.vault_path.join(".pkm");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("config.toml"),
            format!(
                "[stt]\nendpoint = \"{}\"\nmodel = \"whisper-1\"\n",
                server.uri()
            ),
        )
        .unwrap();

        let app = build_app(&vault);
        let wv = webview(&app);

        let res = invoke(&wv, "stt_test_connection", json!({}));
        match res {
            Err(msg) => {
                let msg = msg.as_str().unwrap_or_default().to_string();
                assert!(
                    msg.contains("could not be parsed") || msg.contains("parse"),
                    "parse failure must be surfaced with an actionable message, got: {msg}"
                );
            }
            Ok(v) => panic!("200-with-bad-body must not be reported as ok; got {v:?}"),
        }
    });
}

/// A missing `.pkm/config.toml` for the STT probe produces the actionable
/// "no config file found … Set a transcription endpoint" message, not a
/// cryptic failure.
#[test]
fn stt_test_connection_missing_config_surfaces_actionable_message() {
    let vault = common::create_test_vault();
    // No config.toml.
    let app = build_app(&vault);
    let wv = webview(&app);

    let res = invoke(&wv, "stt_test_connection", json!({}));
    match res {
        Err(msg) => {
            let msg = msg.as_str().unwrap_or_default().to_string();
            assert!(
                msg.contains("No config file found"),
                "missing STT config is actionable, got: {msg}"
            );
            assert!(
                msg.contains("Set a transcription endpoint"),
                "points the user at the fix, got: {msg}"
            );
        }
        Ok(v) => panic!("expected error for missing STT config, got {v:?}"),
    }
}
