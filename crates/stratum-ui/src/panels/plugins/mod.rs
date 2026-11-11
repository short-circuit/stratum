//! Native Plugins panel.
//!
//! [`PluginsView`] ports the shipped desktop `PluginsPanel`: the installed,
//! directory and themes tabs, enable/disable/reload/uninstall, install from a
//! local WASM file, the note-read and HTTP smoke tests, sha256-verified catalog
//! installs and the theme hub (install/apply/uninstall with the built-in
//! fallback). Provider calls stay behind the [`crate::provider::PanelData`]
//! seam; rendering lives in [`view`], [`installed`], [`directory`] and
//! [`themes`], while provider mutations live in [`actions`].
//!
//! [`PLUGINS_I18N_KEYS`] is the release gate list for every string this surface
//! renders. The shipped status chip uses literal `Ready`/`Disabled`/`Error`
//! labels, but no catalog carries a `plugins.ready` or `plugins.error` key; the
//! native chip reuses `plugins.installed`, `plugins.disable` and
//! `plugins.failed_on` instead (see [`status_key`]). Provider text (versions,
//! ids, permissions, catalog metadata and error bodies) is passed through as
//! data, matching the shipped panel.

use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;

use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use crate::panels::PanelRequests;
use crate::provider::{
    InstalledThemeRow, PluginCatalogSnapshot, PluginRow, PluginTestResult, ShellData,
    ThemeCatalogSnapshot,
};

pub mod actions;
pub mod directory;
pub mod installed;
pub mod themes;
pub mod view;

/// Every translation key the Plugins surface renders.
///
/// The contract test walks this list against all eight catalogs, so adding a
/// visible string without a catalog entry fails the build instead of leaking a
/// raw key into the UI.
pub const PLUGINS_I18N_KEYS: &[&str] = &[
    "common.refresh",
    "common.retry",
    "mobile_layout.plugins",
    "plugins._pkm_plugins",
    "plugins.active_theme",
    "plugins.apply",
    "plugins.by_author",
    "plugins.catalog_error",
    "plugins.catalog_fetched_at",
    "plugins.catalog_offline",
    "plugins.click_for_details",
    "plugins.click_to_dismiss",
    "plugins.directory_empty",
    "plugins.directory_subtitle",
    "plugins.directory_title",
    "plugins.disable",
    "plugins.enable",
    "plugins.failed_on",
    "plugins.hooks",
    "plugins.install",
    "plugins.install_a_plugin_wasm_file_or_drop_one_into_pkm_",
    "plugins.install_failed",
    "plugins.installed",
    "plugins.installing",
    "plugins.loading_plugins",
    "plugins.no_plugins_installed",
    "plugins.offline_badge",
    "plugins.reload",
    "plugins.tab_directory",
    "plugins.tab_installed",
    "plugins.tab_themes",
    "plugins.test_http_request",
    "plugins.test_note_read",
    "plugins.theme_dark",
    "plugins.theme_light",
    "plugins.themes_subtitle",
    "plugins.themes_title",
    "plugins.uninstall",
    "plugins.uninstall_failed_on",
    "plugins.verified_sha256",
    "plugins.wasm_plugins_loaded_from",
];

/// The three shipped plugin tabs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PluginsTab {
    /// Installed plugins.
    #[default]
    Installed,
    /// Community plugin directory.
    Directory,
    /// Community theme hub.
    Themes,
}

impl PluginsTab {
    /// Parse a `STRATUM_PLUGINS_TAB` value.
    pub fn from_env(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "installed" => Some(Self::Installed),
            "directory" => Some(Self::Directory),
            "themes" => Some(Self::Themes),
            _ => None,
        }
    }

    /// The catalog key for this tab label.
    pub fn label_key(self) -> &'static str {
        match self {
            Self::Installed => "plugins.tab_installed",
            Self::Directory => "plugins.tab_directory",
            Self::Themes => "plugins.tab_themes",
        }
    }
}

/// Deterministic start state for the plugins panel.
///
/// `STRATUM_PLUGINS_TAB=installed|directory|themes` selects the tab and loads
/// its data during construction so the capture harness can render the catalog
/// or theme hub without a click.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PluginsStart {
    /// Tab to open after construction.
    pub tab: Option<PluginsTab>,
}

/// One inline plugin smoke-test outcome.
#[derive(Clone, Debug, PartialEq)]
pub struct TestOutcome {
    /// Catalog key naming the test (`plugins.test_note_read` or
    /// `plugins.test_http_request`).
    pub label_key: &'static str,
    /// Provider result; failures carry the provider error as `detail`.
    pub result: PluginTestResult,
}

/// The plugins panel entity.
pub struct PluginsView {
    provider: Arc<dyn ShellData>,
    tab: PluginsTab,
    plugins: Vec<PluginRow>,
    load_error: Option<String>,
    busy_id: Option<String>,
    test_results: BTreeMap<String, TestOutcome>,
    catalog: Option<PluginCatalogSnapshot>,
    catalog_error: Option<String>,
    themes: Option<ThemeCatalogSnapshot>,
    themes_error: Option<String>,
    installed_themes: Vec<InstalledThemeRow>,
    expanded_errors: HashSet<String>,
    pending_navigation: Option<String>,
    pending_toast: Option<String>,
}

impl PluginsView {
    /// Build the panel, load the installed list and apply the start tab.
    pub fn new(provider: Arc<dyn ShellData>, start: PluginsStart, cx: &mut Context<Self>) -> Self {
        let (plugins, load_error) = match provider.plugins() {
            Ok(rows) => (rows, None),
            Err(error) => (Vec::new(), Some(failure(&error))),
        };
        let mut view = Self {
            provider,
            tab: start.tab.unwrap_or_default(),
            plugins,
            load_error,
            busy_id: None,
            test_results: BTreeMap::new(),
            catalog: None,
            catalog_error: None,
            themes: None,
            themes_error: None,
            installed_themes: Vec::new(),
            expanded_errors: HashSet::new(),
            pending_navigation: None,
            pending_toast: None,
        };
        match view.tab {
            PluginsTab::Installed => {}
            PluginsTab::Directory => view.refresh_catalog(cx),
            PluginsTab::Themes => view.refresh_themes(cx),
        }
        view
    }

    /// The installed plugin rows.
    pub fn plugins(&self) -> &[PluginRow] {
        &self.plugins
    }

    /// The active tab.
    pub fn tab(&self) -> PluginsTab {
        self.tab
    }

    /// The installed-list load error, if any.
    pub fn load_error(&self) -> Option<&str> {
        self.load_error.as_deref()
    }

    /// The id of the plugin currently being acted on, if any.
    pub fn busy_id(&self) -> Option<&str> {
        self.busy_id.as_deref()
    }

    /// The inline smoke-test outcome for a plugin, if any.
    pub fn test_result(&self, id: &str) -> Option<&TestOutcome> {
        self.test_results.get(id)
    }

    /// The plugin catalog snapshot, if loaded.
    pub fn catalog(&self) -> Option<&PluginCatalogSnapshot> {
        self.catalog.as_ref()
    }

    /// The plugin catalog error, if any.
    pub fn catalog_error(&self) -> Option<&str> {
        self.catalog_error.as_deref()
    }

    /// The theme catalog snapshot, if loaded.
    pub fn themes_catalog(&self) -> Option<&ThemeCatalogSnapshot> {
        self.themes.as_ref()
    }

    /// The theme catalog error, if any.
    pub fn themes_error(&self) -> Option<&str> {
        self.themes_error.as_deref()
    }

    /// The installed theme rows.
    pub fn installed_themes(&self) -> &[InstalledThemeRow] {
        &self.installed_themes
    }

    /// Whether the plugin with `id` is installed.
    pub fn is_installed(&self, id: &str) -> bool {
        self.plugins.iter().any(|plugin| plugin.id == id)
    }

    /// Whether a plugin's error detail is expanded.
    pub fn error_expanded(&self, id: &str) -> bool {
        self.expanded_errors.contains(id)
    }
}

impl PanelRequests for PluginsView {
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

impl Render for PluginsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        view::render(self, cx)
    }
}

/// Map a provider plugin status onto an existing catalog label.
///
/// The shipped chip renders literal `Ready`/`Disabled`/`Error`; the catalogs
/// carry no `plugins.ready`/`plugins.error` key, so the closest existing labels
/// are used instead. The status value itself is provider data.
pub(crate) fn status_key(status: &str) -> &'static str {
    match status {
        "ready" => "plugins.installed",
        "disabled" => "plugins.disable",
        _ => "plugins.failed_on",
    }
}

/// Prefix a provider error with the localized panel title.
pub(crate) fn failure(error: &str) -> String {
    format!("{}: {error}", t_global("mobile_layout.plugins"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::core::prelude::v1::test;

    #[test]
    fn tab_labels_are_catalog_keys() {
        for tab in [
            PluginsTab::Installed,
            PluginsTab::Directory,
            PluginsTab::Themes,
        ] {
            assert!(PLUGINS_I18N_KEYS.contains(&tab.label_key()));
        }
    }

    #[test]
    fn from_env_parses_the_shipped_tabs() {
        assert_eq!(
            PluginsTab::from_env("installed"),
            Some(PluginsTab::Installed)
        );
        assert_eq!(
            PluginsTab::from_env("DIRECTORY"),
            Some(PluginsTab::Directory)
        );
        assert_eq!(PluginsTab::from_env("themes"), Some(PluginsTab::Themes));
        assert_eq!(PluginsTab::from_env("nope"), None);
    }

    #[test]
    fn status_keys_exist_in_the_gate_list() {
        for status in ["ready", "disabled", "error"] {
            assert!(PLUGINS_I18N_KEYS.contains(&status_key(status)));
        }
    }

    #[test]
    fn failure_prefixes_the_panel_title() {
        assert_eq!(failure("boom"), "Plugins: boom");
    }
}
