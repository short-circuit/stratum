//! Community plugin directory tab for the native Plugins panel.
//!
//! Renders the cached catalog exactly like the shipped `PluginDirectoryTab`:
//! offline badge, fetched-at caption, verified-sha256 caption and the
//! install/uninstall action per entry. Entries and their metadata are provider
//! data; every label is a catalog key from [`super::PLUGINS_I18N_KEYS`].
//!
//! Theme colors are copied into locals before any helper that borrows the
//! context mutably, so the render helpers stay free of borrow conflicts.

use gpui_kit::component::ActiveTheme;
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use super::view::{action_button, author_line, badge, permission_chips, version_chip};
use super::PluginsView;
use crate::provider::PluginCatalogEntry;

/// Render the plugin directory tab.
pub fn render(view: &PluginsView, cx: &mut Context<PluginsView>) -> AnyElement {
    let theme = cx.theme();
    let muted = theme.colors.muted_foreground;
    div()
        .id("plugins-directory")
        .debug_selector(|| "plugins-directory".to_string())
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

/// The directory title, subtitle, offline badge and refresh action.
fn header(view: &PluginsView, cx: &mut Context<PluginsView>) -> AnyElement {
    let theme = cx.theme();
    let foreground = theme.colors.foreground;
    let muted = theme.colors.muted_foreground;
    let warning = theme.colors.warning;
    let offline = view.catalog().is_some_and(|catalog| catalog.offline);
    let fetched_at = view
        .catalog()
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
                        .child(t_global("plugins.directory_title")),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .text_size(px(10.0))
                        .text_color(muted)
                        .child(t_global("plugins.directory_subtitle"))
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
            "plugins-catalog-refresh",
            t_global("common.refresh"),
            false,
            cx,
            |view, _window, cx| view.refresh_catalog(cx),
        ))
        .into_any_element()
}

/// The catalog list, loading placeholder or empty state.
fn content(view: &PluginsView, cx: &mut Context<PluginsView>) -> AnyElement {
    let theme = cx.theme();
    let muted = theme.colors.muted_foreground;
    let border = theme.colors.border;
    let Some(catalog) = view.catalog() else {
        return div()
            .id("plugins-directory-loading")
            .debug_selector(|| "plugins-directory-loading".to_string())
            .text_size(px(11.0))
            .text_color(muted)
            .child(t_global("plugins.loading_plugins"))
            .into_any_element();
    };
    if catalog.entries.is_empty() {
        return empty_state(cx);
    }
    div()
        .flex()
        .flex_col()
        .gap(px(10.0))
        .children(catalog.entries.iter().map(|entry| {
            div()
                .border_1()
                .border_color(border)
                .rounded(px(8.0))
                .p(px(14.0))
                .child(card(entry, view, cx))
        }))
        .into_any_element()
}

/// The empty catalog hint with a retry action.
fn empty_state(cx: &mut Context<PluginsView>) -> AnyElement {
    let theme = cx.theme();
    let muted = theme.colors.muted_foreground;
    div()
        .id("plugins-directory-empty")
        .debug_selector(|| "plugins-directory-empty".to_string())
        .flex()
        .flex_col()
        .items_center()
        .gap(px(6.0))
        .py(px(32.0))
        .child(
            div()
                .text_size(px(11.0))
                .text_color(muted)
                .child(t_global("plugins.directory_empty")),
        )
        .child(action_button(
            "plugins-directory-retry",
            t_global("common.retry"),
            false,
            cx,
            |view, _window, cx| view.refresh_catalog(cx),
        ))
        .into_any_element()
}

/// Render one catalog entry.
fn card(
    entry: &PluginCatalogEntry,
    view: &PluginsView,
    cx: &mut Context<PluginsView>,
) -> AnyElement {
    let theme = cx.theme();
    let foreground = theme.colors.foreground;
    let muted = theme.colors.muted_foreground;
    let success = theme.colors.success;
    let installed = view.is_installed(&entry.id);
    let busy = view.busy_id() == Some(entry.id.as_str());
    let install_id = entry.id.clone();
    let uninstall_id = entry.id.clone();
    div()
        .id(format!("plugin-catalog-{}", entry.id))
        .debug_selector({
            let id = entry.id.clone();
            move || format!("plugin-catalog-{id}")
        })
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(
            div()
                .flex()
                .items_start()
                .justify_between()
                .gap(px(12.0))
                .child(
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
                                        .child(entry.name.clone()),
                                )
                                .child(version_chip(&entry.version, cx))
                                .when(installed, |element| {
                                    element.child(badge(
                                        t_global("plugins.installed"),
                                        success,
                                        success.opacity(0.12),
                                        cx,
                                    ))
                                }),
                        )
                        .child(
                            div()
                                .text_size(px(10.0))
                                .text_color(muted)
                                .child(entry.id.clone()),
                        )
                        .when(!entry.description.is_empty(), |element| {
                            element.child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(muted)
                                    .child(entry.description.clone()),
                            )
                        })
                        .when_some(entry.author.clone(), |element, author| {
                            element.child(author_line(&author, cx))
                        })
                        .child(permission_chips(&entry.permissions, cx)),
                )
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
                        .when(installed, |element| {
                            element.child(action_button(
                                format!("plugin-catalog-uninstall-{}", entry.id),
                                t_global("plugins.uninstall"),
                                false,
                                cx,
                                move |view, _window, cx| view.uninstall(&uninstall_id, cx),
                            ))
                        })
                        .when(!installed, |element| {
                            element.child(action_button(
                                format!("plugin-catalog-install-{}", entry.id),
                                t_global("plugins.install"),
                                true,
                                cx,
                                move |view, _window, cx| {
                                    view.install_catalog_plugin(&install_id, cx)
                                },
                            ))
                        }),
                ),
        )
        .into_any_element()
}
