use super::*;
use tempfile::TempDir;

#[test]
fn test_config_defaults() {
    let cfg = Config::default();
    assert_eq!(cfg.sync.mode, SyncMode::Manual);
    assert!(cfg.theme.dark_mode);
    assert_eq!(cfg.theme.font_size, 16);
    assert_eq!(cfg.watcher.debounce_ms, 500);
}

#[test]
fn test_config_save_and_load() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml");

    let cfg = Config {
        vault_path: PathBuf::from("/tmp/test-vault"),
        sync: SyncConfig {
            mode: SyncMode::AutoSync,
            remote_url: Some("git@github.com:user/vault.git".to_string()),
            auto_sync_interval_secs: 900,
            ..Default::default()
        },
        theme: ThemeConfig {
            dark_mode: false,
            font_size: 18,
            ..Default::default()
        },
        ..Default::default()
    };

    cfg.save(&config_path).unwrap();

    let loaded = Config::load(&config_path).unwrap();
    assert_eq!(loaded.vault_path, PathBuf::from("/tmp/test-vault"));
    assert_eq!(loaded.sync.mode, SyncMode::AutoSync);
    assert_eq!(
        loaded.sync.remote_url,
        Some("git@github.com:user/vault.git".to_string())
    );
    assert_eq!(loaded.sync.auto_sync_interval_secs, 900);
    assert!(!loaded.theme.dark_mode);
    assert_eq!(loaded.theme.font_size, 18);
}

#[test]
fn test_config_pkm_paths() {
    let cfg = Config {
        vault_path: PathBuf::from("/home/user/vault"),
        ..Default::default()
    };
    assert_eq!(cfg.pkm_dir(), PathBuf::from("/home/user/vault/.pkm"));
    assert_eq!(
        cfg.cache_db_path(),
        PathBuf::from("/home/user/vault/.pkm/cache.db")
    );
    assert_eq!(
        cfg.search_index_path(),
        PathBuf::from("/home/user/vault/.pkm/search.idx")
    );
}

#[test]
fn test_config_parse_error() {
    let dir = TempDir::new().unwrap();
    let bad_path = dir.path().join("nonexistent.toml");
    let result = Config::load(&bad_path);
    assert!(result.is_err());
}

#[test]
fn test_sync_mode_serde() {
    let modes = [
        (SyncMode::Manual, "Manual"),
        (SyncMode::AutoCommit, "AutoCommit"),
        (SyncMode::AutoSync, "AutoSync"),
        (SyncMode::Background, "Background"),
    ];
    for (mode, expected) in &modes {
        let serialized = serde_yaml::to_string(mode).unwrap();
        assert!(serialized.contains(expected));
        let deserialized: SyncMode = serde_yaml::from_str(&serialized).unwrap();
        assert_eq!(deserialized, *mode);
    }
}

#[test]
fn test_ai_config_defaults() {
    let ai = AiConfig::default();
    assert_eq!(ai.provider, AiProvider::Ollama);
    assert_eq!(ai.model, "llama3.2");
    assert!(ai.rag_enabled);
}

#[test]
fn test_stt_config_defaults() {
    let stt = SttConfig::default();
    assert_eq!(stt.endpoint, "");
    assert_eq!(stt.model, "whisper-1");
    assert_eq!(stt.diarize_model, "pyannote-diarization");
    assert!(stt.diarize);
    assert!(stt.auto_summarize);
    assert!(stt.auto_identify);
    assert_eq!(stt.language, None);
}

#[test]
fn test_stt_config_roundtrip() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml");

    let cfg = Config {
        stt: SttConfig {
            endpoint: "http://127.0.0.1:8081".to_string(),
            model: "whisperx-tiny".to_string(),
            language: Some("en".to_string()),
            auto_summarize: false,
            ..Default::default()
        },
        ..Default::default()
    };
    cfg.save(&config_path).unwrap();

    let loaded = Config::load(&config_path).unwrap();
    assert_eq!(loaded.stt.endpoint, "http://127.0.0.1:8081");
    assert_eq!(loaded.stt.model, "whisperx-tiny");
    assert_eq!(loaded.stt.language.as_deref(), Some("en"));
    assert!(!loaded.stt.auto_summarize);
    assert!(loaded.stt.diarize);
}

#[test]
fn test_old_config_without_stt_section_loads() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml");
    // Config TOML predating the [stt] section must still parse.
    std::fs::write(
        &config_path,
        "[theme]\ndark_mode = false\n\n[ai]\nmodel = \"llama3.2\"\n",
    )
    .unwrap();
    let loaded = Config::load(&config_path).unwrap();
    assert_eq!(loaded.stt.endpoint, "");
    assert!(!loaded.theme.dark_mode);
    assert_eq!(loaded.stt.diarize_model, "pyannote-diarization");
}

#[test]
fn test_vault_layout_recordings_dir_default() {
    assert_eq!(VaultLayout::default().recordings_dir, "assets/recordings");
}

#[test]
fn test_speakers_file_path() {
    let cfg = Config {
        vault_path: PathBuf::from("/tmp/test-vault"),
        ..Default::default()
    };
    assert_eq!(
        cfg.speakers_file_path(),
        PathBuf::from("/tmp/test-vault/.pkm/speakers.toml")
    );
}

#[test]
fn test_watcher_config_defaults() {
    let w = WatcherConfig::default();
    assert!(w.enabled);
    assert_eq!(w.debounce_ms, 500);
}

#[test]
fn test_graph_config_link_curvature_default() {
    let cfg = GraphConfig::default();
    assert_eq!(cfg.link_curvature, 0.15);
}

#[test]
fn test_graph_config_serde_round_trip() {
    let cfg = GraphConfig {
        link_curvature: 0.3,
        ..Default::default()
    };
    let json = serde_json::to_string(&cfg).unwrap();
    let deserialized: GraphConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.link_curvature, 0.3);
}

#[test]
fn test_config_with_plugins() {
    let cfg = Config {
        plugins: vec![PluginConfig {
            name: "my-plugin".to_string(),
            enabled: true,
            wasm_path: PathBuf::from("/tmp/plugin.wasm"),
            permissions: vec!["file:read".to_string(), "network".to_string()],
        }],
        ..Default::default()
    };
    assert_eq!(cfg.plugins.len(), 1);
    assert_eq!(cfg.plugins[0].name, "my-plugin");
    assert!(cfg.plugins[0].enabled);
}
