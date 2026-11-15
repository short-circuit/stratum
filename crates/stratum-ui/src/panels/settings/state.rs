//! Read accessors and status mutators for the native Settings panel.
//!
//! The tab bodies and shell tests read the panel through these methods;
//! keeping them out of `mod.rs` leaves the entity and render wiring readable.

use gpui_kit::prelude::*;
use gpui_kit::Window;

use super::form::ExportKind;
use super::{
    BackupStatusDto, CommitLogEntry, DiscoveredEndpoint, ImportPreview, ImportReport,
    PresetOutcome, SettingsForm, SettingsTab, SettingsView, SttTestOutcome, SyncStatusDto,
    TtsTestOutcome,
};

impl SettingsView {
    /// The active tab.
    pub fn tab(&self) -> SettingsTab {
        self.tab
    }

    /// The editable form.
    pub fn form(&self) -> &SettingsForm {
        &self.form
    }

    /// The editable form, mutably.
    pub fn form_mut(&mut self) -> &mut SettingsForm {
        &mut self.form
    }

    /// The settings-load error, if any.
    pub fn load_error(&self) -> Option<&str> {
        self.load_error.as_deref()
    }

    /// The last status message, if any.
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    /// Whether the last status message is an error.
    pub fn message_is_error(&self) -> bool {
        self.message_is_error
    }

    /// The models reported by the last fetch.
    pub fn available_models(&self) -> &[String] {
        &self.available_models
    }

    /// The discovered local endpoints.
    pub fn discovered(&self) -> &[DiscoveredEndpoint] {
        &self.discovered
    }

    /// The local-discovery error, if any.
    pub fn discovery_error(&self) -> Option<&str> {
        self.discovery_error.as_deref()
    }

    /// The latest sync status, if loaded.
    pub fn sync_status(&self) -> Option<&SyncStatusDto> {
        self.sync_status.as_ref()
    }

    /// The loaded commit log.
    pub fn commits(&self) -> &[CommitLogEntry] {
        &self.commits
    }

    /// Whether the commit-log section is expanded.
    pub fn commits_open(&self) -> bool {
        self.commits_open
    }

    /// The latest backup status, if loaded.
    pub fn backup_status(&self) -> Option<&BackupStatusDto> {
        self.backup_status.as_ref()
    }

    /// The WebDAV test result, if any.
    pub fn webdav_result(&self) -> Option<&str> {
        self.webdav_result.as_deref()
    }

    /// The STT probe outcome, if any.
    pub fn stt_outcome(&self) -> Option<&SttTestOutcome> {
        self.stt_outcome.as_ref()
    }

    /// The TTS probe outcome, if any.
    pub fn tts_outcome(&self) -> Option<&TtsTestOutcome> {
        self.tts_outcome.as_ref()
    }

    /// The preset probe outcome for `url`, if any.
    pub fn preset_outcome(&self, url: &str) -> Option<&PresetOutcome> {
        self.preset_outcomes.get(url)
    }

    /// The last import preview, if any.
    pub fn import_preview(&self) -> Option<&ImportPreview> {
        self.import_preview.as_ref()
    }

    /// The last import report, if any.
    pub fn import_report(&self) -> Option<&ImportReport> {
        self.import_report.as_ref()
    }

    /// Whether an import preview/run is in flight.
    pub fn import_busy(&self) -> bool {
        self.import_busy
    }

    /// The preset currently being pinged, if any.
    pub fn preset_testing(&self) -> Option<&str> {
        self.preset_testing.as_deref()
    }

    /// The endpoint adopted from local discovery, if any.
    pub fn adopted_host(&self) -> Option<&str> {
        self.adopted_host.as_deref()
    }

    /// Whether the inline SSH passphrase prompt is visible.
    pub fn passphrase_prompt(&self) -> bool {
        self.passphrase_prompt
    }

    /// Whether a WebDAV connection test is in flight.
    pub fn webdav_testing(&self) -> bool {
        self.webdav_testing
    }

    /// Whether a backup run is in flight.
    pub fn backup_running(&self) -> bool {
        self.backup_running
    }

    /// The in-progress action label key, if any.
    pub fn busy(&self) -> Option<&'static str> {
        self.busy
    }

    /// The export row currently running, if any.
    pub fn busy_export(&self) -> Option<ExportKind> {
        self.busy_export
    }

    /// Switch tabs and recreate input entities for the new body.
    pub fn set_tab(&mut self, tab: SettingsTab, window: &mut Window, cx: &mut Context<Self>) {
        self.tab = tab;
        self.sync_inputs(window, cx);
        cx.notify();
    }

    /// Set a non-error status message.
    pub fn set_message(&mut self, message: impl Into<String>, cx: &mut Context<Self>) {
        self.message = Some(message.into());
        self.message_is_error = false;
        cx.notify();
    }

    /// Set an error status message.
    pub fn set_error(&mut self, message: impl Into<String>, cx: &mut Context<Self>) {
        self.message = Some(message.into());
        self.message_is_error = true;
        cx.notify();
    }

    /// Dismiss the status message.
    pub fn clear_message(&mut self, cx: &mut Context<Self>) {
        self.message = None;
        cx.notify();
    }
}
