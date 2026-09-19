//! E7.F8 — Voice dictation command-layer verification (mic-free surface).
//!
//! Drives the REAL `#[tauri::command]` handlers over the real Tauri IPC
//! dispatcher (official `tauri::test` mock-app harness) against a REAL temp
//! vault. Covers the acceptance surface that does not depend on a live
//! microphone or audio server:
//!   * `dictation_cancel` with no active recording → clean guard error.
//!   * `dictation_start` / `dictation_stop` — full capture lifecycle when a
//!     real input device is available (skipped when the CI/host has no audio
//!     service, which the sibling mobile/desktop flows treat the same way).
//!   * `speaker_list` / `speaker_delete` — the vault speaker registry
//!     round-trips through the command layer, and deletions persist to disk.
//!   * `stt_test_connection` — probes the configured endpoint; asserts the
//!     happy path against the live service (skipped when the service or the
//!     config is absent) and the failure path against a dead port.
//!
//! The live ASR round-trip itself (mic → FLAC → STT → diarize → enrich →
//! insert) is verified at the crate layer in
//! `crates/pkm-dictation/tests/real_endpoint_live.rs`, and the recordings-dir
//! cleanup / atomic FLAC write contract in
//! `crates/pkm-audio/tests/recordings_hygiene.rs`.

mod common;

use app_lib::commands::vault::{AppState, VaultState};
use serde_json::json;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use tauri::test::{mock_builder, mock_context, noop_assets};
use tauri::webview::InvokeRequest;
use tauri::WebviewWindow;

/// Build a mock Tauri app whose `VaultState` is backed by `vault`, with the
/// dictation command handlers registered.
fn build_app(vault: &common::TestVault) -> tauri::App<tauri::test::MockRuntime> {
    let vs = VaultState::new(vault.vault_path.clone());
    mock_builder()
        .manage(Mutex::new(vs) as AppState)
        .invoke_handler(tauri::generate_handler![
            app_lib::commands::dictation::dictation_start,
            app_lib::commands::dictation::dictation_stop,
            app_lib::commands::dictation::dictation_cancel,
            app_lib::commands::dictation::speaker_list,
            app_lib::commands::dictation::speaker_delete,
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

/// Write the STT config that `dictation_transcribe` / `stt_test_connection`
/// load from `<vault>/.pkm/config.toml`.
fn write_stt_config(vault: &common::TestVault, endpoint: &str) {
    std::fs::create_dir_all(vault.vault_path.join(".pkm")).unwrap();
    std::fs::write(
        vault.vault_path.join(".pkm").join("config.toml"),
        format!(
            "[stt]\nendpoint = \"{endpoint}\"\nmodel = \"whisper-1\"\ndiarize_model = \"pyannote-diarization\"\n"
        ),
    )
    .unwrap();
}

/// Seed the speaker registry with a known voice.
fn seed_registry(vault: &common::TestVault, name: &str) {
    let reg = pkm_dictation::speakers::SpeakerRegistry {
        speakers: vec![pkm_dictation::speakers::SpeakerEntry {
            name: name.to_string(),
            clip: None,
            embedding: None,
            enrolled_at: None,
        }],
    };
    reg.save(&vault.vault_path.join(".pkm").join("speakers.toml"))
        .unwrap();
}

/// Can the host open a live input stream? Guard for mic-dependent tests.
fn audio_available() -> bool {
    static CACHE: AtomicBool = AtomicBool::new(false);
    static CHECKED: AtomicBool = AtomicBool::new(false);
    if CHECKED.load(Ordering::SeqCst) {
        return CACHE.load(Ordering::SeqCst);
    }
    let ok = pkm_audio::AudioRecorder.start().is_ok();
    CACHE.store(ok, Ordering::SeqCst);
    CHECKED.store(true, Ordering::SeqCst);
    ok
}

// ---------------------------------------------------------------------------
// Guard + non-audio paths
// ---------------------------------------------------------------------------

#[test]
fn dictation_cancel_with_no_recording_returns_guard_error() {
    let vault = common::create_test_vault();
    let app = build_app(&vault);
    let wv = webview(&app);
    let res = invoke(&wv, "dictation_cancel", json!({}));
    match res {
        Err(_) => {}
        Ok(v) => panic!("expected guard error, got {v:?}"),
    }
}

#[test]
fn dictation_start_stop_cancel_lifecycle_when_audio_available() {
    if !audio_available() {
        eprintln!("SKIP: no live audio input service on this host");
        return;
    }
    let vault = common::create_test_vault();
    vault.create_md_file("pages/Welcome.md", "# Welcome\n");
    vault.add_page("pages/Welcome.md");
    // Config so the recordings dir resolves to the vault-owned default.
    write_stt_config(&vault, "http://127.0.0.1:8081");

    let app = build_app(&vault);
    let wv = webview(&app);

    // start → returns a real recording path under assets/recordings
    let started = invoke(
        &wv,
        "dictation_start",
        json!({"page_path": "pages/Welcome.md"}),
    )
    .expect("start ok");
    let rec_path = started["recording_path"]
        .as_str()
        .expect("path")
        .to_string();
    assert!(
        rec_path.contains("assets/recordings") || rec_path.contains("recordings"),
        "clip lands in the recordings dir: {rec_path}"
    );

    // A running recorder must reject a second start.
    let second = invoke(
        &wv,
        "dictation_start",
        json!({"page_path": "pages/Welcome.md"}),
    );
    assert!(second.is_err(), "double start rejected");

    // stop → writes a real FLAC clip (atomic temp→rename).
    let stopped = invoke(&wv, "dictation_stop", json!({})).expect("stop ok");
    assert!(stopped["duration_secs"].as_f64().is_some());
    let final_path = std::path::PathBuf::from(&rec_path);
    assert!(final_path.exists(), "final clip written on disk");
    assert!(
        !final_path.with_extension("flac.tmp").exists(),
        "no .tmp residue after stop"
    );
    // First 4 bytes = FLAC magic.
    let magic = &std::fs::read(&final_path).unwrap()[..4];
    assert_eq!(magic, b"fLaC", "clip is real FLAC");
    // Recordings dir is a subdir of the vault.
    let parent = final_path.parent().unwrap();
    assert!(
        parent.starts_with(&vault.vault_path),
        "clip stays inside the vault: {}",
        parent.display()
    );
}

#[test]
fn dictation_cancel_discards_clip_and_tmp_when_audio_available() {
    if !audio_available() {
        eprintln!("SKIP: no live audio input service on this host");
        return;
    }
    let vault = common::create_test_vault();
    vault.create_md_file("pages/Welcome.md", "# Welcome\n");
    vault.add_page("pages/Welcome.md");
    write_stt_config(&vault, "http://127.0.0.1:8081");

    let app = build_app(&vault);
    let wv = webview(&app);

    let started = invoke(
        &wv,
        "dictation_start",
        json!({"page_path": "pages/Welcome.md"}),
    )
    .expect("start ok");
    let rec_path = started["recording_path"]
        .as_str()
        .expect("path")
        .to_string();

    // cancel → removes the partial clip (and any tmp), leaving dir clean.
    let cancelled = invoke(&wv, "dictation_cancel", json!({})).expect("cancel ok");
    assert_eq!(cancelled, json!(null));
    let p = std::path::PathBuf::from(&rec_path);
    // The encoder writes to .tmp then renames on stop; on cancel the call may
    // have only created the parent dir (no file yet). Either way, after a
    // clean cancel there must be no residue: not the final clip nor a .tmp.
    assert!(
        !p.exists(),
        "cancelled clip removed (final): {}",
        p.display()
    );
    assert!(
        !p.with_extension("flac.tmp").exists(),
        "cancelled clip removed (tmp): {}",
        p.display()
    );
}

#[test]
fn speaker_list_returns_registry_and_delete_persists() {
    let vault = common::create_test_vault();
    seed_registry(&vault, "Alice");
    let app = build_app(&vault);
    let wv = webview(&app);

    let list = invoke(&wv, "speaker_list", json!({})).expect("list ok");
    let speakers = list.as_array().expect("list is an array");
    assert_eq!(speakers.len(), 1, "one seeded speaker");
    assert_eq!(speakers[0]["name"], json!("Alice"));

    let del = invoke(&wv, "speaker_delete", json!({"name": "Alice"})).expect("delete ok");
    assert_eq!(del, json!(null));

    // Persisted on disk: registry file now empty.
    let on_disk = pkm_dictation::speakers::SpeakerRegistry::load(
        &vault.vault_path.join(".pkm").join("speakers.toml"),
    )
    .expect("registry loads");
    assert!(on_disk.names().is_empty(), "delete persisted");

    let list2 = invoke(&wv, "speaker_list", json!({})).expect("list ok");
    assert_eq!(list2.as_array().unwrap().len(), 0, "deleted gone from list");
}

#[test]
fn stt_test_connection_fails_cleanly_on_dead_port() {
    let vault = common::create_test_vault();
    // Reserve a port that is very unlikely to be serving an OpenAI endpoint.
    write_stt_config(&vault, "http://127.0.0.1:9");
    let app = build_app(&vault);
    let wv = webview(&app);
    let res = invoke(&wv, "stt_test_connection", json!({}));
    // Failing fast is the contract; the mock harness deserializes the Err as
    // a JSON rejection, which surfaces as Err from `invoke`.
    assert!(res.is_err(), "dead endpoint must not report ok");
}

// ---------------------------------------------------------------------------
// Live-service-dependent paths (skipped when no live endpoint is reachable)
// ---------------------------------------------------------------------------

fn live_endpoint() -> Option<String> {
    std::env::var("STRATUM_LIVE_STT")
        .ok()
        .or_else(|| Some("http://127.0.0.1:8081".to_string()))
        .filter(|url| live_endpoint_up(url))
}

// NOTE: reqwest is a transitive dep of stratum-tauri; blocking requires the
// "blocking" feature. The crate uses non-blocking reqwest, so this helper is
// async-running through a manual HTTP check isn't available without feature
// toggling. We instead gate via a simple TCP connect probe.
fn live_endpoint_up(url: &str) -> bool {
    let host_port = url
        .trim_start_matches("http://")
        .trim_end_matches('/')
        .to_string();
    let (host, port) = match host_port.rsplit_once(':') {
        Some((h, p)) => (h.to_string(), p.parse::<u16>().unwrap_or(0)),
        None => return false,
    };
    std::net::TcpStream::connect_timeout(
        &format!("{host}:{port}").parse().unwrap(),
        std::time::Duration::from_secs(2),
    )
    .is_ok()
}

#[test]
fn stt_test_connection_reports_ok_and_models_against_live_service() {
    let Some(url) = live_endpoint() else {
        eprintln!("SKIP: no live STT endpoint configured/reachable");
        return;
    };
    let vault = common::create_test_vault();
    write_stt_config(&vault, &url);
    let app = build_app(&vault);
    let wv = webview(&app);
    let res = invoke(&wv, "stt_test_connection", json!({})).expect("ok");
    assert_eq!(res["ok"], json!(true), "live endpoint reports ok");
    let models = res["models"].as_array().expect("models list");
    assert!(!models.is_empty(), "live endpoint lists models");
    assert!(
        models.iter().any(|m| m == &json!("whisper-1")),
        "exposes the documented transcription model"
    );
}
