//! E7.F7 — Verify AI & research: API-key-not-in-plaintext acceptance coverage.
//!
//! These tests drive the REAL `#[tauri::command]` handlers (`get_settings`,
//! `save_settings`) over the real Tauri IPC dispatcher against a REAL temp
//! vault, matching the sibling E7 command-level harnesses. Nothing is stubbed:
//! config load/save, the masking helper, and the masked-key preservation path
//! are all production code.
//!
//! Acceptance contract under test (security-audit residual #6 "API key stored
//! in plaintext — env var fallback insufficient", fixed by the masking +
//! preserve-on-save work):
//!   * `get_settings` returns a MASKED key (first 3 + last 4 chars, or `****`)
//!     so the frontend never receives the plaintext secret
//!   * `save_settings` round-trips a masked placeholder back to the on-disk
//!     secret instead of overwriting it with `****`
//!   * saving a genuine plaintext key updates the stored secret
//!   * the env-var source indicator is reported for provider families that
//!     support env fallback (openai/custom-openai), so the UI can show where
//!     the effective key came from

mod common;

use app_lib::commands::vault::{AppState, VaultState};
use serde_json::json;
use std::sync::Mutex;
use tauri::test::{mock_builder, mock_context, noop_assets};
use tauri::webview::InvokeRequest;
use tauri::WebviewWindow;

fn build_app(vault: &common::TestVault) -> tauri::App<tauri::test::MockRuntime> {
    let vs = VaultState::new(vault.vault_path.clone());
    mock_builder()
        .manage(Mutex::new(vs) as AppState)
        .invoke_handler(tauri::generate_handler![
            app_lib::commands::settings::get_settings,
            app_lib::commands::settings::save_settings,
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

/// Write a config.toml under the vault's `.pkm/` carrying an `[ai]` section
/// with a fixed secret, mirroring what a user would configure.
fn seed_config_with_key(tv: &common::TestVault, key: &str) {
    let dir = tv.vault_path.join(".pkm");
    std::fs::create_dir_all(&dir).unwrap();
    let toml = format!(
        "vault_path = \"{}\"\n\n[ai]\nprovider = \"CustomOpenAI\"\nendpoint = \"http://localhost:8080/v1\"\napi_key = \"{key}\"\nmodel = \"llama3.2\"\nrag_enabled = true\nrag_chunk_count = 3\n\n[research]\nsearxng_endpoint = \"http://localhost:8888\"\nmax_results = 3\nmax_depth = 2\n",
        tv.vault_path.display()
    );
    std::fs::write(dir.join("config.toml"), toml).unwrap();
}

/// Build a `save_settings` body carrying the given (possibly masked) AI key,
/// with all other sections defaulted to values compatible with the handler.
fn settings_body(tv: &common::TestVault, ai_key: serde_json::Value) -> serde_json::Value {
    json!({
        "vault_path": tv.vault_path,
        "theme": { "dark_mode": false, "primary_color": "#fff", "secondary_color": "#000", "font_size": 14 },
        "ai": {
            "provider": "custom-openai",
            "endpoint": "http://localhost:8080/v1",
            "api_key": ai_key,
            "model": "llama3.2",
            "models": [],
            "rag_enabled": true,
            "rag_chunk_count": 3,
            "embedding_dimensions": 0
        },
        "research": { "searxng_endpoint": "http://localhost:8888", "max_results": 3, "max_depth": 2 },
        "graph": { "show_connected": true, "show_orphaned": true, "show_tags": true, "charge_strength": 30.0, "link_distance": 120.0, "alpha_decay": 0.1, "velocity_decay": 0.2, "link_curvature": 0.0 },
        "sync": { "mode": "manual", "branch": "master", "auto_commit_interval_secs": 300, "auto_sync_interval_secs": 3600, "commit_template": "{{message}}" },
        "stt": { "endpoint": "http://localhost:9001", "model": "whisper-1", "diarize_model": "", "language": null, "diarize": false, "auto_summarize": false, "auto_identify": false },
        "tts": { "endpoint": "", "voice": "alloy", "format": "mp3", "speed": 1.0 }
    })
}

#[test]
fn get_settings_masks_api_key_everywhere() {
    let tv = common::create_test_vault();
    seed_config_with_key(&tv, "sk-secretvalue1234567890");
    let app = build_app(&tv);
    let wv = webview(&app);

    let res = invoke(&wv, "get_settings", json!({})).expect("resolve");
    let ai_key = res["ai"]["api_key"].as_str().expect("ai.api_key present");

    assert!(
        ai_key.contains("****"),
        "ai key must be masked, got: {ai_key}"
    );
    assert!(
        ai_key.starts_with("sk-") && ai_key.ends_with("7890"),
        "mask keeps identifiable prefix/suffix: {ai_key}"
    );
    assert!(
        !ai_key.contains("secretvalue"),
        "middle of the key must not leak: {ai_key}"
    );
    assert!(
        !ai_key.contains("secretvalue1234567890"),
        "full key must not appear"
    );
}

#[test]
fn save_settings_masked_key_preserves_stored_secret() {
    let tv = common::create_test_vault();
    seed_config_with_key(&tv, "sk-originalsecret999");
    let app = build_app(&tv);
    let wv = webview(&app);

    // The frontend sends back the masked placeholder (no plaintext in transit).
    let masked = "sk-****999";
    invoke(
        &wv,
        "save_settings",
        json!({ "settings": settings_body(&tv, json!(masked)) }),
    )
    .expect("save resolves");

    // The on-disk secret must be preserved, not overwritten with `****`.
    let on_disk = std::fs::read_to_string(tv.vault_path.join(".pkm").join("config.toml")).unwrap();
    assert!(
        on_disk.contains("sk-originalsecret999"),
        "stored secret must survive a masked save: {on_disk}"
    );
    assert!(
        !on_disk.contains("****999"),
        "masked placeholder must not reach disk"
    );
}

#[test]
fn save_settings_valid_key_overwrites_stored_secret() {
    let tv = common::create_test_vault();
    seed_config_with_key(&tv, "sk-oldsecret111");
    let app = build_app(&tv);
    let wv = webview(&app);

    // A genuinely new key (no asterisks) must be persisted.
    invoke(
        &wv,
        "save_settings",
        json!({ "settings": settings_body(&tv, json!("sk-newsecret222")) }),
    )
    .expect("save resolves");

    let on_disk = std::fs::read_to_string(tv.vault_path.join(".pkm").join("config.toml")).unwrap();
    assert!(
        on_disk.contains("sk-newsecret222"),
        "new key must be stored"
    );
    assert!(
        !on_disk.contains("sk-oldsecret111"),
        "old key must be replaced"
    );
}

#[test]
fn get_settings_reports_env_key_source_for_openai_family() {
    let tv = common::create_test_vault();
    seed_config_with_key(&tv, "sk-secretvalue1234567890");
    let app = build_app(&tv);
    let wv = webview(&app);

    // With a custom-openai provider and no OPENAI_API_KEY in the env, the flag
    // must be false, so the UI does not claim the key came from the environment.
    unsafe {
        std::env::remove_var("OPENAI_API_KEY");
    }
    let res = invoke(&wv, "get_settings", json!({})).expect("resolve");
    assert_eq!(
        res["ai"]["api_key_from_env"].as_bool(),
        Some(false),
        "no env key configured -> api_key_from_env=false"
    );

    // With OPENAI_API_KEY set, the flag must flip to true while the config file
    // key is still masked.
    unsafe {
        std::env::set_var("OPENAI_API_KEY", "sk-envvarsource456");
    }
    let res2 = invoke(&wv, "get_settings", json!({})).expect("resolve");
    assert_eq!(
        res2["ai"]["api_key_from_env"].as_bool(),
        Some(true),
        "OPENAI_API_KEY set -> api_key_from_env=true"
    );
    let ai_key = res2["ai"]["api_key"].as_str().unwrap_or("");
    assert!(
        ai_key.contains("****"),
        "config key stays masked under env path"
    );
    unsafe {
        std::env::remove_var("OPENAI_API_KEY");
    }
}
