use super::*;
use tempfile::TempDir;

#[test]
fn test_config_defaults() {
    let cfg = Config::default();
    assert_eq!(cfg.sync.mode, SyncMode::Manual);
    assert!(cfg.theme.dark_mode);
    assert_eq!(cfg.theme.font_size, 16);
    assert_eq!(cfg.watcher.debounce_ms, 500);
    assert_eq!(cfg.locale, "en");
}

#[test]
fn test_config_locale_roundtrip() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml");

    let cfg = Config {
        locale: "de".to_string(),
        ..Default::default()
    };
    cfg.save(&config_path).unwrap();

    let loaded = Config::load(&config_path).unwrap();
    assert_eq!(loaded.locale, "de");

    // Unknown/unset locales are tolerated at load time (frontend falls back to en).
    let mut without_locale = cfg.clone();
    without_locale.locale = "fr".to_string();
    without_locale.save(&config_path).unwrap();
    let loaded_fr = Config::load(&config_path).unwrap();
    assert_eq!(loaded_fr.locale, "fr");
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
fn test_voice_capture_config_defaults() {
    let cfg = Config::default();
    assert!(cfg.voice.enabled);
    assert_eq!(cfg.voice.target, "journal:today");
}

#[test]
fn test_voice_capture_config_roundtrip() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml");

    let cfg = Config {
        voice: VoiceCaptureConfig {
            enabled: false,
            target: "journal:today".into(),
        },
        ..Default::default()
    };
    cfg.save(&config_path).unwrap();

    let loaded = Config::load(&config_path).unwrap();
    assert!(!loaded.voice.enabled);
    assert_eq!(loaded.voice.target, "journal:today");
}

#[test]
fn test_old_config_without_voice_section_loads() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml");
    // Config TOML predating the [voice] section must still parse.
    std::fs::write(
        &config_path,
        "[theme]\ndark_mode = false\n\n[ai]\nmodel = \"llama3.2\"\n",
    )
    .unwrap();
    let loaded = Config::load(&config_path).unwrap();
    assert!(loaded.voice.enabled);
    assert_eq!(loaded.voice.target, "journal:today");
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
fn test_onboarding_pending_defaults_to_false_on_old_config() {
    // A config written before the `onboarding_pending` field existed (or that
    // simply omits it) must load with `false` — the safe "onboarding done"
    // default — never fail to deserialize.
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml");
    std::fs::write(&config_path, "[theme]\ndark_mode = true\n").unwrap();

    let loaded = Config::load(&config_path).unwrap();
    assert!(!loaded.onboarding_pending);
}

#[test]
fn test_onboarding_pending_roundtrip() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml");

    let cfg = Config {
        onboarding_pending: true,
        ..Default::default()
    };
    cfg.save(&config_path).unwrap();

    let loaded = Config::load(&config_path).unwrap();
    assert!(loaded.onboarding_pending);

    // Clearing the flag round-trips back to false.
    let mut cleared = loaded;
    cleared.onboarding_pending = false;
    cleared.save(&config_path).unwrap();
    assert!(!Config::load(&config_path).unwrap().onboarding_pending);
}

#[test]
fn test_default_config_onboarding_pending_false() {
    assert!(!Config::default().onboarding_pending);
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

#[test]
fn test_tray_config_defaults() {
    let tray = TrayConfig::default();
    assert!(tray.enabled);
    assert!(tray.close_to_tray);
    assert_eq!(tray.quick_capture_shortcut, "CmdOrCtrl+Shift+Space");
    assert_eq!(tray.capture_target, CaptureTarget::Journal);
    assert_eq!(tray.inbox_path, "Inbox.md");
}

#[test]
fn test_tray_config_serde_yaml_roundtrip_custom_values() {
    let tray = TrayConfig {
        enabled: false,
        close_to_tray: false,
        quick_capture_shortcut: "Ctrl+Alt+Q".to_string(),
        capture_target: CaptureTarget::Inbox,
        inbox_path: "pages/Inbox.md".to_string(),
    };
    let yaml = serde_yaml::to_string(&tray).unwrap();
    let deserialized: TrayConfig = serde_yaml::from_str(&yaml).unwrap();
    assert!(!deserialized.enabled);
    assert!(!deserialized.close_to_tray);
    assert_eq!(deserialized.quick_capture_shortcut, "Ctrl+Alt+Q");
    assert_eq!(deserialized.capture_target, CaptureTarget::Inbox);
    assert_eq!(deserialized.inbox_path, "pages/Inbox.md");
}

#[test]
fn test_config_with_tray_roundtrips_preserving_tray_section() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml");

    let cfg = Config {
        vault_path: PathBuf::from("/tmp/test-vault"),
        tray: TrayConfig {
            enabled: false,
            close_to_tray: false,
            quick_capture_shortcut: "Ctrl+Alt+Q".to_string(),
            capture_target: CaptureTarget::Inbox,
            inbox_path: "pages/Inbox.md".to_string(),
        },
        ..Default::default()
    };
    cfg.save(&config_path).unwrap();

    let loaded = Config::load(&config_path).unwrap();
    assert!(!loaded.tray.enabled);
    assert!(!loaded.tray.close_to_tray);
    assert_eq!(loaded.tray.quick_capture_shortcut, "Ctrl+Alt+Q");
    assert_eq!(loaded.tray.capture_target, CaptureTarget::Inbox);
    assert_eq!(loaded.tray.inbox_path, "pages/Inbox.md");

    // A config without a [tray] section loads with tray defaults (back-compat).
    std::fs::write(&config_path, "[theme]\ndark_mode = false\n").unwrap();
    let without_tray = Config::load(&config_path).unwrap();
    assert!(without_tray.tray.enabled);
    assert_eq!(without_tray.tray.capture_target, CaptureTarget::Journal);
}

#[test]
fn test_ai_config_discovery_fields_roundtrip() {
    // build an AiConfig with the discovery fields populated, serialize +
    // deserialize as TOML, and assert equality on round-trip (E10.F2 item 7).
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml");

    let ai = AiConfig {
        extra_local_hosts: vec![
            "127.0.0.1:8123".to_string(),
            "http://192.168.1.5:11434".to_string(),
        ],
        last_discovered: vec![DiscoveredEndpoint {
            host: "http://127.0.0.1:11434".to_string(),
            name: "Ollama".to_string(),
            kind: "ollama".to_string(),
            models: vec!["llama3.2:latest".to_string()],
            status: "available".to_string(),
            latency_ms: 12,
            error: None,
        }],
        ..Default::default()
    };

    let cfg = Config {
        ai,
        ..Default::default()
    };
    cfg.save(&config_path).unwrap();

    let loaded = Config::load(&config_path).unwrap();
    assert_eq!(loaded.ai.extra_local_hosts.len(), 2);
    assert_eq!(loaded.ai.extra_local_hosts[0], "127.0.0.1:8123");
    assert_eq!(loaded.ai.extra_local_hosts[1], "http://192.168.1.5:11434");
    assert_eq!(loaded.ai.last_discovered.len(), 1);
    let ep = &loaded.ai.last_discovered[0];
    assert_eq!(ep.host, "http://127.0.0.1:11434");
    assert_eq!(ep.name, "Ollama");
    assert_eq!(ep.kind, "ollama");
    assert_eq!(ep.models, vec!["llama3.2:latest".to_string()]);
    assert_eq!(ep.status, "available");
    assert_eq!(ep.latency_ms, 12);
    assert_eq!(ep.error, None);
}

#[test]
fn test_old_config_without_discovery_fields_loads() {
    // A config TOML predating the discovery fields must still parse, loading
    // both fields as their empty defaults (E10.F2 item 7).
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml");
    std::fs::write(&config_path, "[ai]\nmodel = \"llama3.2\"\n").unwrap();
    let loaded = Config::load(&config_path).unwrap();
    assert_eq!(loaded.ai.extra_local_hosts, Vec::<String>::new());
    assert_eq!(loaded.ai.last_discovered, Vec::<DiscoveredEndpoint>::new());
}
