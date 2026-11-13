//! Community theme hub tab for the native Plugins panel.
//!
//! Renders the installed themes first (offline-safe) and then the catalog
//! entries that are not installed, matching the shipped `ThemeHubTab`. Installing
//! applies the theme immediately; uninstalling the active theme falls back to the
//! built-in theme in [`super::actions`]. Entries are provider data; every label
//! is a catalog key from [`super::PLUGINS_I18N_KEYS`].
//!
//! Theme colors are copied into locals before any helper that borrows the
//! context mutably, so the render helpers stay free of borrow conflicts.

use gpui_kit::component::ActiveTheme;
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use super::view::{action_button, author_line, badge, version_chip};
use super::PluginsView;
use crate::provider::{InstalledThemeRow, ThemeCatalogEntry};

/// Render the theme hub tab.
pub fn render(view: &PluginsView, cx: &mut Context<PluginsView>) -> AnyElement {
    let theme = cx.theme();
    let muted = theme.colors.muted_foreground;
    div()
        .id("plugins-themes")
        .debug_selector(|| "plugins-themes".to_string())
        .flex()
        .flex_col()
        .gap(px(10.0))
        .child(header(view, cx))
        .child(content(view, cx))
        .child(
            div()
                .text_size(px(9.0))
                .text_color(muted)
                .child(t_global("plugins.verified_sha256")),
        )
        .into_any_element()
}

/// The theme hub title, subtitle, offline badge and refresh action.
fn header(view: &PluginsView, cx: &mut Context<PluginsView>) -> AnyElement {
    let theme = cx.theme();
    let foreground = theme.colors.foreground;
    let muted = theme.colors.muted_foreground;
    let warning = theme.colors.warning;
    let offline = view.themes_catalog().is_some_and(|catalog| catalog.offline);
    let fetched_at = view
        .themes_catalog()
        .and_then(|catalog| catalog.fetched_at.clone());
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(10.0))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(2.0))
                .child(
                    div()
                        .text_size(px(14.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(foreground)
                        .child(t_global("plugins.themes_title")),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .text_size(px(10.0))
                        .text_color(muted)
                        .child(t_global("plugins.themes_subtitle"))
                        .when(offline, |element| {
                            element.child(badge(
                                t_global("plugins.offline_badge"),
                                warning,
                                warning.opacity(0.15),
                                cx,
                            ))
                        })
                        .when_some(fetched_at, |element, fetched_at| {
                            element.child(div().child(format!(
                                "{} {fetched_at}",
                                t_global("plugins.catalog_fetched_at")
                            )))
                        }),
                ),
        )
        .child(action_button(
            "plugins-themes-refresh",
            t_global("common.refresh"),
            false,
            cx,
            |view, _window, cx| view.refresh_themes(cx),
        ))
        .into_any_element()
}

/// The installed themes, catalog entries and empty state.
fn content(view: &PluginsView, cx: &mut Context<PluginsView>) -> AnyElement {
    let theme = cx.theme();
    let border = theme.colors.border;
    let catalog_entries = view
        .themes_catalog()
        .map(|catalog| catalog.entries.as_slice())
        .unwrap_or(&[]);
    if view.installed_themes().is_empty() && catalog_entries.is_empty() {
        return empty_state(view, cx);
    }
    let installed_ids: Vec<&str> = view
        .installed_themes()
        .iter()
        .map(|row| row.id.as_str())
        .collect();
    div()
        .flex()
        .flex_col()
        .gap(px(10.0))
        .children(view.installed_themes().iter().map(|row| {
            div()
                .border_1()
                .border_color(border)
                .rounded(px(8.0))
                .p(px(14.0))
                .child(installed_card(
                    row,
                    catalog_entries.iter().find(|entry| entry.id == row.id),
                    view,
                    cx,
                ))
        }))
        .children(
            catalog_entries
                .iter()
                .filter(|entry| !installed_ids.contains(&entry.id.as_str()))
                .map(|entry| {
                    div()
                        .border_1()
                        .border_color(border)
                        .rounded(px(8.0))
                        .p(px(14.0))
                        .child(catalog_card(entry, view, cx))
                }),
        )
        .into_any_element()
}

/// The empty theme hub with a retry action.
fn empty_state(view: &PluginsView, cx: &mut Context<PluginsView>) -> AnyElement {
    let theme = cx.theme();
    let muted = theme.colors.muted_foreground;
    let loading = view.themes_catalog().is_none();
    div()
        .id("plugins-themes-empty")
        .debug_selector(|| "plugins-themes-empty".to_string())
        .flex()
        .flex_col()
        .items_center()
        .gap(px(6.0))
        .py(px(32.0))
        .child(
            div()
                .text_size(px(11.0))
                .text_color(muted)
                .child(if loading {
                    t_global("plugins.loading_plugins")
                } else {
                    t_global("plugins.directory_empty")
                }),
        )
        .child(action_button(
            "plugins-themes-retry",
            t_global("common.retry"),
            false,
            cx,
            |view, _window, cx| view.refresh_themes(cx),
        ))
        .into_any_element()
}

/// Render one installed theme with apply/uninstall actions.
fn installed_card(
    row: &InstalledThemeRow,
    entry: Option<&ThemeCatalogEntry>,
    view: &PluginsView,
    cx: &mut Context<PluginsView>,
) -> AnyElement {
    let theme = cx.theme();
    let muted = theme.colors.muted_foreground;
    let success = theme.colors.success;
    let busy = view.busy_id() == Some(row.id.as_str());
    let apply_id = row.id.clone();
    let uninstall_id = row.id.clone();
    div()
        .id(format!("theme-installed-{}", row.id))
        .debug_selector({
            let id = row.id.clone();
            move || format!("theme-installed-{id}")
        })
        .flex()
        .items_start()
        .justify_between()
        .gap(px(12.0))
        .child(theme_meta(&row.name, &row.id, entry, cx))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(4.0))
                .when(row.active, |element| {
                    element.child(badge(
                        t_global("plugins.active_theme"),
                        success,
                        success.opacity(0.12),
                        cx,
                    ))
                })
                .when(busy, |element| {
                    element.child(
                        div()
                            .text_size(px(10.0))
                            .text_color(muted)
                            .child(t_global("plugins.loading_plugins")),
                    )
                })
                .child(action_button(
                    format!("theme-apply-{}", row.id),
                    t_global("plugins.apply"),
                    false,
                    cx,
                    move |view, _window, cx| view.apply_theme(&apply_id, cx),
                ))
                .child(action_button(
                    format!("theme-uninstall-{}", row.id),
                    t_global("plugins.uninstall"),
                    false,
                    cx,
                    move |view, _window, cx| view.uninstall_theme(&uninstall_id, cx),
                )),
        )
        .into_any_element()
}

/// Render one catalog theme that is not installed.
fn catalog_card(
    entry: &ThemeCatalogEntry,
    view: &PluginsView,
    cx: &mut Context<PluginsView>,
) -> AnyElement {
    let theme = cx.theme();
    let muted = theme.colors.muted_foreground;
    let busy = view.busy_id() == Some(entry.id.as_str());
    let install_id = entry.id.clone();
    div()
        .id(format!("theme-catalog-{}", entry.id))
        .debug_selector({
            let id = entry.id.clone();
            move || format!("theme-catalog-{id}")
        })
        .flex()
        .items_start()
        .justify_between()
        .gap(px(12.0))
        .child(theme_meta(&entry.name, &entry.id, Some(entry), cx))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(4.0))
                .when(busy, |element| {
                    element.child(
                        div()
                            .text_size(px(10.0))
                            .text_color(muted)
                            .child(t_global("plugins.installing")),
                    )
                })
                .child(action_button(
                    format!("theme-install-{}", entry.id),
                    t_global("plugins.install"),
                    true,
                    cx,
                    move |view, _window, cx| view.install_theme(&install_id, cx),
                )),
        )
        .into_any_element()
}

/// Render a theme's name, id, metadata and color mode.
fn theme_meta(
    name: &str,
    id: &str,
    entry: Option<&ThemeCatalogEntry>,
    cx: &mut Context<PluginsView>,
) -> AnyElement {
    let theme = cx.theme();
    let foreground = theme.colors.foreground;
    let muted = theme.colors.muted_foreground;
    let version = entry.map(|entry| entry.version.clone());
    let description = entry
        .map(|entry| entry.description.clone())
        .filter(|description| !description.is_empty());
    let author = entry.and_then(|entry| entry.author.clone());
    let mode = entry.map(|entry| {
        let key = if entry.dark_mode {
            "plugins.theme_dark"
        } else {
            "plugins.theme_light"
        };
        format!("{} · {}", t_global(key), entry.primary_color)
    });
    div()
        .flex()
        .flex_col()
        .gap(px(3.0))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(
                    div()
                        .text_size(px(13.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(foreground)
                        .child(name.to_string()),
                )
                .when_some(version, |element, version| {
                    element.child(version_chip(&version, cx))
                }),
        )
        .child(
            div()
                .text_size(px(10.0))
                .text_color(muted)
                .child(id.to_string()),
        )
        .when_some(description, |element, description| {
            element.child(
                div()
                    .text_size(px(11.0))
                    .text_color(muted)
                    .child(description),
            )
        })
        .when_some(author, |element, author| {
            element.child(author_line(&author, cx))
        })
        .when_some(mode, |element, mode| {
            element.child(div().text_size(px(10.0)).text_color(muted).child(mode))
        })
        .into_any_element()
}
