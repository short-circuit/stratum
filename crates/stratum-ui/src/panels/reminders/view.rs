//! Render helpers for the native Reminders panel.
//!
//! Split out of [`super`] so the entity/action module stays under the
//! repository file-length limit. These functions only read the panel's public
//! accessors and dispatch actions through its methods; every visible label is a
//! catalog key from [`super::REMINDERS_I18N_KEYS`].

use gpui_kit::component::ActiveTheme;
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use super::model::ReminderGroups;
use super::{dialog, failure, RemindersView};

/// Render the reminders panel body.
pub fn render(view: &RemindersView, cx: &mut Context<RemindersView>) -> impl IntoElement {
    let theme = cx.theme();
    let groups = view.groups();
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
                .id("reminders-body")
                .debug_selector(|| "reminders-body".to_string())
                .flex()
                .flex_col()
                .flex_1()
                .overflow_y_scroll()
                .p(px(24.0))
                .gap(px(14.0))
                .when_some(view.load_error().map(str::to_string), |element, error| {
                    element.child(error_banner(failure(&error), cx))
                })
                .child(body(view, &groups, cx)),
        )
        .when(view.dialog_open(), |element| {
            element.child(dialog::overlay(view, cx))
        })
}

/// The panel title bar with the shipped "Due" refresh action.
fn header(_view: &RemindersView, cx: &mut Context<RemindersView>) -> impl IntoElement {
    let theme = cx.theme();
    div()
        .id("reminders-header")
        .debug_selector(|| "reminders-header".to_string())
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
                .child(t_global("reminders.panel.title")),
        )
        .child(action_chip(
            "reminders-refresh",
            t_global("reminders.panel.due"),
            theme.colors.accent,
            theme.colors.accent_foreground,
            cx,
            |view, _window, cx| view.refresh(cx),
        ))
}

/// Render the grouped sections, the empty state or the load failure.
fn body(
    view: &RemindersView,
    groups: &ReminderGroups,
    cx: &mut Context<RemindersView>,
) -> AnyElement {
    if view.load_error().is_some() && view.rows().is_empty() {
        return error_banner(failure(view.load_error().unwrap_or_default()), cx).into_any_element();
    }
    if groups.total() == 0 {
        return empty_state(cx).into_any_element();
    }
    // Copy the colors out of the theme guard first: `section` takes `&mut Context`
    // and the guard borrows `cx`, so holding it across the calls would conflict.
    let theme = cx.theme();
    let danger = theme.colors.danger;
    let warning = theme.colors.warning;
    let muted = theme.colors.muted_foreground;
    div()
        .flex()
        .flex_col()
        .gap(px(14.0))
        .child(section(
            "reminder-section-overdue",
            t_global("reminders.panel.overdue"),
            &groups.overdue,
            danger,
            view,
            cx,
        ))
        .child(section(
            "reminder-section-today",
            t_global("reminders.panel.today"),
            &groups.today,
            warning,
            view,
            cx,
        ))
        .child(section(
            "reminder-section-upcoming",
            t_global("reminders.panel.upcoming"),
            &groups.upcoming,
            muted,
            view,
            cx,
        ))
        .into_any_element()
}

/// Render one urgency section; empty sections are omitted like the shipped panel.
fn section(
    id: &'static str,
    title: String,
    indices: &[usize],
    color: Hsla,
    view: &RemindersView,
    cx: &mut Context<RemindersView>,
) -> AnyElement {
    if indices.is_empty() {
        return div().into_any_element();
    }
    // The row builder needs `&mut Context`; keep only copied colors alive here.
    let theme = cx.theme();
    let border = theme.colors.border;
    div()
        .id(id)
        .debug_selector(move || id.to_string())
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(
            div()
                .text_size(px(12.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(color)
                .child(format!("{title} ({})", indices.len())),
        )
        .children(indices.iter().map(|index| {
            div()
                .border_b_1()
                .border_color(border)
                .child(row(*index, view, cx))
        }))
        .into_any_element()
}

/// Render one reminder row with its action chips.
fn row(index: usize, view: &RemindersView, cx: &mut Context<RemindersView>) -> AnyElement {
    let theme = cx.theme();
    let foreground = theme.colors.foreground;
    let muted = theme.colors.muted_foreground;
    let accent = theme.colors.accent;
    let accent_foreground = theme.colors.accent_foreground;
    let danger = theme.colors.danger;
    let Some(reminder) = view.rows().get(index) else {
        return div().into_any_element();
    };
    let subtitle = super::model::row_subtitle(reminder, view.now());
    div()
        .id(format!("reminder-row-{index}"))
        .debug_selector(move || format!("reminder-row-{index}"))
        .flex()
        .items_center()
        .justify_between()
        .gap(px(12.0))
        .py(px(6.0))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(2.0))
                .child(
                    div()
                        .text_size(px(13.0))
                        .text_color(foreground)
                        .child(reminder.content.clone()),
                )
                .child(div().text_size(px(11.0)).text_color(muted).child(subtitle)),
        )
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap(px(4.0))
                .child(action_chip(
                    format!("reminder-row-{index}-snooze10"),
                    t_global("reminders.panel.snooze_10min"),
                    muted.opacity(0.18),
                    foreground,
                    cx,
                    move |view, _window, cx| view.snooze(index, 10, cx),
                ))
                .child(action_chip(
                    format!("reminder-row-{index}-snooze60"),
                    t_global("reminders.panel.snooze_1h"),
                    muted.opacity(0.18),
                    foreground,
                    cx,
                    move |view, _window, cx| view.snooze(index, 60, cx),
                ))
                .child(action_chip(
                    format!("reminder-row-{index}-done"),
                    t_global("reminders.panel.complete"),
                    accent,
                    accent_foreground,
                    cx,
                    move |view, _window, cx| view.complete(index, cx),
                ))
                .child(action_chip(
                    format!("reminder-row-{index}-open"),
                    t_global("reminders.panel.open"),
                    accent,
                    accent_foreground,
                    cx,
                    move |view, _window, cx| view.open_row(index, cx),
                ))
                .child(action_chip(
                    format!("reminder-row-{index}-dialog"),
                    t_global("reminders.chip.title"),
                    muted.opacity(0.18),
                    danger,
                    cx,
                    move |view, window, cx| view.open_dialog(index, window, cx),
                )),
        )
        .into_any_element()
}

/// The shipped empty-state hint with the refresh action.
fn empty_state(cx: &mut Context<RemindersView>) -> AnyElement {
    let theme = cx.theme();
    let muted = theme.colors.muted_foreground;
    div()
        .id("reminders-empty")
        .debug_selector(|| "reminders-empty".to_string())
        .flex()
        .flex_col()
        .gap(px(8.0))
        .child(
            div()
                .text_size(px(12.0))
                .text_color(muted)
                .child(t_global("reminders.panel.empty")),
        )
        .child(action_chip(
            "reminders-empty-refresh",
            t_global("reminders.panel.due"),
            theme.colors.accent,
            theme.colors.accent_foreground,
            cx,
            |view, _window, cx| view.refresh(cx),
        ))
        .into_any_element()
}

/// A provider failure line.
fn error_banner(message: String, cx: &mut Context<RemindersView>) -> AnyElement {
    let theme = cx.theme();
    div()
        .id("reminders-error")
        .debug_selector(|| "reminders-error".to_string())
        .text_size(px(12.0))
        .text_color(theme.colors.danger)
        .child(message)
        .into_any_element()
}

/// A clickable pill action.
///
/// The listener receives the window so the due-dialog chip can create its input
/// entity; actions that ignore the window use the same signature.
fn action_chip(
    id: impl Into<String>,
    label: String,
    background: Hsla,
    color: Hsla,
    cx: &mut Context<RemindersView>,
    action: impl Fn(&mut RemindersView, &mut Window, &mut Context<RemindersView>) + 'static,
) -> AnyElement {
    let id = id.into();
    let debug_id = id.clone();
    div()
        .id(id)
        .debug_selector(move || debug_id.clone())
        .px(px(8.0))
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
