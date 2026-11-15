//! Mobile Query variant.
//!
//! Wraps the live [`QueryView`](crate::panels::query::QueryView) and pins the
//! run action to the bottom of the content column, above the navigation bar, so
//! it stays reachable while the builder/results scroll.

use gpui_kit::component::ActiveTheme;
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use crate::mobile::insets::MobileFrame;
use crate::mobile::nav::MIN_TOUCH_TARGET;
use crate::router::Panel;
use crate::shell::Shell;

/// Render the Query surface for the mobile shell.
pub fn render(shell: &Shell, _frame: &MobileFrame, cx: &mut Context<Shell>) -> AnyElement {
    let theme = cx.theme();
    let accent = theme.colors.accent;
    let accent_foreground = theme.colors.accent_foreground;

    let Some(query) = shell.query_view().cloned() else {
        return super::placeholder(Panel::Query, cx);
    };

    let run_query = query.clone();
    let run = div()
        .id("mobile-query-run")
        .debug_selector(|| "mobile-query-run".to_string())
        .flex()
        .items_center()
        .justify_center()
        .h(px(48.0))
        .min_h(px(MIN_TOUCH_TARGET))
        .mx(px(16.0))
        .mb(px(8.0))
        .rounded(px(8.0))
        .bg(accent)
        .text_color(accent_foreground)
        .text_size(px(14.0))
        .font_weight(FontWeight::SEMIBOLD)
        .cursor_pointer()
        .child(t_global("query.run_query"))
        .on_click(cx.listener(move |_shell, _, _, cx| {
            run_query.update(cx, |view, cx| view.run(cx));
            cx.notify();
        }));

    div()
        .flex()
        .flex_col()
        .size_full()
        .child(div().flex_1().overflow_hidden().child(query))
        .child(run)
        .into_any_element()
}
