//! Installed-plugin tab for the native Plugins panel.
//!
//! Renders the shipped plugin cards: manifest metadata, the status chip with
//! its expandable error detail, enable/disable/reload/uninstall actions and the
//! note-read/HTTP smoke-test row. All visible strings are catalog keys from
//! [`super::PLUGINS_I18N_KEYS`].
//!
//! Theme colors are copied into locals before any helper that borrows the
//! context mutably, so the render helpers stay free of borrow conflicts.

use gpui_kit::assets::IconName;
use gpui_kit::component::{ActiveTheme, Icon, Sizable};
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use super::view::{action_button, author_line, permission_chips, version_chip};
use super::{status_key, PluginsView};
use crate::provider::PluginRow;

/// Render the installed plugins tab.
pub fn render(view: &PluginsView, cx: &mut Context<PluginsView>) -> AnyElement {
    if view.plugins().is_empty() {
        return empty_state(cx);
    }
    let theme = cx.theme();
    let border = theme.colors.border;
    div()
        .id("plugins-installed")
        .debug_selector(|| "plugins-installed".to_string())
        .flex()
        .flex_col()
        .gap(px(10.0))
        .children(view.plugins().iter().map(|plugin| {
            div()
                .border_1()
                .border_color(border)
                .rounded(px(8.0))
                .p(px(14.0))
                .child(card(plugin, view, cx))
        }))
        .into_any_element()
}

/// The shipped empty state with its install action.
fn empty_state(cx: &mut Context<PluginsView>) -> AnyElement {
    let theme = cx.theme();
    let muted = theme.colors.muted_foreground;
    let foreground = theme.colors.foreground;
    div()
        .id("plugins-empty")
        .debug_selector(|| "plugins-empty".to_string())
        .flex()
        .flex_col()
        .items_center()
        .gap(px(8.0))
        .py(px(40.0))
        .child(
            Icon::new(IconName::Puzzle)
                .with_size(px(40.0))
                .text_color(muted),
        )
        .child(
            div()
                .text_size(px(13.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(foreground)
                .child(t_global("plugins.no_plugins_installed")),
        )
        .child(div().text_size(px(11.0)).text_color(muted).child(t_global(
            "plugins.install_a_plugin_wasm_file_or_drop_one_into_pkm_",
        )))
        .child(action_button(
            "plugins-empty-install",
            t_global("plugins.install"),
            true,
            cx,
            |view, _window, cx| view.install_from_file(cx),
        ))
        .into_any_element()
}

/// Render one installed plugin card.
fn card(plugin: &PluginRow, view: &PluginsView, cx: &mut Context<PluginsView>) -> AnyElement {
    let theme = cx.theme();
    let foreground = theme.colors.foreground;
    let muted = theme.colors.muted_foreground;
    let busy = view.busy_id() == Some(plugin.id.as_str());
    div()
        .id(format!("plugin-card-{}", plugin.id))
        .debug_selector({
            let id = plugin.id.clone();
            move || format!("plugin-card-{id}")
        })
        .flex()
        .flex_col()
        .gap(px(8.0))
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
                                        .child(plugin.name.clone()),
                                )
                                .child(version_chip(&plugin.version, cx)),
                        )
                        .child(
                            div()
                                .text_size(px(10.0))
                                .text_color(muted)
                                .child(plugin.id.clone()),
                        )
                        .when(!plugin.description.is_empty(), |element| {
                            element.child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(muted)
                                    .child(plugin.description.clone()),
                            )
                        })
                        .when(!plugin.author.is_empty(), |element| {
                            element.child(author_line(&plugin.author, cx))
                        })
                        .child(permission_chips(&plugin.permissions, cx))
                        .when(!plugin.hooks.is_empty(), |element| {
                            element.child(hook_row(&plugin.hooks, cx))
                        }),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .items_end()
                        .gap(px(6.0))
                        .child(status_chip(plugin, view, cx))
                        .child(row_actions(plugin, busy, cx)),
                ),
        )
        .child(test_row(plugin, view, busy, cx))
        .into_any_element()
}

/// The localized status chip; clicking an errored chip expands the detail.
fn status_chip(
    plugin: &PluginRow,
    view: &PluginsView,
    cx: &mut Context<PluginsView>,
) -> AnyElement {
    let theme = cx.theme();
    let success = theme.colors.success;
    let muted = theme.colors.muted_foreground;
    let danger = theme.colors.danger;
    let label = t_global(status_key(&plugin.status));
    let (color, background) = match plugin.status.as_str() {
        "ready" => (success, success.opacity(0.12)),
        "disabled" => (muted, muted.opacity(0.15)),
        _ => (danger, danger.opacity(0.12)),
    };
    let expanded = view.error_expanded(&plugin.id);
    let id = plugin.id.clone();
    let clickable = plugin.error.is_some();
    div()
        .flex()
        .flex_col()
        .items_start()
        .gap(px(4.0))
        .child(
            div()
                .id(format!("plugin-status-{}", plugin.id))
                .debug_selector({
                    let id = plugin.id.clone();
                    move || format!("plugin-status-{id}")
                })
                .px(px(7.0))
                .py(px(2.0))
                .rounded(px(4.0))
                .bg(background)
                .text_color(color)
                .text_size(px(9.0))
                .font_weight(FontWeight::SEMIBOLD)
                .when(clickable, |element| element.cursor_pointer())
                .child(label)
                .on_click(cx.listener(move |view, _, _, cx| view.toggle_error(&id, cx))),
        )
        .when(clickable && expanded, |element| {
            element.child(
                div()
                    .id(format!("plugin-error-{}", plugin.id))
                    .debug_selector({
                        let id = plugin.id.clone();
                        move || format!("plugin-error-{id}")
                    })
                    .max_w(px(420.0))
                    .text_size(px(10.0))
                    .text_color(muted)
                    .child(plugin.error.clone().unwrap_or_default()),
            )
        })
        .into_any_element()
}

/// Enable/disable, reload and uninstall actions.
fn row_actions(plugin: &PluginRow, busy: bool, cx: &mut Context<PluginsView>) -> AnyElement {
    let toggle_label = if plugin.enabled {
        t_global("plugins.disable")
    } else {
        t_global("plugins.enable")
    };
    let enabled = plugin.enabled;
    let toggle_id = plugin.id.clone();
    let reload_id = plugin.id.clone();
    let uninstall_id = plugin.id.clone();
    div()
        .flex()
        .items_center()
        .gap(px(4.0))
        .when(busy, |element| {
            element.child(
                div()
                    .text_size(px(10.0))
                    .child(t_global("plugins.loading_plugins")),
            )
        })
        .child(action_button(
            format!("plugin-toggle-{}", plugin.id),
            toggle_label,
            !enabled,
            cx,
            move |view, _window, cx| {
                if enabled {
                    view.disable(&toggle_id, cx);
                } else {
                    view.enable(&toggle_id, cx);
                }
            },
        ))
        .child(action_button(
            format!("plugin-reload-{}", plugin.id),
            t_global("plugins.reload"),
            false,
            cx,
            move |view, _window, cx| view.reload(&reload_id, cx),
        ))
        .child(action_button(
            format!("plugin-uninstall-{}", plugin.id),
            t_global("plugins.uninstall"),
            false,
            cx,
            move |view, _window, cx| view.uninstall(&uninstall_id, cx),
        ))
        .into_any_element()
}

/// The note-read/HTTP smoke-test row and its inline result.
fn test_row(
    plugin: &PluginRow,
    view: &PluginsView,
    busy: bool,
    cx: &mut Context<PluginsView>,
) -> AnyElement {
    let theme = cx.theme();
    let muted = theme.colors.muted_foreground;
    let border = theme.colors.border;
    let success = theme.colors.success;
    let danger = theme.colors.danger;
    let note_id = plugin.id.clone();
    let http_id = plugin.id.clone();
    let outcome = view.test_result(&plugin.id);
    div()
        .flex()
        .items_center()
        .gap(px(6.0))
        .pt(px(6.0))
        .border_t_1()
        .border_color(border)
        .child(action_button(
            format!("plugin-test-note-{}", plugin.id),
            t_global("plugins.test_note_read"),
            false,
            cx,
            move |view, _window, cx| view.run_note_read_test(&note_id, cx),
        ))
        .child(action_button(
            format!("plugin-test-http-{}", plugin.id),
            t_global("plugins.test_http_request"),
            false,
            cx,
            move |view, _window, cx| view.run_http_request_test(&http_id, cx),
        ))
        .when(busy, |element| {
            element.child(
                div()
                    .text_size(px(10.0))
                    .text_color(muted)
                    .child(t_global("plugins.loading_plugins")),
            )
        })
        .when_some(outcome, |element, outcome| {
            let label = t_global(outcome.label_key);
            let status = if outcome.result.ok {
                t_global("plugins.installed")
            } else {
                t_global("plugins.failed_on")
            };
            let color = if outcome.result.ok { success } else { danger };
            let id = plugin.id.clone();
            element.child(
                div()
                    .id(format!("plugin-test-result-{}", plugin.id))
                    .debug_selector({
                        let id = plugin.id.clone();
                        move || format!("plugin-test-result-{id}")
                    })
                    .flex_1()
                    .text_size(px(10.0))
                    .text_color(color)
                    .cursor_pointer()
                    .child(format!("{label}: {status} — {}", outcome.result.detail))
                    .on_click(cx.listener(move |view, _, _, cx| view.clear_test(&id, cx))),
            )
        })
        .into_any_element()
}

/// The manifest hook list (provider data).
fn hook_row(hooks: &[String], cx: &mut Context<PluginsView>) -> AnyElement {
    let theme = cx.theme();
    let muted = theme.colors.muted_foreground;
    let border = theme.colors.border;
    div()
        .flex()
        .items_center()
        .flex_wrap()
        .gap(px(4.0))
        .child(
            div()
                .text_size(px(9.0))
                .text_color(muted)
                .child(t_global("plugins.hooks")),
        )
        .children(hooks.iter().map(|hook| {
            div()
                .px(px(5.0))
                .py(px(1.0))
                .rounded(px(4.0))
                .border_1()
                .border_color(border)
                .text_size(px(9.0))
                .text_color(muted)
                .child(hook.clone())
        }))
        .into_any_element()
}
