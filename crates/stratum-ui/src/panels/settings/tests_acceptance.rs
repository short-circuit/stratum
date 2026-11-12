//! Acceptance tests for the native Settings surface.
//!
//! The first test edits every editable field through the form registry, saves,
//! reloads and round-trips the DTO again. The second drives the fixture
//! conflict-resolution seam (Accept and Abort Merge). No GPUI window is opened.

use stratum_app::settings::AiModelDto;

use super::form::{
    KEY_AI_EMBEDDING_DIMENSIONS, KEY_AI_EXTRA_HOSTS, KEY_AI_RAG_CHUNKS, KEY_BACKUP_EXCLUDE,
    KEY_BACKUP_INTERVAL, KEY_RESEARCH_MAX_DEPTH, KEY_RESEARCH_MAX_RESULTS,
    KEY_SYNC_COMMIT_INTERVAL, KEY_SYNC_SYNC_INTERVAL, KEY_THEME_FONT_SIZE, KEY_TTS_SPEED,
};
use super::validate::validate;
use super::SettingsForm;
use crate::provider::{FixtureProvider, SettingsData};

/// A DTO that exercises every editable field with a non-default value.
#[test]
fn settings_change_save_reload_persists_every_field() {
    let provider = FixtureProvider::new();
    let mut form = SettingsForm::from_settings(provider.settings().expect("fixture settings"));

    // Vault and layout.
    form.apply_input("vault_path", "/vaults/acceptance".to_string());
    form.settings.layout.pages_view = "tree".to_string();

    // Theme and locale.
    form.settings.locale = "de".to_string();
    form.settings.theme.dark_mode = false;
    form.apply_input("theme_primary", "#0ea5e9".to_string());
    form.apply_input("theme_secondary", "#111827".to_string());
    form.apply_input(KEY_THEME_FONT_SIZE, "20".to_string());

    // AI provider, models, RAG, gateway and extra hosts.
    form.settings.ai.provider = "custom".to_string();
    form.apply_input("ai_endpoint", "https://ai.example.test/v1".to_string());
    form.apply_input("ai_api_key", "sk-acceptance".to_string());
    form.apply_input("ai_model", "gpt-acceptance".to_string());
    form.settings.ai.models = vec![AiModelDto {
        name: "gpt-acceptance".to_string(),
        capabilities: vec!["chat".to_string(), "embedding".to_string()],
    }];
    form.settings.ai.rag_enabled = false;
    form.apply_input(KEY_AI_RAG_CHUNKS, "7".to_string());
    form.apply_input(KEY_AI_EMBEDDING_DIMENSIONS, "1024".to_string());
    form.settings.ai.use_llm_gateway_and_auth = true;
    form.apply_input(KEY_AI_EXTRA_HOSTS, "http://127.0.0.1:4321".to_string());

    // Research.
    form.apply_input(
        "research_endpoint",
        "https://searx.example.test".to_string(),
    );
    form.apply_input(KEY_RESEARCH_MAX_RESULTS, "5".to_string());
    form.apply_input(KEY_RESEARCH_MAX_DEPTH, "4".to_string());

    // Sync, SSH and backup.
    form.settings.sync.mode = "auto_sync".to_string();
    form.apply_input(
        "sync_remote_url",
        "https://example.test/vault.git".to_string(),
    );
    form.apply_input("sync_branch", "dev".to_string());
    form.apply_input(KEY_SYNC_COMMIT_INTERVAL, "120".to_string());
    form.apply_input(KEY_SYNC_SYNC_INTERVAL, "900".to_string());
    form.apply_input("sync_ssh_key_path", "/keys/id_ed25519".to_string());
    form.apply_input(
        "sync_commit_template",
        "sync({datetime}): {editedfiles} edited, {newfiles} added, {deletedfiles} deleted, {count} total"
            .to_string(),
    );
    form.settings.sync.backup.enabled = true;
    form.settings.sync.backup.mode = "webdav".to_string();
    form.apply_input(KEY_BACKUP_INTERVAL, "15".to_string());
    form.apply_input(KEY_BACKUP_EXCLUDE, "tmp, cache".to_string());
    form.apply_input(
        "backup_webdav_url",
        "https://dav.example.test/vault".to_string(),
    );
    form.apply_input("backup_webdav_username", "alice".to_string());
    form.apply_input("backup_webdav_password", "secret".to_string());
    form.apply_input("backup_folder_path", "/backups".to_string());

    // Speech-to-text.
    form.apply_input("stt_endpoint", "https://stt.example.test".to_string());
    form.apply_input("stt_api_key", "stt-key".to_string());
    form.apply_input("stt_model", "whisper-acceptance".to_string());
    form.apply_input("stt_diarize_model", "pyannote-acceptance".to_string());
    form.apply_input("stt_language", "en".to_string());
    form.settings.stt.diarize = false;
    form.settings.stt.auto_summarize = false;
    form.settings.stt.auto_identify = true;
    form.settings.stt.use_llm_gateway_and_auth = true;

    // Text-to-speech.
    form.apply_input("tts_endpoint", "https://tts.example.test".to_string());
    form.apply_input("tts_api_key", "tts-key".to_string());
    form.apply_input("tts_voice", "nova".to_string());
    form.apply_input("tts_format", "wav".to_string());
    form.apply_input(KEY_TTS_SPEED, "1.25".to_string());
    form.settings.tts.use_llm_gateway_and_auth = true;

    // Tray.
    form.settings.tray.enabled = false;
    form.settings.tray.close_to_tray = false;
    form.apply_input("tray_shortcut", "Ctrl+Shift+K".to_string());
    form.settings.tray.capture_target = "inbox".to_string();
    form.apply_input("tray_inbox_path", "pages/Inbox.md".to_string());

    // Notifications.
    form.settings.reminders.enabled = false;
    form.settings.reminders.sound = false;

    // Voice capture.
    form.settings.voice.enabled = false;
    form.apply_input("voice_target", "journal:yesterday".to_string());

    let candidate = form.candidate().expect("acceptance candidate parses");
    let errors = validate(&candidate);
    assert!(
        errors.is_empty(),
        "unexpected validation errors: {errors:?}"
    );

    let saved = provider
        .save_settings(candidate.clone())
        .expect("save settings");
    assert_eq!(saved, candidate);

    let reloaded = provider.settings().expect("reload settings");
    assert_eq!(reloaded, candidate);

    let round_trip = SettingsForm::from_settings(reloaded);
    assert_eq!(
        round_trip.candidate().expect("round-trip candidate parses"),
        candidate
    );
}

#[test]
fn fixture_resolves_and_aborts_seeded_conflicts() {
    let provider = FixtureProvider::with_conflicts(vec![
        "pages/Conflict One.md".to_string(),
        "pages/Conflict Two.md".to_string(),
    ]);
    let status = provider.sync_status().expect("sync status");
    assert_eq!(status.status, "conflicts");
    assert_eq!(status.conflicts.len(), 2);
    assert_eq!(status.ahead, 1);
    assert_eq!(status.behind, 2);

    provider
        .resolve_conflict("pages/Conflict One.md")
        .expect("resolve one");
    let status = provider.sync_status().expect("sync status");
    assert_eq!(status.status, "conflicts");
    assert_eq!(status.conflicts, vec!["pages/Conflict Two.md".to_string()]);

    provider.abort_merge().expect("abort merge");
    let status = provider.sync_status().expect("sync status");
    assert_eq!(status.status, "ok");
    assert!(status.conflicts.is_empty());
    assert_eq!(status.behind, 0);
}
