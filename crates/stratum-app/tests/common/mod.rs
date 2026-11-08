//! Shared fixtures for headless `stratum-app` service tests.
//!
//! These tests drive [`StratumService`] directly — no Tauri runtime, no IPC.
//! The fixture creates a real temp vault with the same directory layout the
//! production boot path expects, and [`RecordingSink`] captures the events a
//! command would have forwarded to the host event bus.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use stratum_app::{EventSink, StratumService};
use tempfile::TempDir;

/// A headless test vault fixture. The `TempDir` is kept alive for the lifetime
/// of the struct so the service can keep its SQLite/Tantivy handles open.
pub struct TestVault {
    pub _dir: TempDir,
    pub vault_path: PathBuf,
    pub service: StratumService,
}

impl TestVault {
    /// Write a vault-relative file, creating parent directories as needed.
    pub fn write_file(&self, rel_path: &str, content: &str) -> PathBuf {
        let full = self.vault_path.join(rel_path);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&full, content).unwrap();
        full
    }

    /// Read a vault-relative file as UTF-8.
    pub fn read_file(&self, rel_path: &str) -> String {
        std::fs::read_to_string(self.vault_path.join(rel_path)).unwrap()
    }

    /// Whether a vault-relative path exists on disk.
    pub fn exists(&self, rel_path: &str) -> bool {
        self.vault_path.join(rel_path).exists()
    }

    /// Write `.pkm/config.toml` with the supplied TOML body.
    pub fn write_config(&self, toml: &str) {
        let dir = self.vault_path.join(".pkm");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("config.toml"), toml).unwrap();
    }

    /// Persist a typed `pkm_core::Config` to `<vault>/.pkm/config.toml`.
    pub fn save_config(&self, config: &pkm_core::Config) {
        let dir = self.vault_path.join(".pkm");
        std::fs::create_dir_all(&dir).unwrap();
        config.save(dir.join("config.toml")).unwrap();
    }

    /// A default config rooted at this vault.
    pub fn config(&self) -> pkm_core::Config {
        pkm_core::Config {
            vault_path: self.vault_path.clone(),
            ..Default::default()
        }
    }

    /// Install a plugin manager into the vault state (production boot does the
    /// same in `setup_vault`). Returns the manager for direct assertions.
    pub fn init_plugin_manager(&self) -> std::sync::Arc<stratum_app::plugins::PluginManager> {
        let manager = stratum_app::plugins::PluginManager::init_for_vault(&self.vault_path)
            .expect("init plugin manager");
        self.service.lock().unwrap().plugin_manager = Some(manager.clone());
        manager
    }

    /// Open a fresh block store for assertions that need direct SQL access.
    pub fn store(&self) -> pkm_block::BlockStore {
        pkm_block::BlockStore::open(&self.vault_path.join(".pkm").join("blocks.db")).unwrap()
    }
}

/// Create a fully-realized test vault in a temp directory.
pub fn create_test_vault() -> TestVault {
    let dir = TempDir::new().unwrap();
    let vault_path = dir.path().to_path_buf();

    std::fs::create_dir_all(vault_path.join(".pkm")).unwrap();
    std::fs::create_dir_all(vault_path.join("pages")).unwrap();
    std::fs::create_dir_all(vault_path.join("journals")).unwrap();
    std::fs::create_dir_all(vault_path.join("templates")).unwrap();

    let service = StratumService::new(vault_path.clone());

    TestVault {
        _dir: dir,
        vault_path,
        service,
    }
}

/// Create an empty directory (not a full vault) for import-source fixtures.
pub fn temp_dir() -> TempDir {
    TempDir::new().unwrap()
}

/// Event sink that records every emitted event for assertions.
#[derive(Clone, Default)]
pub struct RecordingSink {
    events: Arc<Mutex<Vec<(String, serde_json::Value)>>>,
}

impl RecordingSink {
    pub fn new() -> Self {
        Self::default()
    }

    /// Snapshot of `(event, payload)` pairs captured so far.
    pub fn events(&self) -> Vec<(String, serde_json::Value)> {
        self.events.lock().unwrap().clone()
    }

    /// Whether any captured event has the given name.
    pub fn has_event(&self, name: &str) -> bool {
        self.events
            .lock()
            .unwrap()
            .iter()
            .any(|(event, _)| event == name)
    }
}

impl EventSink for RecordingSink {
    fn emit(&self, event: &str, payload: serde_json::Value) {
        self.events
            .lock()
            .unwrap()
            .push((event.to_string(), payload));
    }

    fn shared(&self) -> Arc<dyn EventSink> {
        Arc::new(self.clone())
    }
}

/// Assert that `path` is contained in `root` (helper for path assertions).
pub fn assert_within(root: &Path, path: &Path) {
    assert!(
        path.starts_with(root),
        "expected {path:?} to be within {root:?}"
    );
}

/// Drop the vault's plugin runtime on a dedicated OS thread.
///
/// `PluginManager` owns an embedded tokio runtime; dropping it from inside an
/// async context panics on tokio >= 1.52 ("Cannot drop a runtime in a context
/// where blocking is not allowed"). Production boot has the same workaround in
/// `setup_vault`; headless tests must do it before the fixture is dropped at the
/// end of a `#[tokio::test]` body.
pub fn drop_plugin_runtime(service: &StratumService) {
    let manager = service
        .lock()
        .ok()
        .and_then(|mut state| state.plugin_manager.take());
    if let Some(manager) = manager {
        std::thread::spawn(move || drop(manager)).join().ok();
    }
}

/// Mount a wiremock server exposing an OpenAI-compatible chat surface and a
/// SearXNG search surface (the same shape the production engines consume).
pub async fn mount_llm_research_backend() -> wiremock::MockServer {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, ResponseTemplate};

    let server = wiremock::MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "choices": [{ "message": { "role": "assistant", "content": "AI output with [[rust notes]]" } }],
            "usage": { "prompt_tokens": 10, "completion_tokens": 5 }
        })))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [{
                "title": "Rust Programming Language",
                "url": format!("{}/page1", server.uri()),
                "content": "Rust is a systems programming language."
            }]
        })))
        .mount(&server)
        .await;

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

/// Seed a config with `[ai]` + `[research]` pointed at `ai_endpoint` and
/// `searxng_endpoint` (an OpenAI-compatible custom provider).
pub fn seed_ai_config(vault: &TestVault, ai_endpoint: &str, searxng_endpoint: &str) {
    let mut config = vault.config();
    config.ai.provider = pkm_core::AiProvider::CustomOpenAI;
    config.ai.endpoint = Some(ai_endpoint.to_string());
    config.ai.api_key = Some("sk-test".to_string());
    config.ai.model = "test-model".to_string();
    config.ai.rag_enabled = true;
    config.ai.rag_chunk_count = 3;
    config.research.searxng_endpoint = searxng_endpoint.to_string();
    config.research.max_results = 3;
    config.research.max_depth = 1;
    vault.save_config(&config);
}

/// Can the host open a live input stream? Guard for mic-dependent tests.
pub fn audio_available() -> bool {
    use std::sync::atomic::{AtomicBool, Ordering};
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

/// Encode `seconds` of synthetic audio as a valid FLAC clip.
pub fn make_flac_clip(path: &std::path::Path, seconds: u32) -> std::path::PathBuf {
    let rate = 16_000u32;
    let samples: Vec<f32> = (0..rate as usize * seconds as usize)
        .map(|i| (i as f32 * 220.0 * std::f32::consts::TAU / rate as f32).sin() * 0.4)
        .collect();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    pkm_audio::encode_flac(&samples, rate, path).unwrap();
    path.to_path_buf()
}

/// Build a minimal [`stratum_app::block::BlockDto`] for save/get tests.
pub fn block_dto(content: &str) -> stratum_app::block::BlockDto {
    stratum_app::block::BlockDto {
        id: uuid::Uuid::new_v4().to_string(),
        content: content.to_string(),
        parent_id: None,
        left_id: None,
        properties: Vec::new(),
        marker: None,
        priority: None,
        collapsed: false,
        heading_level: None,
    }
}
