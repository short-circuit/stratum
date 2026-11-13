//! Sidebar footer: refresh, export and version controls.
//!
//! Mirrors `src/components/Sidebar/SidebarFooter.tsx`. In the collapsed rail
//! the two actions become icon buttons stacked vertically; expanded, they are
//! labelled buttons beside the application version.

use gpui_kit::assets::IconName;
use gpui_kit::component::{ActiveTheme, Icon, Sizable};
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use crate::shell::Shell;

/// Render the sidebar footer.
pub fn footer(shell: &Shell, cx: &mut Context<Shell>) -> impl IntoElement {
    let theme = cx.theme();
    let border = theme.colors.border;
    let muted = theme.colors.muted_foreground;
    let foreground = theme.colors.foreground;
    let collapsed = shell.sidebar_collapsed();

    let mut footer = div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(4.0))
        .border_t_1()
        .border_color(border)
        .px(px(if collapsed { 4.0 } else { 12.0 }))
        .py(px(if collapsed { 6.0 } else { 8.0 }));

    if collapsed {
        footer = footer
            .flex_col()
            .child(icon_button(
                "sidebar-refresh",
                IconName::RefreshCw,
                muted,
                cx.listener(|this, _, _, cx| {
                    this.refresh();
                    cx.notify();
                }),
            ))
            .child(icon_button(
                "sidebar-export",
                IconName::Upload,
                muted,
                cx.listener(|this, _, _, cx| {
                    this.export();
                    cx.notify();
                }),
            ));
    } else {
        footer = footer
            .child(text_button(
                "sidebar-refresh",
                IconName::RefreshCw,
                t_global("sidebar.refresh"),
                muted,
                cx.listener(|this, _, _, cx| {
                    this.refresh();
                    cx.notify();
                }),
            ))
            .child(text_button(
                "sidebar-export",
                IconName::Upload,
                t_global("sidebar.export"),
                muted,
                cx.listener(|this, _, _, cx| {
                    this.export();
                    cx.notify();
                }),
            ))
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(foreground)
                    .child(format!("v{}", env!("CARGO_PKG_VERSION"))),
            );
    }

    footer
}

/// A compact icon-only action used by the collapsed rail.
fn icon_button(
    id: &'static str,
    icon: IconName,
    color: gpui::Hsla,
    listener: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .size(px(28.0))
        .rounded(px(4.0))
        .cursor_pointer()
        .on_click(listener)
        .child(Icon::new(icon).with_size(px(18.0)).text_color(color))
}

/// A labelled action used by the expanded footer.
fn text_button(
    id: &'static str,
    icon: IconName,
    label: String,
    color: gpui::Hsla,
    listener: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .flex()
        .items_center()
        .gap(px(4.0))
        .px(px(6.0))
        .py(px(4.0))
        .rounded(px(4.0))
        .cursor_pointer()
        .text_color(color)
        .on_click(listener)
        .child(Icon::new(icon).with_size(px(16.0)).text_color(color))
        .child(div().text_size(px(12.0)).child(label))
}
