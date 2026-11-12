//! Settings input entities and their synchronisation.
//!
//! Every editable field is a gpui [`InputState`] or [`TextareaState`] created
//! lazily and keyed by its form key. Change events write straight into
//! [`SettingsForm`]; programmatic transitions (reload, adopting a discovered
//! endpoint, choosing a preset) update the visible widget before the next paint
//! so the form and the screen never disagree.

use gpui_kit::component::input::{InputEvent, InputState, TextareaState};
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use super::form::{
    SettingsForm, KEY_AI_EMBEDDING_DIMENSIONS, KEY_AI_EXTRA_HOSTS, KEY_AI_RAG_CHUNKS,
    KEY_BACKUP_EXCLUDE, KEY_BACKUP_INTERVAL, KEY_RESEARCH_MAX_DEPTH, KEY_RESEARCH_MAX_RESULTS,
    KEY_SYNC_COMMIT_INTERVAL, KEY_SYNC_SYNC_INTERVAL, KEY_THEME_FONT_SIZE, KEY_TTS_SPEED,
};
use super::SettingsView;

/// One editable field's current value and rendering options.
struct FieldSpec {
    key: &'static str,
    value: String,
    masked: bool,
    textarea: bool,
    placeholder: Option<String>,
}

impl SettingsView {
    /// Create or refresh every input entity for the current form.
    pub(super) fn sync_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let specs = field_specs(&self.form);
        for spec in specs {
            if spec.textarea {
                self.sync_textarea(spec, window, cx);
            } else {
                self.sync_input(spec, window, cx);
            }
        }
    }

    /// The input entity for `key`, if it has been created.
    pub fn input(&self, key: &'static str) -> Option<&Entity<InputState>> {
        self.inputs.get(key)
    }

    /// The textarea entity for `key`, if it has been created.
    pub fn textarea(&self, key: &'static str) -> Option<&Entity<TextareaState>> {
        self.textareas.get(key)
    }

    /// Create or update one single-line input.
    fn sync_input(&mut self, spec: FieldSpec, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(input) = self.inputs.get(spec.key).cloned() {
            let current = input.read(cx).value().to_string();
            if current != spec.value {
                input.update(cx, |input, cx| input.set_value(spec.value, window, cx));
            }
            return;
        }
        let key = spec.key;
        let value = spec.value;
        let masked = spec.masked;
        let placeholder = spec.placeholder.unwrap_or_default();
        let window_ref = &mut *window;
        let input = cx.new(move |cx| {
            let state = InputState::new(window_ref, cx).default_value(value);
            let state = if masked { state.masked(true) } else { state };
            if placeholder.is_empty() {
                state
            } else {
                state.placeholder(placeholder)
            }
        });
        let subscription = cx.subscribe(
            &input,
            move |this: &mut Self, input, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    let value = input.read(cx).value().to_string();
                    this.form.apply_input(key, value);
                    this.sync_theme_from_form(cx);
                    cx.notify();
                }
            },
        );
        self.inputs.insert(key, input);
        self.input_subscriptions.push(subscription);
    }

    /// Create or update one multiline textarea.
    fn sync_textarea(&mut self, spec: FieldSpec, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(textarea) = self.textareas.get(spec.key).cloned() {
            let current = textarea.read(cx).value().to_string();
            if current != spec.value {
                textarea.update(cx, |textarea, cx| {
                    textarea.set_value(spec.value, window, cx);
                });
            }
            return;
        }
        let key = spec.key;
        let value = spec.value;
        let window_ref = &mut *window;
        let textarea = cx.new(move |cx| TextareaState::new(window_ref, cx).default_value(value));
        let subscription = cx.subscribe(
            &textarea,
            move |this: &mut Self, textarea, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    let value = textarea.read(cx).value().to_string();
                    this.form.apply_input(key, value);
                    cx.notify();
                }
            },
        );
        self.textareas.insert(key, textarea);
        self.input_subscriptions.push(subscription);
    }
}

/// Build the current value for every editable field.
fn field_specs(form: &SettingsForm) -> Vec<FieldSpec> {
    let settings = &form.settings;
    let raw = |key: &'static str| form.raw.get(key).cloned().unwrap_or_default();
    let masked_placeholder = || Some(t_global("settings.key_saved_enter_new_value_to_change"));
    vec![
        field("vault_path", settings.vault_path.clone()),
        field("theme_primary", settings.theme.primary_color.clone()),
        field("theme_secondary", settings.theme.secondary_color.clone()),
        field("theme_font_size", raw(KEY_THEME_FONT_SIZE)),
        field_with(
            "ai_endpoint",
            settings.ai.endpoint.clone().unwrap_or_default(),
            Some("http://localhost:11434".to_string()),
        ),
        masked(
            "ai_api_key",
            settings.ai.api_key.clone().unwrap_or_default(),
            masked_placeholder(),
        ),
        field_with(
            "ai_model",
            settings.ai.model.clone(),
            Some("gpt-4o".to_string()),
        ),
        field("ai_rag_chunk_count", raw(KEY_AI_RAG_CHUNKS)),
        field("ai_embedding_dimensions", raw(KEY_AI_EMBEDDING_DIMENSIONS)),
        textarea("ai_extra_local_hosts", raw(KEY_AI_EXTRA_HOSTS)),
        field_with(
            "research_endpoint",
            settings.research.searxng_endpoint.clone(),
            Some("http://localhost:8888".to_string()),
        ),
        field("research_max_results", raw(KEY_RESEARCH_MAX_RESULTS)),
        field("research_max_depth", raw(KEY_RESEARCH_MAX_DEPTH)),
        field_with(
            "sync_remote_url",
            settings.sync.remote_url.clone().unwrap_or_default(),
            Some("git@github.com:user/vault.git".to_string()),
        ),
        field_with(
            "sync_branch",
            settings.sync.branch.clone(),
            Some("main".to_string()),
        ),
        field("sync_commit_interval", raw(KEY_SYNC_COMMIT_INTERVAL)),
        field("sync_sync_interval", raw(KEY_SYNC_SYNC_INTERVAL)),
        field_with(
            "sync_ssh_key_path",
            settings.sync.ssh_key_path.clone().unwrap_or_default(),
            Some("~/.ssh/id_ed25519".to_string()),
        ),
        field(
            "sync_commit_template",
            settings.sync.commit_template.clone(),
        ),
        masked(
            "passphrase",
            form.passphrase.clone(),
            Some(t_global("common.ssh_key_passphrase")),
        ),
        field(
            "backup_folder_path",
            settings.sync.backup.folder_path.clone().unwrap_or_default(),
        ),
        field_with(
            "backup_webdav_url",
            settings.sync.backup.webdav_url.clone().unwrap_or_default(),
            Some("https://dav.example.com/backup".to_string()),
        ),
        field(
            "backup_webdav_username",
            settings
                .sync
                .backup
                .webdav_username
                .clone()
                .unwrap_or_default(),
        ),
        masked(
            "backup_webdav_password",
            settings
                .sync
                .backup
                .webdav_password
                .clone()
                .unwrap_or_default(),
            masked_placeholder(),
        ),
        field("backup_interval", raw(KEY_BACKUP_INTERVAL)),
        textarea("backup_exclude", raw(KEY_BACKUP_EXCLUDE)),
        field_with(
            "stt_endpoint",
            settings.stt.endpoint.clone(),
            Some("http://localhost:8081".to_string()),
        ),
        masked(
            "stt_api_key",
            settings.stt.api_key.clone().unwrap_or_default(),
            masked_placeholder(),
        ),
        field("stt_model", settings.stt.model.clone()),
        field("stt_diarize_model", settings.stt.diarize_model.clone()),
        field_with(
            "stt_language",
            settings.stt.language.clone().unwrap_or_default(),
            Some("en".to_string()),
        ),
        field("tts_endpoint", settings.tts.endpoint.clone()),
        masked(
            "tts_api_key",
            settings.tts.api_key.clone().unwrap_or_default(),
            masked_placeholder(),
        ),
        field_with(
            "tts_voice",
            settings.tts.voice.clone(),
            Some("alloy".to_string()),
        ),
        field_with(
            "tts_format",
            settings.tts.format.clone(),
            Some("mp3".to_string()),
        ),
        field("tts_speed", raw(KEY_TTS_SPEED)),
        field(
            "tray_shortcut",
            settings.tray.quick_capture_shortcut.clone(),
        ),
        field("tray_inbox_path", settings.tray.inbox_path.clone()),
        field("voice_target", settings.voice.target.clone()),
        field(
            "export_html",
            form.exports
                .get(&super::ExportKind::Html)
                .cloned()
                .unwrap_or_default(),
        ),
        field(
            "export_json",
            form.exports
                .get(&super::ExportKind::Json)
                .cloned()
                .unwrap_or_default(),
        ),
        field(
            "export_pdf",
            form.exports
                .get(&super::ExportKind::Pdf)
                .cloned()
                .unwrap_or_default(),
        ),
        field(
            "export_zip",
            form.exports
                .get(&super::ExportKind::Zip)
                .cloned()
                .unwrap_or_default(),
        ),
        field("import_path", form.import_path.clone()),
    ]
}

fn field(key: &'static str, value: String) -> FieldSpec {
    field_with(key, value, None)
}

fn field_with(key: &'static str, value: String, placeholder: Option<String>) -> FieldSpec {
    FieldSpec {
        key,
        value,
        masked: false,
        textarea: false,
        placeholder,
    }
}

fn masked(key: &'static str, value: String, placeholder: Option<String>) -> FieldSpec {
    FieldSpec {
        key,
        value,
        masked: true,
        textarea: false,
        placeholder,
    }
}

fn textarea(key: &'static str, value: String) -> FieldSpec {
    FieldSpec {
        key,
        value,
        masked: false,
        textarea: true,
        placeholder: None,
    }
}
