//! Shell chrome elements: content header, error bar and success toaster.
//!
//! These render helpers are split from [`super`] so the shell state machine and
//! its `Render` implementation stay in one readable file.

use gpui_kit::assets::IconName;
use gpui_kit::component::{ActiveTheme, Icon, Sizable};
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use crate::router::Panel;

use super::Shell;

/// The content header. It mirrors the React page header: the localized panel
/// title on the left and the read-mode control on the right.
pub(crate) fn header(shell: &Shell, cx: &mut Context<Shell>) -> impl IntoElement {
    let theme = cx.theme();
    let background = theme.colors.secondary;
    let border = theme.colors.border;
    let foreground = theme.colors.foreground;
    let muted = theme.colors.muted_foreground;
    let accent = theme.colors.accent;
    let accent_foreground = theme.colors.accent_foreground;

    let title = t_global(shell.active.title_key());
    let subtitle = shell.current_page().map(str::to_string);
    let read_label = if shell.read_mode() {
        t_global("page.edit_mode")
    } else {
        t_global("page.read_mode")
    };

    div()
        .flex()
        .items_center()
        .justify_between()
        .h(px(56.0))
        .px(px(20.0))
        .bg(background)
        .border_b_1()
        .border_color(border)
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(2.0))
                .child(
                    div()
                        .text_size(px(18.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(foreground)
                        .child(title),
                )
                .when_some(subtitle, |element, path| {
                    element.child(div().text_size(px(12.0)).text_color(muted).child(path))
                }),
        )
        .when(shell.active_panel() == Panel::Page, |element| {
            element.child(
                div()
                    .id("header-read-mode")
                    .px(px(12.0))
                    .py(px(6.0))
                    .rounded(px(6.0))
                    .bg(accent)
                    .text_color(accent_foreground)
                    .text_size(px(12.0))
                    .font_weight(FontWeight::MEDIUM)
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.toggle_read_mode();
                        cx.notify();
                    }))
                    .child(read_label),
            )
        })
}

/// The error bar shown below the header. Dismissing it is also bound to Escape.
pub(crate) fn error_bar(shell: &Shell, cx: &mut Context<Shell>) -> impl IntoElement {
    let theme = cx.theme();
    let danger = theme.colors.danger;
    let border = theme.colors.border;

    div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(12.0))
        .px(px(20.0))
        .py(px(10.0))
        .bg(danger)
        .text_color(gpui::white())
        .border_b_1()
        .border_color(border)
        .child(
            div()
                .flex_1()
                .text_size(px(13.0))
                .child(shell.error().unwrap_or_default().to_string()),
        )
        .child(
            div()
                .id("error-dismiss")
                .flex()
                .items_center()
                .justify_center()
                .size(px(22.0))
                .rounded(px(4.0))
                .cursor_pointer()
                .on_click(cx.listener(|this, _, _, cx| {
                    this.dismiss_overlay();
                    cx.notify();
                }))
                .child(Icon::new(IconName::X).with_size(px(16.0))),
        )
}

/// The success toaster in the lower-right corner.
pub(crate) fn toast_overlay(shell: &Shell, cx: &mut Context<Shell>) -> impl IntoElement {
    let theme = cx.theme();
    let background = theme.colors.secondary;
    let border = theme.colors.border;
    let foreground = theme.colors.foreground;
    let muted = theme.colors.muted_foreground;

    div()
        .absolute()
        .right(px(16.0))
        .bottom(px(40.0))
        .flex()
        .items_center()
        .gap(px(12.0))
        .w(px(360.0))
        .px(px(16.0))
        .py(px(12.0))
        .rounded(px(8.0))
        .bg(background)
        .border_1()
        .border_color(border)
        .child(
            div()
                .flex_1()
                .text_size(px(13.0))
                .text_color(foreground)
                .child(shell.toast_message().unwrap_or_default().to_string()),
        )
        .child(
            div()
                .id("toast-dismiss")
                .flex()
                .items_center()
                .justify_center()
                .size(px(22.0))
                .rounded(px(4.0))
                .cursor_pointer()
                .on_click(cx.listener(|this, _, _, cx| {
                    this.toast = None;
                    cx.notify();
                }))
                .child(Icon::new(IconName::X).with_size(px(16.0)).text_color(muted)),
        )
}
