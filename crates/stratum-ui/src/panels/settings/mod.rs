//! Native Settings panel.
//!
//! [`SettingsView`] ports the shipped eight-tab desktop `SettingsPage` onto the
//! gpui shell: vault/import/export, theme (with live palette application), AI,
//! research, developer tools, sync/backup, tray and notifications. Every field
//! is persisted through the [`SettingsData`] provider seam; the shell only owns
//! presentation and the pending navigation/toast requests it drains.
//!
//! Form state and validation live in [`form`] and [`validate`]; input entities
//! and their subscriptions in [`inputs`]; provider calls in [`actions`];
//! reusable styled widgets in [`controls`]; and the tab bodies in [`tabs`].
//!
//! [`SETTINGS_I18N_KEYS`] is the release gate list for every string this
//! surface renders.

use std::collections::BTreeMap;
use std::sync::Arc;

use gpui_kit::component::input::{InputState, TextareaState};
use gpui_kit::component::ActiveTheme;
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use crate::panels::PanelRequests;
pub use crate::provider::{
    AiModelDto, BackupStatusDto, CommitLogEntry, DiscoveredEndpoint, ImportPreview, ImportReport,
    ShellData, SttTestOutcome, SyncStatusDto, TtsTestOutcome,
};

pub mod actions;
pub mod actions_ai;
pub mod actions_ops;
pub mod banners;
pub mod controls;
pub mod form;
pub mod inputs;
pub mod keys;
pub mod raw;
pub mod state;
pub mod tabs;
pub mod validate;

pub use form::{ExportKind, SettingsForm};
pub use keys::SETTINGS_I18N_KEYS;

/// The eight shipped settings tabs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SettingsTab {
    /// Vault path, page-list layout, import and export.
    #[default]
    Vault,
    /// Theme, locale and live palette.
    Theme,
    /// AI provider, models, RAG, STT, TTS and voice capture.
    Ai,
    /// SearXNG research settings and presets.
    Research,
    /// Reindex/repair/normalize, onboarding reset and starter templates.
    Developer,
    /// Git sync, commit log and cloud/folder backup.
    Sync,
    /// System tray and quick capture.
    Tray,
    /// Block reminder notifications.
    Notifications,
}

impl SettingsTab {
    /// Every tab, in shipped order.
    pub const ALL: [Self; 8] = [
        Self::Vault,
        Self::Theme,
        Self::Ai,
        Self::Research,
        Self::Developer,
        Self::Sync,
        Self::Tray,
        Self::Notifications,
    ];

    /// Parse a `STRATUM_SETTINGS_TAB` value.
    pub fn from_env(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "vault" => Some(Self::Vault),
            "theme" => Some(Self::Theme),
            "ai" => Some(Self::Ai),
            "research" => Some(Self::Research),
            "developer" => Some(Self::Developer),
            "sync" => Some(Self::Sync),
            "tray" => Some(Self::Tray),
            "notifications" => Some(Self::Notifications),
            _ => None,
        }
    }

    /// Stable identifier used by capture debug selectors.
    pub fn id(self) -> &'static str {
        match self {
            Self::Vault => "vault",
            Self::Theme => "theme",
            Self::Ai => "ai",
            Self::Research => "research",
            Self::Developer => "developer",
            Self::Sync => "sync",
            Self::Tray => "tray",
            Self::Notifications => "notifications",
        }
    }

    /// The catalog key for this tab label.
    pub fn label_key(self) -> &'static str {
        match self {
            Self::Vault => "settings.vault",
            Self::Theme => "settings.theme",
            Self::Ai => "settings.ai",
            Self::Research => "settings.research",
            Self::Developer => "settings.developer",
            Self::Sync => "settings.sync",
            Self::Tray => "settings.tray",
            Self::Notifications => "settings.notifications.title",
        }
    }
}

/// Deterministic start state for the Settings panel.
///
/// `STRATUM_SETTINGS_TAB=vault|theme|ai|research|developer|sync|tray|notifications`
/// selects the tab during construction so the capture harness can render each
/// surface without a click.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SettingsStart {
    /// Tab to open after construction.
    pub tab: Option<SettingsTab>,
}

/// One inline SearXNG preset probe outcome.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PresetOutcome {
    /// Whether the instance answered.
    pub ok: bool,
    /// Localized detail line.
    pub detail: String,
}

/// The settings panel entity.
pub struct SettingsView {
    provider: Arc<dyn ShellData>,
    tab: SettingsTab,
    form: SettingsForm,
    load_error: Option<String>,
    message: Option<String>,
    message_is_error: bool,
    inputs: BTreeMap<&'static str, Entity<InputState>>,
    textareas: BTreeMap<&'static str, Entity<TextareaState>>,
    input_subscriptions: Vec<Subscription>,
    available_models: Vec<String>,
    discovered: Vec<DiscoveredEndpoint>,
    discovery_error: Option<String>,
    sync_status: Option<SyncStatusDto>,
    commits: Vec<CommitLogEntry>,
    commits_open: bool,
    backup_status: Option<BackupStatusDto>,
    webdav_result: Option<String>,
    stt_outcome: Option<SttTestOutcome>,
    tts_outcome: Option<TtsTestOutcome>,
    preset_outcomes: BTreeMap<String, PresetOutcome>,
    preset_testing: Option<String>,
    adopted_host: Option<String>,
    import_preview: Option<ImportPreview>,
    import_report: Option<ImportReport>,
    import_busy: bool,
    busy: Option<&'static str>,
    busy_export: Option<ExportKind>,
    passphrase_prompt: bool,
    webdav_testing: bool,
    backup_running: bool,
    pdf_page: Option<String>,
    pending_navigation: Option<String>,
    pending_toast: Option<String>,
}

impl SettingsView {
    /// Build the panel, load the effective settings and refresh the sync/backup
    /// status the shipped page fetches on mount.
    pub fn new(
        provider: Arc<dyn ShellData>,
        start: SettingsStart,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let (form, load_error) = match provider.settings() {
            Ok(settings) => (SettingsForm::from_settings(settings), None),
            Err(error) => (
                SettingsForm::fallback("."),
                Some(format!("{} {error}", t_global("settings.load_failed"))),
            ),
        };
        let sync_status = provider.sync_status().ok();
        let commits = provider.commit_log().unwrap_or_default();
        let backup_status = provider.backup_status().ok();
        let theme = form.last_valid_theme.clone();
        crate::theme::apply_to_kit(cx, &theme);
        let mut view = Self {
            provider,
            tab: start.tab.unwrap_or_default(),
            form,
            load_error,
            message: None,
            message_is_error: false,
            inputs: BTreeMap::new(),
            textareas: BTreeMap::new(),
            input_subscriptions: Vec::new(),
            available_models: Vec::new(),
            discovered: Vec::new(),
            discovery_error: None,
            sync_status,
            commits,
            commits_open: false,
            backup_status,
            webdav_result: None,
            stt_outcome: None,
            tts_outcome: None,
            preset_outcomes: BTreeMap::new(),
            preset_testing: None,
            adopted_host: None,
            import_preview: None,
            import_report: None,
            import_busy: false,
            busy: None,
            busy_export: None,
            passphrase_prompt: false,
            webdav_testing: false,
            backup_running: false,
            pdf_page: None,
            pending_navigation: None,
            pending_toast: None,
        };
        view.sync_inputs(window, cx);
        view
    }
}

impl PanelRequests for SettingsView {
    fn pending_navigation(&self) -> Option<&str> {
        self.pending_navigation.as_deref()
    }

    fn clear_pending_navigation(&mut self) {
        self.pending_navigation = None;
    }

    fn pending_toast(&self) -> Option<&str> {
        self.pending_toast.as_deref()
    }

    fn take_pending_toast(&mut self) -> Option<String> {
        self.pending_toast.take()
    }
}

impl Render for SettingsView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_inputs(window, cx);
        let palette = controls::palette(cx);
        let theme = cx.theme();
        div()
            .relative()
            .flex()
            .flex_col()
            .size_full()
            .bg(theme.colors.background)
            .text_color(theme.colors.foreground)
            .child(controls::header(self, &palette, cx))
            .child(
                div()
                    .id("settings-body")
                    .debug_selector(|| "settings-body".to_string())
                    .flex()
                    .flex_col()
                    .flex_1()
                    .overflow_y_scroll()
                    .p(px(24.0))
                    .gap(px(12.0))
                    .child(controls::tab_bar(self, &palette, cx))
                    .when_some(self.message.clone(), |element, message| {
                        element.child(controls::message_banner(
                            message,
                            self.message_is_error,
                            &palette,
                            cx,
                        ))
                    })
                    .when_some(self.load_error.clone(), |element, message| {
                        element.child(controls::error_banner(message, &palette))
                    })
                    .child(tabs::render(self, &palette, cx)),
            )
    }
}

#[cfg(test)]
mod tests;
