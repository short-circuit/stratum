//! Render helpers for the native Plugins panel header and tab chrome.
//!
//! Split out of [`super`] so the entity/action module stays under the
//! repository file-length limit. The tab bodies live in [`super::installed`],
//! [`super::directory`] and [`super::themes`]; every visible label here is a
//! catalog key from [`super::PLUGINS_I18N_KEYS`].

use gpui_kit::component::ActiveTheme;
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::{t_global, tr_global};

use super::{directory, installed, themes, PluginsTab, PluginsView};

/// Render the plugins panel body.
pub fn render(view: &PluginsView, cx: &mut Context<PluginsView>) -> impl IntoElement {
    let theme = cx.theme();
    div()
        .relative()
        .flex()
        .flex_col()
        .size_full()
        .bg(theme.colors.background)
        .text_color(theme.colors.foreground)
        .child(header(view, cx))
        .child(
            div()
                .id("plugins-body")
                .debug_selector(|| "plugins-body".to_string())
                .flex()
                .flex_col()
                .flex_1()
                .overflow_y_scroll()
                .p(px(24.0))
                .gap(px(12.0))
                .child(caption(cx))
                .child(tab_bar(view, cx))
                .when_some(panel_error(view), |element, message| {
                    element.child(error_banner(message, cx))
                })
                .child(body(view, cx)),
        )
}

/// The panel title bar with refresh and install actions.
fn header(view: &PluginsView, cx: &mut Context<PluginsView>) -> impl IntoElement {
    let theme = cx.theme();
    let installing = view.busy_id() == Some("__install__");
    let install_label = if installing {
        t_global("plugins.installing")
    } else {
        t_global("plugins.install")
    };
    div()
        .id("plugins-header")
        .debug_selector(|| "plugins-header".to_string())
        .flex()
        .items_center()
        .justify_between()
        .px(px(24.0))
        .py(px(12.0))
        .border_b_1()
        .border_color(theme.colors.border)
        .child(
            div()
                .text_size(px(18.0))
                .font_weight(FontWeight::BOLD)
                .text_color(theme.colors.foreground)
                .child(t_global("mobile_layout.plugins")),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(action_button(
                    "plugins-refresh",
                    t_global("common.refresh"),
                    false,
                    cx,
                    |view, _window, cx| view.refresh_plugins(cx),
                ))
                .child(action_button(
                    "plugins-install-file",
                    install_label,
                    true,
                    cx,
                    |view, _window, cx| view.install_from_file(cx),
                )),
        )
}

/// The shipped `~/.pkm/plugins` caption.
fn caption(cx: &mut Context<PluginsView>) -> impl IntoElement {
    let theme = cx.theme();
    div()
        .flex()
        .items_center()
        .gap(px(4.0))
        .text_size(px(11.0))
        .text_color(theme.colors.muted_foreground)
        .child(t_global("plugins.wasm_plugins_loaded_from"))
        .child(
            div()
                .px(px(4.0))
                .rounded(px(3.0))
                .bg(theme.colors.muted_foreground.opacity(0.12))
                .child(t_global("plugins._pkm_plugins")),
        )
}

/// The installed / directory / themes tab bar.
fn tab_bar(view: &PluginsView, cx: &mut Context<PluginsView>) -> impl IntoElement {
    let theme = cx.theme();
    div()
        .id("plugins-tabs")
        .debug_selector(|| "plugins-tabs".to_string())
        .flex()
        .items_center()
        .gap(px(4.0))
        .children(
            [
                PluginsTab::Installed,
                PluginsTab::Directory,
                PluginsTab::Themes,
            ]
            .into_iter()
            .map(|tab| {
                let active = view.tab() == tab;
                let label = t_global(tab.label_key());
                let (background, color) = if active {
                    (theme.colors.accent, theme.colors.accent_foreground)
                } else {
                    (
                        theme.colors.muted_foreground.opacity(0.12),
                        theme.colors.foreground,
                    )
                };
                div()
                    .id(format!("plugins-tab-{}", tab.label_key()))
                    .debug_selector(move || format!("plugins-tab-{}", tab.label_key()))
                    .px(px(10.0))
                    .py(px(4.0))
                    .rounded(px(5.0))
                    .bg(background)
                    .text_color(color)
                    .text_size(px(11.0))
                    .cursor_pointer()
                    .child(label)
                    .on_click(cx.listener(move |view, _, _, cx| view.set_tab(tab, cx)))
            }),
        )
}

/// Render the active tab body.
fn body(view: &PluginsView, cx: &mut Context<PluginsView>) -> AnyElement {
    match view.tab() {
        PluginsTab::Installed => installed::render(view, cx).into_any_element(),
        PluginsTab::Directory => directory::render(view, cx).into_any_element(),
        PluginsTab::Themes => themes::render(view, cx).into_any_element(),
    }
}

/// The first provider error the panel wants to surface.
fn panel_error(view: &PluginsView) -> Option<String> {
    match view.tab() {
        PluginsTab::Installed => view.load_error().map(str::to_string),
        PluginsTab::Directory => view.catalog_error().map(str::to_string),
        PluginsTab::Themes => view.themes_error().map(str::to_string),
    }
}

/// A click-to-dismiss provider error banner.
fn error_banner(message: String, cx: &mut Context<PluginsView>) -> AnyElement {
    let theme = cx.theme();
    div()
        .id("plugins-error")
        .debug_selector(|| "plugins-error".to_string())
        .flex()
        .items_center()
        .justify_between()
        .gap(px(8.0))
        .text_size(px(11.0))
        .text_color(theme.colors.danger)
        .cursor_pointer()
        .child(message)
        .child(
            div()
                .text_size(px(10.0))
                .text_color(theme.colors.muted_foreground)
                .child(t_global("plugins.click_to_dismiss")),
        )
        .on_click(cx.listener(|view, _, _, cx| view.clear_error(cx)))
        .into_any_element()
}

/// A version pill (`v` + provider version).
pub(super) fn version_chip(version: &str, cx: &mut Context<PluginsView>) -> AnyElement {
    let theme = cx.theme();
    div()
        .px(px(6.0))
        .py(px(1.0))
        .rounded(px(4.0))
        .border_1()
        .border_color(theme.colors.border)
        .text_size(px(9.0))
        .text_color(theme.colors.muted_foreground)
        .child(format!("v{version}"))
        .into_any_element()
}

/// Permission pills (provider data).
pub(super) fn permission_chips(
    permissions: &[String],
    cx: &mut Context<PluginsView>,
) -> AnyElement {
    let theme = cx.theme();
    div()
        .flex()
        .flex_wrap()
        .gap(px(3.0))
        .children(permissions.iter().map(|permission| {
            div()
                .px(px(5.0))
                .py(px(1.0))
                .rounded(px(4.0))
                .bg(theme.colors.muted_foreground.opacity(0.12))
                .text_size(px(9.0))
                .text_color(theme.colors.muted_foreground)
                .child(permission.clone())
        }))
        .into_any_element()
}

/// A localized author line using the existing `plugins.by_author` key.
pub(super) fn author_line(author: &str, cx: &mut Context<PluginsView>) -> AnyElement {
    let theme = cx.theme();
    div()
        .text_size(px(10.0))
        .text_color(theme.colors.muted_foreground)
        .child(tr_global("plugins.by_author", &[("author", author)]))
        .into_any_element()
}

/// A small colored status badge.
pub(super) fn badge(
    label: String,
    color: Hsla,
    background: Hsla,
    cx: &mut Context<PluginsView>,
) -> AnyElement {
    let _ = cx;
    div()
        .px(px(6.0))
        .py(px(1.0))
        .rounded(px(4.0))
        .bg(background)
        .text_color(color)
        .text_size(px(9.0))
        .font_weight(FontWeight::SEMIBOLD)
        .child(label)
        .into_any_element()
}

/// A clickable action pill used by the header and the tab bodies.
pub(super) fn action_button(
    id: impl Into<String>,
    label: String,
    primary: bool,
    cx: &mut Context<PluginsView>,
    action: impl Fn(&mut PluginsView, &mut Window, &mut Context<PluginsView>) + 'static,
) -> AnyElement {
    let theme = cx.theme();
    let id = id.into();
    let debug_id = id.clone();
    let (background, color) = if primary {
        (theme.colors.accent, theme.colors.accent_foreground)
    } else {
        (
            theme.colors.muted_foreground.opacity(0.15),
            theme.colors.foreground,
        )
    };
    div()
        .id(id)
        .debug_selector(move || debug_id.clone())
        .flex()
        .items_center()
        .justify_center()
        .px(px(9.0))
        .py(px(3.0))
        .rounded(px(5.0))
        .bg(background)
        .text_color(color)
        .text_size(px(10.0))
        .cursor_pointer()
        .child(label)
        .on_click(cx.listener(move |view, _, window, cx| action(view, window, cx)))
        .into_any_element()
}
