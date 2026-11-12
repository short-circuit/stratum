//! Provider actions for the native Settings panel.
//!
//! Every mutation, probe and export lives here so the tab bodies stay
//! declarative. Provider calls are synchronous because the [`SettingsData`]
//! seam blocks on the service runtime; only the platform path pickers run off
//! the UI thread.
//!
//! [`SettingsData`]: crate::provider::SettingsData

use std::path::PathBuf;
use std::sync::Arc;

use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_app::import::ImportOptions;
use stratum_i18n::{set_locale, t_global, tr_global, Locale};

use super::form::{ExportKind, KEY_THEME_FONT_SIZE};
use super::{SettingsTab, SettingsView};
use crate::theme::{apply_to_kit, ThemeConfig};

impl SettingsView {
    /// Repaint the shell from the current theme edits.
    ///
    /// Invalid colours keep the last palette that passed validation, so typing
    /// a partial hex value never flashes a fallback theme.
    pub fn sync_theme_from_form(&mut self, cx: &mut Context<Self>) {
        if let Some(config) = self.form.theme_candidate() {
            self.form.last_valid_theme = config.clone();
            apply_to_kit(cx, &config);
        }
    }

    /// Validate and persist the form, then apply the stored theme.
    pub fn save(&mut self, cx: &mut Context<Self>) {
        self.busy = Some("save");
        self.message = None;
        let candidate = match self.form.candidate() {
            Ok(candidate) => candidate,
            Err(errors) => {
                self.form.errors = errors;
                self.busy = None;
                self.set_error(t_global("settings.validation.save_blocked"), cx);
                return;
            }
        };
        let errors = super::validate::validate(&candidate);
        if !errors.is_empty() {
            self.form.errors = errors;
            self.busy = None;
            self.set_error(t_global("settings.validation.save_blocked"), cx);
            return;
        }
        match self.provider.save_settings(candidate) {
            Ok(saved) => {
                self.form = super::SettingsForm::from_settings(saved);
                let config = ThemeConfig::from_settings(&self.form.settings.theme);
                self.form.last_valid_theme = config.clone();
                apply_to_kit(cx, &config);
                self.passphrase_prompt = false;
                self.busy = None;
                self.set_message(t_global("settings.saved"), cx);
            }
            Err(error) => {
                self.busy = None;
                self.set_error(format!("{} {error}", t_global("settings.save_failed")), cx);
            }
        }
    }

    /// Reload the effective settings from the provider.
    pub fn reload(&mut self, cx: &mut Context<Self>) {
        self.load_error = None;
        match self.provider.settings() {
            Ok(settings) => {
                self.form = super::SettingsForm::from_settings(settings);
                let config = ThemeConfig::from_settings(&self.form.settings.theme);
                self.form.last_valid_theme = config.clone();
                apply_to_kit(cx, &config);
            }
            Err(error) => {
                self.load_error = Some(format!("{} {error}", t_global("settings.load_failed")));
            }
        }
        cx.notify();
    }

    /// Open the platform folder picker for the vault path.
    pub fn browse_vault(&mut self, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some(t_global("settings.vault_path").into()),
        });
        cx.spawn(
            async move |this: WeakEntity<SettingsView>, cx: &mut AsyncApp| {
                if let Ok(Ok(Some(paths))) = receiver.await {
                    if let Some(path) = paths.into_iter().next() {
                        let value = path.to_string_lossy().to_string();
                        let _ = this.update(cx, |view, cx| {
                            view.form.apply_input("vault_path", value);
                            cx.notify();
                        });
                    }
                }
            },
        )
        .detach();
    }

    /// Open the platform picker for an export output path.
    pub fn browse_export(&mut self, kind: ExportKind, cx: &mut Context<Self>) {
        let current = self.form.exports.get(&kind).cloned().unwrap_or_default();
        if kind.is_folder() {
            let receiver = cx.prompt_for_paths(PathPromptOptions {
                files: false,
                directories: true,
                multiple: false,
                prompt: Some(t_global("export.output_path").into()),
            });
            cx.spawn(
                async move |this: WeakEntity<SettingsView>, cx: &mut AsyncApp| {
                    if let Ok(Ok(Some(paths))) = receiver.await {
                        if let Some(path) = paths.into_iter().next() {
                            let value = path.to_string_lossy().to_string();
                            let _ = this.update(cx, |view, cx| {
                                view.form.apply_input(kind.input_key(), value);
                                cx.notify();
                            });
                        }
                    }
                },
            )
            .detach();
            return;
        }
        let path = PathBuf::from(&current);
        let directory = path
            .parent()
            .map(|parent| parent.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().to_string());
        let receiver = cx.prompt_for_new_path(&directory, name.as_deref());
        cx.spawn(
            async move |this: WeakEntity<SettingsView>, cx: &mut AsyncApp| {
                if let Ok(Ok(Some(path))) = receiver.await {
                    let value = path.to_string_lossy().to_string();
                    let _ = this.update(cx, |view, cx| {
                        view.form.apply_input(kind.input_key(), value);
                        cx.notify();
                    });
                }
            },
        )
        .detach();
    }

    /// Open the platform folder picker for the import source.
    pub fn browse_import(&mut self, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some(t_global("importers.select_source_folder").into()),
        });
        cx.spawn(
            async move |this: WeakEntity<SettingsView>, cx: &mut AsyncApp| {
                if let Ok(Ok(Some(paths))) = receiver.await {
                    if let Some(path) = paths.into_iter().next() {
                        let value = path.to_string_lossy().to_string();
                        let _ = this.update(cx, |view, cx| {
                            view.form.apply_input("import_path", value);
                            view.import_preview = None;
                            view.import_report = None;
                            cx.notify();
                        });
                    }
                }
            },
        )
        .detach();
    }

    /// Preview the import source without writing anything.
    pub fn preview_import(&mut self, cx: &mut Context<Self>) {
        let path = self.form.import_path.trim().to_string();
        if path.is_empty() {
            self.set_error(t_global("importers.no_selection"), cx);
            return;
        }
        self.import_busy = true;
        self.import_preview = None;
        self.import_report = None;
        let options = ImportOptions {
            format: self.form.import_format,
            dry_run: true,
            ..ImportOptions::default()
        };
        match self.provider.preview_import(&path, options) {
            Ok(preview) => self.import_preview = Some(preview),
            Err(error) => self.set_error(
                format!("{} {error}", t_global("importers.detect_failed")),
                cx,
            ),
        }
        self.import_busy = false;
        cx.notify();
    }

    /// Run the import (or a dry run when the dry-run flag is set).
    pub fn run_import(&mut self, cx: &mut Context<Self>) {
        let path = self.form.import_path.trim().to_string();
        if path.is_empty() {
            self.set_error(t_global("importers.no_selection"), cx);
            return;
        }
        self.import_busy = true;
        self.import_report = None;
        let options = ImportOptions {
            format: self.form.import_format,
            dry_run: self.form.import_dry_run,
            ..ImportOptions::default()
        };
        match self.provider.run_import(&path, options) {
            Ok(report) => {
                let summary = tr_global(
                    "importers.report_summary",
                    &[
                        ("pages", report.pages_imported.to_string().as_str()),
                        ("assets", report.assets_imported.to_string().as_str()),
                        ("duplicates", report.duplicates_skipped.to_string().as_str()),
                        ("conflicts", report.conflicts_skipped.to_string().as_str()),
                    ],
                );
                self.import_report = Some(report);
                self.import_busy = false;
                self.set_message(summary, cx);
            }
            Err(error) => {
                self.import_busy = false;
                self.set_error(
                    format!("{} {error}", t_global("importers.detect_failed")),
                    cx,
                );
            }
        }
    }

    /// Clear the import preview/report.
    pub fn clear_import(&mut self, cx: &mut Context<Self>) {
        self.import_preview = None;
        self.import_report = None;
        cx.notify();
    }

    /// Run one export row against the current path.
    pub fn run_export(&mut self, kind: ExportKind, cx: &mut Context<Self>) {
        let path = self.form.exports.get(&kind).cloned().unwrap_or_default();
        if path.trim().is_empty() {
            self.set_error(t_global("settings.validation.export_path"), cx);
            return;
        }
        if kind == ExportKind::Pdf && self.pdf_page.is_none() {
            self.set_error(t_global("export.no_current_page"), cx);
            return;
        }
        self.busy_export = Some(kind);
        let result = match kind {
            ExportKind::Html => self.provider.export_html_to(&path),
            ExportKind::Json => self.provider.export_json_to(&path),
            ExportKind::Pdf => self
                .provider
                .export_pdf_to(self.pdf_page.as_deref().unwrap_or_default(), &path),
            ExportKind::Zip => self.provider.export_zip_to(&path),
        };
        self.busy_export = None;
        match result {
            Ok(outcome) => {
                let message = match kind {
                    ExportKind::Html => export_message(
                        "export.success_html",
                        outcome.count,
                        outcome.pages,
                        &outcome.output_path,
                    ),
                    ExportKind::Json => export_message(
                        "export.success_json",
                        outcome.count,
                        outcome.pages,
                        &outcome.output_path,
                    ),
                    ExportKind::Pdf => tr_global(
                        "export.success_pdf",
                        &[("path", outcome.output_path.as_str())],
                    ),
                    ExportKind::Zip => export_message(
                        "export.success_zip",
                        outcome.count,
                        outcome.pages,
                        &outcome.output_path,
                    ),
                };
                self.set_message(message, cx);
            }
            Err(error) => {
                self.set_error(format!("{} {error}", t_global("export.failed")), cx);
            }
        }
    }

    /// Apply the locale immediately and persist it best-effort.
    pub fn change_locale(&mut self, code: &str, cx: &mut Context<Self>) {
        let Some(locale) = Locale::from_code(code) else {
            return;
        };
        set_locale(locale);
        self.form.settings.locale = code.to_string();
        let mut candidate = self.form.settings.clone();
        candidate.locale = code.to_string();
        if let Ok(saved) = self.provider.save_settings(candidate) {
            self.form.settings.locale = saved.locale;
        }
        cx.notify();
    }

    /// Set a boolean field from a toggle.
    pub fn set_flag(&mut self, field: &'static str, value: bool, cx: &mut Context<Self>) {
        self.form.errors.clear();
        match field {
            "dark_mode" => self.form.settings.theme.dark_mode = value,
            "rag_enabled" => self.form.settings.ai.rag_enabled = value,
            "ai_gateway" => self.form.settings.ai.use_llm_gateway_and_auth = value,
            "stt_diarize" => self.form.settings.stt.diarize = value,
            "stt_auto_summarize" => self.form.settings.stt.auto_summarize = value,
            "stt_auto_identify" => self.form.settings.stt.auto_identify = value,
            "stt_gateway" => self.form.settings.stt.use_llm_gateway_and_auth = value,
            "tts_gateway" => self.form.settings.tts.use_llm_gateway_and_auth = value,
            "voice_enabled" => self.form.settings.voice.enabled = value,
            "tray_enabled" => self.form.settings.tray.enabled = value,
            "tray_close_to_tray" => self.form.settings.tray.close_to_tray = value,
            "reminders_enabled" => self.form.settings.reminders.enabled = value,
            "reminders_sound" => self.form.settings.reminders.sound = value,
            "backup_enabled" => self.form.settings.sync.backup.enabled = value,
            "import_dry_run" => self.form.import_dry_run = value,
            _ => {}
        }
        if field == "dark_mode" {
            self.sync_theme_from_form(cx);
        }
        cx.notify();
    }

    /// Set an enumerated field from a chip row.
    pub fn set_choice(&mut self, field: &'static str, value: &str, cx: &mut Context<Self>) {
        self.form.errors.clear();
        match field {
            "pages_view" => self.form.settings.layout.pages_view = value.to_string(),
            "ai_provider" => self.form.settings.ai.provider = value.to_string(),
            "sync_mode" => self.form.settings.sync.mode = value.to_string(),
            "backup_mode" => self.form.settings.sync.backup.mode = value.to_string(),
            "tray_capture_target" => self.form.settings.tray.capture_target = value.to_string(),
            "voice_target" => self.form.settings.voice.target = value.to_string(),
            "import_format" => {
                self.form.import_format = match value {
                    "obsidian" => stratum_app::import::ImportFormat::Obsidian,
                    "logseq" => stratum_app::import::ImportFormat::Logseq,
                    "generic" => stratum_app::import::ImportFormat::Generic,
                    _ => stratum_app::import::ImportFormat::Auto,
                }
            }
            _ => {}
        }
        cx.notify();
    }

    /// Set a theme colour from a preset swatch.
    pub fn set_theme_color(&mut self, field: &'static str, value: &str, cx: &mut Context<Self>) {
        self.form.errors.clear();
        match field {
            "theme_primary" => self.form.settings.theme.primary_color = value.to_string(),
            "theme_secondary" => self.form.settings.theme.secondary_color = value.to_string(),
            _ => {}
        }
        self.sync_theme_from_form(cx);
        cx.notify();
    }

    /// Append a commit-template placeholder to the template field.
    pub fn insert_template_placeholder(&mut self, placeholder: &str, cx: &mut Context<Self>) {
        self.form
            .settings
            .sync
            .commit_template
            .push_str(placeholder);
        cx.notify();
    }

    /// The current raw font size (used by the theme preview label).
    pub fn font_size_value(&self) -> u32 {
        self.form
            .raw
            .get(KEY_THEME_FONT_SIZE)
            .and_then(|raw| raw.trim().parse::<u32>().ok())
            .unwrap_or(self.form.settings.theme.font_size)
    }

    /// The page path used by the PDF export row.
    ///
    /// The shell clears `current_page` when navigating to Settings, so the row
    /// is disabled in practice; a future host can set this to enable it.
    pub fn pdf_page(&self) -> Option<&str> {
        self.pdf_page.as_deref()
    }

    /// Set the page used by the PDF export row.
    pub fn set_pdf_page(&mut self, page: Option<String>) {
        self.pdf_page = page;
    }
}

/// Interpolate an export success message.
fn export_message(key: &str, count: usize, pages: usize, path: &str) -> String {
    tr_global(
        key,
        &[
            ("count", count.to_string().as_str()),
            ("pages", pages.to_string().as_str()),
            ("path", path),
        ],
    )
}

/// Keep the tab list referenced from actions for future host integrations.
pub fn default_tab() -> SettingsTab {
    SettingsTab::default()
}

/// Keep the `Arc` import meaningful for hosts that construct the panel.
pub type ProviderHandle = Arc<dyn crate::provider::ShellData>;
