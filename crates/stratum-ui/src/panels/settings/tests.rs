//! Window-free tests for the native Settings surface.
//!
//! These cover the form/DTO round-trip for every shipped field, the validation
//! rules that block a save, the live-theme fallback for partial hex values, the
//! fixture persistence seam and the [`super::SETTINGS_I18N_KEYS`] catalog gate
//! across all eight locales. No GPUI window is opened.

use std::collections::HashSet;

use stratum_app::settings::{
    AiModelDto, AiSettingsDto, BackupSettingsDto, GraphSettingsDto, LayoutSettingsDto,
    ReminderSettingsDto, ResearchSettingsDto, SettingsDto, SttSettingsDto, SyncSettingsDto,
    ThemeSettingsDto, TraySettingsDto, TtsSettingsDto, VoiceSettingsDto,
};
use stratum_i18n::{t, Locale};

use super::form::{KEY_AI_EXTRA_HOSTS, KEY_THEME_FONT_SIZE, KEY_TTS_SPEED};
use super::validate::validate;
use super::{SettingsForm, SettingsTab, SETTINGS_I18N_KEYS};
use crate::provider::{FixtureProvider, SettingsData};
use crate::theme::ThemeConfig;

/// A DTO that exercises every editable field with a non-default value.
fn sample_settings() -> SettingsDto {
    SettingsDto {
        vault_path: "/vaults/round-trip".to_string(),
        locale: "de".to_string(),
        layout: LayoutSettingsDto {
            pages_view: "tree".to_string(),
        },
        theme: ThemeSettingsDto {
            dark_mode: false,
            primary_color: "#0ea5e9".to_string(),
            secondary_color: "#111827".to_string(),
            font_size: 18,
        },
        ai: AiSettingsDto {
            provider: "custom".to_string(),
            endpoint: Some("https://ai.example.test/v1".to_string()),
            api_key: Some("sk-test".to_string()),
            api_key_from_env: true,
            model: "gpt-test".to_string(),
            models: vec![AiModelDto {
                name: "gpt-test".to_string(),
                capabilities: vec!["chat".to_string(), "embedding".to_string()],
            }],
            rag_enabled: true,
            rag_chunk_count: 7,
            embedding_dimensions: 1024,
            use_llm_gateway_and_auth: true,
            extra_local_hosts: vec!["http://127.0.0.1:1234".to_string()],
            last_discovered: Vec::new(),
        },
        research: ResearchSettingsDto {
            searxng_endpoint: "https://searx.example.test".to_string(),
            max_results: 5,
            max_depth: 4,
        },
        graph: GraphSettingsDto {
            show_connected: false,
            show_orphaned: true,
            show_tags: true,
            charge_strength: -45.0,
            link_distance: 60.0,
            alpha_decay: 0.2,
            velocity_decay: 0.7,
            link_curvature: 0.3,
            use_3d: false,
        },
        sync: SyncSettingsDto {
            mode: "auto_sync".to_string(),
            remote_url: Some("https://example.test/vault.git".to_string()),
            branch: "dev".to_string(),
            auto_commit_interval_secs: 120,
            auto_sync_interval_secs: 900,
            ssh_key_path: Some("/keys/id_ed25519".to_string()),
            commit_template:
                "sync({datetime}): {editedfiles} edited, {newfiles} added, {deletedfiles} deleted, {count} total"
                    .to_string(),
            backup: BackupSettingsDto {
                enabled: true,
                mode: "webdav".to_string(),
                interval_minutes: 15,
                exclude: vec!["tmp".to_string(), "cache".to_string()],
                webdav_url: Some("https://dav.example.test/vault".to_string()),
                webdav_username: Some("alice".to_string()),
                webdav_password: Some("secret".to_string()),
                folder_path: Some("/backups".to_string()),
            },
        },
        stt: SttSettingsDto {
            endpoint: "https://stt.example.test".to_string(),
            api_key: Some("stt-key".to_string()),
            model: "whisper-test".to_string(),
            diarize_model: "pyannote-test".to_string(),
            language: Some("en".to_string()),
            diarize: false,
            auto_summarize: false,
            auto_identify: true,
            use_llm_gateway_and_auth: true,
        },
        tts: TtsSettingsDto {
            endpoint: "https://tts.example.test".to_string(),
            api_key: Some("tts-key".to_string()),
            voice: "nova".to_string(),
            format: "wav".to_string(),
            speed: 1.25,
            use_llm_gateway_and_auth: true,
        },
        tray: TraySettingsDto {
            enabled: false,
            close_to_tray: false,
            quick_capture_shortcut: "Ctrl+Shift+K".to_string(),
            capture_target: "inbox".to_string(),
            inbox_path: "pages/Inbox.md".to_string(),
        },
        voice: VoiceSettingsDto {
            enabled: false,
            target: "journal:yesterday".to_string(),
        },
        reminders: ReminderSettingsDto {
            enabled: false,
            sound: false,
        },
    }
}

#[test]
fn form_round_trips_every_settings_field() {
    let settings = sample_settings();
    let form = SettingsForm::from_settings(settings.clone());
    let candidate = form.candidate().expect("sample settings parse");
    assert_eq!(candidate, settings);
}

#[test]
fn form_round_trips_textual_inputs_through_the_registry() {
    let mut form = SettingsForm::from_settings(sample_settings());
    form.apply_input("vault_path", "/vaults/edited".to_string());
    form.apply_input("theme_primary", "#22c55e".to_string());
    form.apply_input("theme_secondary", "#0f172a".to_string());
    form.apply_input("ai_endpoint", "https://ai2.example.test".to_string());
    form.apply_input("ai_api_key", "sk-2".to_string());
    form.apply_input("ai_model", "model-2".to_string());
    form.apply_input(
        "research_endpoint",
        "https://searx2.example.test".to_string(),
    );
    form.apply_input("sync_remote_url", "https://example.test/v2.git".to_string());
    form.apply_input("sync_branch", "release".to_string());
    form.apply_input("sync_ssh_key_path", "/keys/id2".to_string());
    form.apply_input(
        "sync_commit_template",
        "sync({datetime}): {count}".to_string(),
    );
    form.apply_input("backup_folder_path", "/backups2".to_string());
    form.apply_input("backup_webdav_url", "https://dav2.example.test".to_string());
    form.apply_input("backup_webdav_username", "bob".to_string());
    form.apply_input("backup_webdav_password", "secret2".to_string());
    form.apply_input("stt_endpoint", "https://stt2.example.test".to_string());
    form.apply_input("stt_api_key", "stt-key-2".to_string());
    form.apply_input("stt_model", "whisper-2".to_string());
    form.apply_input("stt_diarize_model", "pyannote-2".to_string());
    form.apply_input("stt_language", "fr".to_string());
    form.apply_input("tts_endpoint", "https://tts2.example.test".to_string());
    form.apply_input("tts_api_key", "tts-key-2".to_string());
    form.apply_input("tts_voice", "shimmer".to_string());
    form.apply_input("tts_format", "opus".to_string());
    form.apply_input("tray_shortcut", "Ctrl+Alt+K".to_string());
    form.apply_input("tray_inbox_path", "pages/Inbox2.md".to_string());
    form.apply_input("voice_target", "journal:tomorrow".to_string());

    let candidate = form.candidate().expect("edited settings parse");
    assert_eq!(candidate.vault_path, "/vaults/edited");
    assert_eq!(candidate.theme.primary_color, "#22c55e");
    assert_eq!(candidate.theme.secondary_color, "#0f172a");
    assert_eq!(
        candidate.ai.endpoint.as_deref(),
        Some("https://ai2.example.test")
    );
    assert_eq!(candidate.ai.api_key.as_deref(), Some("sk-2"));
    assert_eq!(candidate.ai.model, "model-2");
    assert_eq!(
        candidate.research.searxng_endpoint,
        "https://searx2.example.test"
    );
    assert_eq!(
        candidate.sync.remote_url.as_deref(),
        Some("https://example.test/v2.git")
    );
    assert_eq!(candidate.sync.branch, "release");
    assert_eq!(candidate.sync.ssh_key_path.as_deref(), Some("/keys/id2"));
    assert_eq!(candidate.sync.commit_template, "sync({datetime}): {count}");
    assert_eq!(
        candidate.sync.backup.folder_path.as_deref(),
        Some("/backups2")
    );
    assert_eq!(
        candidate.sync.backup.webdav_url.as_deref(),
        Some("https://dav2.example.test")
    );
    assert_eq!(
        candidate.sync.backup.webdav_username.as_deref(),
        Some("bob")
    );
    assert_eq!(
        candidate.sync.backup.webdav_password.as_deref(),
        Some("secret2")
    );
    assert_eq!(candidate.stt.endpoint, "https://stt2.example.test");
    assert_eq!(candidate.stt.api_key.as_deref(), Some("stt-key-2"));
    assert_eq!(candidate.stt.model, "whisper-2");
    assert_eq!(candidate.stt.diarize_model, "pyannote-2");
    assert_eq!(candidate.stt.language.as_deref(), Some("fr"));
    assert_eq!(candidate.tts.endpoint, "https://tts2.example.test");
    assert_eq!(candidate.tts.api_key.as_deref(), Some("tts-key-2"));
    assert_eq!(candidate.tts.voice, "shimmer");
    assert_eq!(candidate.tts.format, "opus");
    assert_eq!(candidate.tray.quick_capture_shortcut, "Ctrl+Alt+K");
    assert_eq!(candidate.tray.inbox_path, "pages/Inbox2.md");
    assert_eq!(candidate.voice.target, "journal:tomorrow");
}

#[test]
fn invalid_raw_text_blocks_the_candidate() {
    let mut form = SettingsForm::from_settings(sample_settings());
    form.apply_input(KEY_THEME_FONT_SIZE, "huge".to_string());
    form.apply_input(KEY_TTS_SPEED, "fast".to_string());
    form.apply_input(KEY_AI_EXTRA_HOSTS, "http://ok.test\nnot a url".to_string());

    let errors = form.candidate().expect_err("bad text must fail");
    assert_eq!(
        errors.get("theme_font_size"),
        Some(&"settings.validation.range")
    );
    assert_eq!(errors.get("tts_speed"), Some(&"settings.validation.range"));
    let theme = form.theme_candidate().expect("hex colours stay valid");
    assert_eq!(theme.font_size, Some(18.0));
}

#[test]
fn validation_rejects_every_malformed_field() {
    let mut settings = sample_settings();
    settings.vault_path.clear();
    settings.theme.primary_color = "orange".to_string();
    settings.theme.secondary_color = "#12345".to_string();
    settings.theme.font_size = 4;
    settings.ai.endpoint = Some("localhost:1".to_string());
    settings.ai.rag_chunk_count = 0;
    settings.ai.embedding_dimensions = 5000;
    settings.ai.extra_local_hosts = vec!["nope".to_string()];
    settings.research.searxng_endpoint = "ftp://searx.test".to_string();
    settings.research.max_results = 0;
    settings.research.max_depth = 9;
    settings.sync.branch.clear();
    settings.sync.auto_commit_interval_secs = 5;
    settings.sync.auto_sync_interval_secs = 10;
    settings.sync.commit_template = "no placeholder".to_string();
    settings.sync.backup.interval_minutes = 0;
    settings.sync.backup.webdav_url = None;
    settings.stt.endpoint = "bad".to_string();
    settings.tts.endpoint = "bad".to_string();
    settings.tts.voice.clear();
    settings.tts.format.clear();
    settings.tts.speed = 9.0;
    settings.tray.quick_capture_shortcut.clear();
    settings.tray.inbox_path.clear();
    settings.voice.target.clear();

    let errors = validate(&settings);
    for field in [
        "vault_path",
        "theme_primary",
        "theme_secondary",
        "theme_font_size",
        "ai_endpoint",
        "ai_rag_chunk_count",
        "ai_embedding_dimensions",
        "ai_extra_local_hosts",
        "research_endpoint",
        "research_max_results",
        "research_max_depth",
        "sync_branch",
        "sync_commit_interval",
        "sync_sync_interval",
        "sync_commit_template",
        "backup_interval",
        "backup_webdav_url",
        "stt_endpoint",
        "tts_endpoint",
        "tts_voice",
        "tts_format",
        "tts_speed",
        "tray_shortcut",
        "tray_inbox_path",
        "voice_target",
    ] {
        assert!(errors.contains_key(field), "missing validation for {field}");
    }
    assert!(validate(&sample_settings()).is_empty());
}

#[test]
fn theme_candidate_falls_back_until_the_hex_is_valid() {
    let mut form = SettingsForm::from_settings(sample_settings());
    form.apply_input("theme_primary", "#0ea5".to_string());
    assert_eq!(form.theme_candidate(), None);

    form.apply_input("theme_primary", "#0ea5e9".to_string());
    form.apply_input(KEY_THEME_FONT_SIZE, "20".to_string());
    let theme = form.theme_candidate().expect("valid hex");
    assert_eq!(theme.primary_hex, "#0ea5e9");
    assert_eq!(theme.font_size, Some(20.0));
    assert!(!theme.dark);

    let fallback = ThemeConfig::from_settings(&ThemeSettingsDto {
        dark_mode: true,
        primary_color: "nope".to_string(),
        secondary_color: "#12345".to_string(),
        font_size: 0,
    });
    let default = ThemeConfig::default();
    assert_eq!(fallback.primary_hex, default.primary_hex);
    assert_eq!(fallback.secondary_hex, default.secondary_hex);
    assert_eq!(fallback.font_size, None);
    assert!(fallback.dark);
}

#[test]
fn fixture_save_persists_the_whole_settings_dto() {
    let provider = FixtureProvider::new();
    let mut settings = provider.settings().expect("fixture settings");
    settings.vault_path = "/vaults/persisted".to_string();
    settings.theme.primary_color = "#22c55e".to_string();
    settings.ai.rag_chunk_count = 9;
    settings.sync.backup.enabled = true;
    settings.tray.capture_target = "inbox".to_string();

    let saved = provider.save_settings(settings.clone()).expect("save");
    assert_eq!(saved, settings);
    assert_eq!(provider.settings().expect("reload"), settings);
}

#[test]
fn settings_tabs_parse_from_env_and_have_stable_ids() {
    assert_eq!(SettingsTab::from_env("VAULT"), Some(SettingsTab::Vault));
    assert_eq!(SettingsTab::from_env("theme"), Some(SettingsTab::Theme));
    assert_eq!(SettingsTab::from_env("ai"), Some(SettingsTab::Ai));
    assert_eq!(
        SettingsTab::from_env("research"),
        Some(SettingsTab::Research)
    );
    assert_eq!(
        SettingsTab::from_env("developer"),
        Some(SettingsTab::Developer)
    );
    assert_eq!(SettingsTab::from_env("sync"), Some(SettingsTab::Sync));
    assert_eq!(SettingsTab::from_env("tray"), Some(SettingsTab::Tray));
    assert_eq!(
        SettingsTab::from_env("notifications"),
        Some(SettingsTab::Notifications)
    );
    assert_eq!(SettingsTab::from_env("nope"), None);

    let mut ids = HashSet::new();
    for tab in SettingsTab::ALL {
        assert!(ids.insert(tab.id()), "duplicate tab id {}", tab.id());
        assert_ne!(t(Locale::En, tab.label_key()), tab.label_key());
    }
}

#[test]
fn settings_keys_are_present_in_every_catalog_and_unique() {
    let mut seen = HashSet::new();
    for key in SETTINGS_I18N_KEYS {
        assert!(seen.insert(*key), "duplicate key: {key}");
        for locale in Locale::ALL {
            let value = t(locale, key);
            assert_ne!(value, *key, "{key} missing in {}", locale.code());
        }
    }
    for key in [
        "settings.validation.hex_color",
        "settings.validation.url",
        "settings.validation.required",
        "settings.validation.range",
        "settings.validation.commit_interval",
        "settings.validation.sync_interval",
        "settings.validation.positive",
        "settings.validation.commit_template",
        "settings.validation.save_blocked",
        "settings.validation.export_path",
    ] {
        assert!(
            SETTINGS_I18N_KEYS.contains(&key),
            "{key} is missing from the release gate list"
        );
    }
}
