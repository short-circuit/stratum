//! Mobile journal variant.
//!
//! Wraps the live [`JournalView`] and replaces the desktop calendar popover with
//! a full-width bottom sheet built from the shared
//! [`calendar_body`](crate::panels::journal::calendar::calendar_body) content.

use gpui_kit::component::ActiveTheme;
use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::mobile::insets::MobileFrame;
use crate::mobile::nav::NAV_ITEM_HEIGHT;
use crate::panels::journal::calendar;
use crate::router::Panel;
use crate::shell::Shell;

/// Render the journal surface for the mobile shell.
pub fn render(shell: &Shell, frame: &MobileFrame, cx: &mut Context<Shell>) -> AnyElement {
    let theme = cx.theme();
    let background = theme.colors.background;
    let foreground = theme.colors.foreground;
    let panel = theme.colors.secondary;
    let border = theme.colors.border;

    let Some(journal) = shell.journal_view().cloned() else {
        return div()
            .flex()
            .flex_col()
            .size_full()
            .bg(background)
            .text_color(foreground)
            .child(super::placeholder(Panel::Journal, cx))
            .into_any_element();
    };

    // The mobile variant owns the calendar sheet, so suppress the desktop
    // popover inside the wrapped journal entity.
    journal.update(cx, |view, _| view.set_mobile(true));

    let calendar_open = journal.read(cx).calendar_open();
    let mut root = div()
        .relative()
        .flex()
        .flex_col()
        .size_full()
        .bg(background)
        .text_color(foreground)
        .child(div().flex_1().overflow_hidden().child(journal.clone()));

    if calendar_open {
        let body = journal.update(cx, |view, cx| {
            calendar::calendar_body(view, cx).into_any_element()
        });
        let close_journal = journal.clone();
        let scrim = div()
            .id("mobile-calendar-scrim")
            .absolute()
            .top(px(0.0))
            .left(px(0.0))
            .size_full()
            .bg(background.opacity(0.55))
            .on_click(cx.listener(move |_shell, _, _, cx| {
                close_journal.update(cx, |view, cx| view.close_calendar(cx));
                cx.notify();
            }));
        let sheet = div()
            .absolute()
            .left(px(frame.left))
            .right(px(frame.right))
            .bottom(px(frame.bottom + NAV_ITEM_HEIGHT))
            .max_h(px(560.0))
            .rounded_t(px(16.0))
            .bg(panel)
            .border_1()
            .border_color(border)
            .p(px(16.0))
            .child(body);
        root = root.child(
            div()
                .absolute()
                .top(px(0.0))
                .left(px(0.0))
                .size_full()
                .child(scrim)
                .child(sheet),
        );
    }

    root.into_any_element()
}
