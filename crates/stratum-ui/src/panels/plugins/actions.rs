//! Provider actions for the native Plugins panel.
//!
//! Every mutation and catalog refresh lives here so [`super`] stays focused on
//! state and accessors. Provider calls run synchronously (the [`PanelData`]
//! seam blocks on the service runtime), except install-from-file which opens the
//! platform picker and applies the result on the entity.
//!
//! [`PanelData`]: crate::provider::PanelData

use std::sync::Arc;

use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_app::themes::BUILTIN_THEME_ID;
use stratum_i18n::t_global;

use super::{failure, PluginsTab, PluginsView, TestOutcome};
use crate::theme::{apply_to_kit, ThemeConfig};

impl PluginsView {
    /// Switch tabs, lazily loading the catalog or theme hub on first open.
    pub fn set_tab(&mut self, tab: PluginsTab, cx: &mut Context<Self>) {
        self.tab = tab;
        match tab {
            PluginsTab::Installed => {}
            PluginsTab::Directory => {
                if self.catalog.is_none() && self.catalog_error.is_none() {
                    self.refresh_catalog(cx);
                }
            }
            PluginsTab::Themes => {
                if self.themes.is_none() && self.themes_error.is_none() {
                    self.refresh_themes(cx);
                }
            }
        }
        cx.notify();
    }

    /// Reload the installed plugin list.
    pub fn refresh_plugins(&mut self, cx: &mut Context<Self>) {
        match self.provider.plugins() {
            Ok(rows) => {
                self.plugins = rows;
                self.load_error = None;
            }
            Err(error) => {
                self.plugins.clear();
                self.load_error = Some(failure(&error));
            }
        }
        cx.notify();
    }

    /// Refresh the community plugin catalog.
    pub fn refresh_catalog(&mut self, cx: &mut Context<Self>) {
        self.catalog_error = None;
        match self.provider.plugin_catalog() {
            Ok(snapshot) => {
                self.catalog_error = snapshot.error.as_deref().map(|error| {
                    if snapshot.offline {
                        format!("{} ({error})", t_global("plugins.catalog_offline"))
                    } else {
                        format!("{} {error}", t_global("plugins.catalog_error"))
                    }
                });
                self.catalog = Some(snapshot);
            }
            Err(error) => {
                self.catalog_error = Some(format!("{} {error}", t_global("plugins.catalog_error")));
            }
        }
        cx.notify();
    }

    /// Refresh the theme catalog and the installed theme list.
    pub fn refresh_themes(&mut self, cx: &mut Context<Self>) {
        self.themes_error = None;
        match self.provider.theme_catalog() {
            Ok(snapshot) => {
                self.themes_error = snapshot.error.as_deref().map(|error| {
                    if snapshot.offline {
                        format!("{} ({error})", t_global("plugins.catalog_offline"))
                    } else {
                        format!("{} {error}", t_global("plugins.catalog_error"))
                    }
                });
                self.themes = Some(snapshot);
            }
            Err(error) => {
                self.themes_error = Some(format!("{} {error}", t_global("plugins.catalog_error")));
            }
        }
        self.reload_installed_themes();
        cx.notify();
    }

    /// Enable a plugin.
    pub fn enable(&mut self, id: &str, cx: &mut Context<Self>) {
        self.busy_id = Some(id.to_string());
        let result = self.provider.set_plugin_enabled(id, true);
        self.finish_row_action(id, result, cx);
    }

    /// Disable a plugin.
    pub fn disable(&mut self, id: &str, cx: &mut Context<Self>) {
        self.busy_id = Some(id.to_string());
        let result = self.provider.set_plugin_enabled(id, false);
        self.finish_row_action(id, result, cx);
    }

    /// Reload a plugin.
    pub fn reload(&mut self, id: &str, cx: &mut Context<Self>) {
        self.busy_id = Some(id.to_string());
        let result = self.provider.reload_plugin(id);
        self.finish_row_action(id, result, cx);
    }

    /// Uninstall a plugin (installed tab and directory tab share this).
    pub fn uninstall(&mut self, id: &str, cx: &mut Context<Self>) {
        self.busy_id = Some(id.to_string());
        match self.provider.uninstall_plugin(id) {
            Ok(()) => {
                self.load_error = None;
                self.catalog_error = None;
                self.plugins.retain(|plugin| plugin.id != id);
                self.test_results.remove(id);
                self.expanded_errors.remove(id);
            }
            Err(error) => {
                self.load_error = Some(format!(
                    "{} {id}: {error}",
                    t_global("plugins.uninstall_failed_on")
                ));
            }
        }
        self.busy_id = None;
        cx.notify();
    }

    /// Open the platform picker and install a local WASM plugin.
    ///
    /// The picker and the provider call run off the UI thread; a cancelled
    /// prompt leaves the panel untouched.
    pub fn install_from_file(&mut self, cx: &mut Context<Self>) {
        self.load_error = None;
        self.busy_id = Some("__install__".to_string());
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some(t_global("plugins.install").into()),
        });
        let provider = Arc::clone(&self.provider);
        cx.spawn(
            async move |this: WeakEntity<PluginsView>, cx: &mut AsyncApp| {
                let path = match receiver.await {
                    Ok(Ok(Some(paths))) => paths.into_iter().next(),
                    _ => None,
                };
                let Some(path) = path else {
                    let _ = this.update(cx, |view, cx| {
                        view.busy_id = None;
                        cx.notify();
                    });
                    return;
                };
                let path = path.to_string_lossy().to_string();
                let result = cx
                    .background_executor()
                    .spawn(async move { provider.install_plugin_path(&path) })
                    .await;
                let _ = this.update(cx, |view, cx| {
                    view.busy_id = None;
                    match result {
                        Ok(row) => view.upsert_plugin(row),
                        Err(error) => {
                            view.load_error =
                                Some(format!("{} {error}", t_global("plugins.install_failed")));
                        }
                    }
                    cx.notify();
                });
            },
        )
        .detach();
    }

    /// Install a catalog entry by id.
    pub fn install_catalog_plugin(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(entry) = self
            .catalog
            .as_ref()
            .and_then(|catalog| catalog.entries.iter().find(|entry| entry.id == id))
            .cloned()
        else {
            return;
        };
        self.busy_id = Some(id.to_string());
        self.catalog_error = None;
        let sha256 = (!entry.sha256.is_empty()).then_some(entry.sha256.as_str());
        match self
            .provider
            .install_plugin_url(&entry.download_url, sha256, &entry.id)
        {
            Ok(row) => self.upsert_plugin(row),
            Err(error) => {
                self.catalog_error = Some(format!(
                    "{} {id}: {error}",
                    t_global("plugins.install_failed")
                ));
            }
        }
        self.busy_id = None;
        cx.notify();
    }

    /// Run the note-read smoke test for a plugin.
    pub fn run_note_read_test(&mut self, id: &str, cx: &mut Context<Self>) {
        self.busy_id = Some(id.to_string());
        let result = self
            .provider
            .plugin_note_read("test-plugin.md")
            .map(|content| crate::provider::PluginTestResult {
                ok: true,
                detail: format!("test-plugin.md ({} bytes)", content.len()),
            });
        self.finish_test(id, "plugins.test_note_read", result, cx);
    }

    /// Run the HTTP smoke test for a plugin.
    pub fn run_http_request_test(&mut self, id: &str, cx: &mut Context<Self>) {
        self.busy_id = Some(id.to_string());
        let result = self
            .provider
            .plugin_http_request("https://example.com/data.json", "GET")
            .map(|detail| crate::provider::PluginTestResult { ok: true, detail });
        self.finish_test(id, "plugins.test_http_request", result, cx);
    }

    /// Dismiss an inline smoke-test result.
    pub fn clear_test(&mut self, id: &str, cx: &mut Context<Self>) {
        self.test_results.remove(id);
        cx.notify();
    }

    /// Expand or collapse a plugin's error detail.
    pub fn toggle_error(&mut self, id: &str, cx: &mut Context<Self>) {
        if !self.expanded_errors.remove(id) {
            self.expanded_errors.insert(id.to_string());
        }
        cx.notify();
    }

    /// Install a catalog theme and apply it immediately.
    pub fn install_theme(&mut self, id: &str, cx: &mut Context<Self>) {
        self.busy_id = Some(id.to_string());
        self.themes_error = None;
        match self.provider.install_theme(id) {
            Ok(_) => {
                self.reload_installed_themes();
                self.apply_theme(id, cx);
            }
            Err(error) => {
                self.themes_error = Some(format!(
                    "{} {id}: {error}",
                    t_global("plugins.install_failed")
                ));
            }
        }
        self.busy_id = None;
        cx.notify();
    }

    /// Apply an installed theme through the provider and the shell theme.
    pub fn apply_theme(&mut self, id: &str, cx: &mut Context<Self>) {
        self.busy_id = Some(id.to_string());
        match self.provider.apply_theme(id) {
            Ok(values) => {
                let cfg = ThemeConfig {
                    primary_hex: values.primary_color,
                    secondary_hex: values.secondary_color,
                    dark: values.dark_mode,
                    font_size: Some(values.font_size as f32),
                };
                apply_to_kit(cx, &cfg);
                self.themes_error = None;
                self.reload_installed_themes();
            }
            Err(error) => {
                self.themes_error =
                    Some(format!("{} {id}: {error}", t_global("plugins.failed_on")));
            }
        }
        self.busy_id = None;
        cx.notify();
    }

    /// Uninstall a theme; removing the active theme falls back to the built-in.
    pub fn uninstall_theme(&mut self, id: &str, cx: &mut Context<Self>) {
        let was_active = self
            .installed_themes
            .iter()
            .any(|theme| theme.id == id && theme.active);
        self.busy_id = Some(id.to_string());
        match self.provider.uninstall_theme(id) {
            Ok(()) => {
                self.themes_error = None;
                self.reload_installed_themes();
                if was_active {
                    match self.provider.apply_theme(BUILTIN_THEME_ID) {
                        Ok(values) => {
                            let cfg = ThemeConfig {
                                primary_hex: values.primary_color,
                                secondary_hex: values.secondary_color,
                                dark: values.dark_mode,
                                font_size: Some(values.font_size as f32),
                            };
                            apply_to_kit(cx, &cfg);
                        }
                        Err(error) => {
                            self.themes_error =
                                Some(format!("{} {error}", t_global("plugins.failed_on")));
                        }
                    }
                }
            }
            Err(error) => {
                self.themes_error = Some(format!(
                    "{} {id}: {error}",
                    t_global("plugins.uninstall_failed_on")
                ));
            }
        }
        self.busy_id = None;
        cx.notify();
    }

    /// Dismiss the panel error banner.
    pub fn clear_error(&mut self, cx: &mut Context<Self>) {
        self.load_error = None;
        self.catalog_error = None;
        self.themes_error = None;
        cx.notify();
    }

    /// Reload the installed theme rows.
    fn reload_installed_themes(&mut self) {
        self.installed_themes = self.provider.installed_themes();
    }

    /// Insert or replace a plugin row.
    fn upsert_plugin(&mut self, row: crate::provider::PluginRow) {
        self.load_error = None;
        match self.plugins.iter_mut().find(|plugin| plugin.id == row.id) {
            Some(existing) => *existing = row,
            None => self.plugins.push(row),
        }
    }

    /// Apply a row-action result to the panel.
    fn finish_row_action(
        &mut self,
        id: &str,
        result: Result<crate::provider::PluginRow, String>,
        cx: &mut Context<Self>,
    ) {
        self.busy_id = None;
        match result {
            Ok(row) => {
                self.load_error = None;
                self.upsert_plugin(row);
            }
            Err(error) => {
                self.load_error = Some(format!("{} {id}: {error}", t_global("plugins.failed_on")));
            }
        }
        cx.notify();
    }

    /// Record a smoke-test result.
    fn finish_test(
        &mut self,
        id: &str,
        label_key: &'static str,
        result: Result<crate::provider::PluginTestResult, String>,
        cx: &mut Context<Self>,
    ) {
        self.busy_id = None;
        let result = result.unwrap_or_else(|error| crate::provider::PluginTestResult {
            ok: false,
            detail: error,
        });
        self.test_results
            .insert(id.to_string(), TestOutcome { label_key, result });
        cx.notify();
    }
}
