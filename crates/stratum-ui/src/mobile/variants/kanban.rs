//! Mobile Kanban variant.
//!
//! The native shell does not host a live Kanban entity yet, so the mobile
//! surface renders the shipped empty-state placeholder (board title and the
//! "no tasks yet" hint) inside the safe-area frame.

use gpui_kit::assets::IconName;
use gpui_kit::component::{ActiveTheme, Icon, Sizable};
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use crate::mobile::insets::MobileFrame;
use crate::shell::Shell;

/// Render the Kanban placeholder for the mobile shell.
pub fn render(_shell: &Shell, _frame: &MobileFrame, cx: &mut Context<Shell>) -> AnyElement {
    let theme = cx.theme();
    let background = theme.colors.background;
    let foreground = theme.colors.foreground;
    let muted = theme.colors.muted_foreground;

    div()
        .flex()
        .flex_col()
        .size_full()
        .bg(background)
        .text_color(foreground)
        .child(
            div()
                .flex()
                .flex_col()
                .flex_1()
                .items_center()
                .justify_center()
                .gap(px(12.0))
                .px(px(24.0))
                .child(
                    Icon::new(IconName::ListChecks)
                        .with_size(px(48.0))
                        .text_color(muted),
                )
                .child(
                    div()
                        .text_size(px(18.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(t_global("kanban.kanban")),
                )
                .child(
                    div()
                        .text_size(px(13.0))
                        .text_color(muted)
                        .child(t_global("kanban.no_tasks_yet")),
                ),
        )
        .into_any_element()
}
