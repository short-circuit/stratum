//! Journal header row.
//!
//! Mirrors the shipped desktop journal header: previous/next day arrows around
//! a clickable localized date that opens the calendar popup, a read-mode toggle
//! and the shared voice-capture control. Rendering is thin; every action lives
//! on [`super::JournalView`], so the same state machine is exercised by the
//! window-free contract tests.

use gpui_kit::assets::IconName;
use gpui_kit::component::{ActiveTheme, Icon, Sizable};
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use super::JournalView;

/// Render the journal header row.
pub fn header(view: &JournalView, cx: &mut Context<JournalView>) -> impl IntoElement {
    let theme = cx.theme();
    let border = theme.colors.border;
    let muted = theme.colors.muted_foreground;
    let accent = theme.colors.accent;
    let accent_foreground = theme.colors.accent_foreground;

    div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(8.0))
        .px(px(12.0))
        .py(px(8.0))
        .border_b_1()
        .border_color(border)
        .child(day_navigation(view, muted, accent, cx))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(read_mode_button(view, accent, accent_foreground, muted, cx))
                .child(calendar_button(muted, cx))
                .child(view.voice().clone()),
        )
}

/// Previous arrow, clickable date and next arrow.
fn day_navigation(
    view: &JournalView,
    muted: Hsla,
    accent: Hsla,
    cx: &mut Context<JournalView>,
) -> impl IntoElement {
    let label = super::dates::display_date(view.target_date());
    div()
        .flex()
        .items_center()
        .gap(px(6.0))
        .child(icon_button(
            "journal-previous-day",
            IconName::ChevronLeft,
            muted,
            cx,
            |view, cx| view.previous_day(cx),
        ))
        .child(
            div()
                .id("journal-date")
                .px(px(6.0))
                .py(px(2.0))
                .rounded(px(4.0))
                .text_size(px(13.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(accent)
                .cursor_pointer()
                .child(label)
                .on_click(cx.listener(|view, _, _, cx| view.toggle_calendar(cx))),
        )
        .child(icon_button(
            "journal-next-day",
            IconName::ChevronRight,
            muted,
            cx,
            |view, cx| view.next_day(cx),
        ))
}

/// Read-mode toggle, mirroring the shipped prose button.
fn read_mode_button(
    view: &JournalView,
    accent: Hsla,
    accent_foreground: Hsla,
    muted: Hsla,
    cx: &mut Context<JournalView>,
) -> impl IntoElement {
    let read_mode = view.read_mode();
    let label = if read_mode {
        t_global("page.edit_mode")
    } else {
        t_global("page.read_mode")
    };
    div()
        .id("journal-read-mode")
        .flex()
        .items_center()
        .gap(px(4.0))
        .px(px(8.0))
        .py(px(4.0))
        .rounded(px(6.0))
        .bg(if read_mode {
            accent
        } else {
            muted.opacity(0.15)
        })
        .text_color(if read_mode { accent_foreground } else { muted })
        .text_size(px(12.0))
        .cursor_pointer()
        .child(Icon::new(IconName::BookOpen).with_size(px(14.0)))
        .child(div().child(label))
        .on_click(cx.listener(|view, _, _, cx| {
            view.set_read_mode(!view.read_mode(), cx);
        }))
}

/// Calendar popup trigger.
fn calendar_button(muted: Hsla, cx: &mut Context<JournalView>) -> impl IntoElement {
    div()
        .id("journal-calendar-button")
        .flex()
        .items_center()
        .gap(px(4.0))
        .px(px(8.0))
        .py(px(4.0))
        .rounded(px(6.0))
        .bg(muted.opacity(0.15))
        .text_color(muted)
        .text_size(px(12.0))
        .cursor_pointer()
        .child(Icon::new(IconName::Calendar).with_size(px(14.0)))
        .child(div().child(t_global("journal_calendar.calendar")))
        .on_click(cx.listener(|view, _, _, cx| view.toggle_calendar(cx)))
}

/// A square icon button with a click handler.
fn icon_button(
    id: &'static str,
    icon: IconName,
    color: Hsla,
    cx: &mut Context<JournalView>,
    action: impl Fn(&mut JournalView, &mut Context<JournalView>) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .size(px(24.0))
        .rounded(px(4.0))
        .cursor_pointer()
        .on_click(cx.listener(move |view, _, _, cx| action(view, cx)))
        .child(Icon::new(icon).with_size(px(16.0)).text_color(color))
}
