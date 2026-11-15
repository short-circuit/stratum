//! Set/change/clear reminder dialog for the native Reminders panel.
//!
//! [`overlay`] reproduces the shipped `ReminderDialog`: the `datetime-local`
//! input, the quick choices, the invalid hint, the clear/cancel/save actions and
//! the provider error line. Validation and provider calls stay on
//! [`super::RemindersView`].

use gpui_kit::component::input::Input;
use gpui_kit::component::ActiveTheme;
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_app::reminders::due::local_input_to_utc_iso;
use stratum_i18n::t_global;

use super::model::QuickDue;
use super::RemindersView;

/// Render the open reminder dialog.
pub fn overlay(view: &RemindersView, cx: &mut Context<RemindersView>) -> AnyElement {
    // Copy every color before the builder calls `quick_row`/`action_row`, which
    // take `&mut Context`; the theme guard would otherwise stay borrowed.
    let theme = cx.theme();
    let scrim = theme.colors.background.opacity(0.6);
    let surface = theme.colors.secondary;
    let border = theme.colors.border;
    let foreground = theme.colors.foreground;
    let danger = theme.colors.danger;
    let Some(dialog) = view.dialog() else {
        return div().into_any_element();
    };
    let title = t_global(dialog.title_key());
    let value = dialog.value.clone();
    let error = dialog.error.clone();
    let initial = dialog.initial_due.is_some();
    let invalid = !value.trim().is_empty() && local_input_to_utc_iso(&value).is_empty();
    let input = view.dialog_input().cloned();

    div()
        .id("reminder-dialog")
        .debug_selector(|| "reminder-dialog".to_string())
        .absolute()
        .inset_0()
        .flex()
        .items_center()
        .justify_center()
        .bg(scrim)
        .child(
            div()
                .w(px(380.0))
                .p(px(16.0))
                .rounded(px(8.0))
                .bg(surface)
                .border_1()
                .border_color(border)
                .flex()
                .flex_col()
                .gap(px(10.0))
                .child(
                    div()
                        .text_size(px(14.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(foreground)
                        .child(title),
                )
                .when_some(input, |element, input| {
                    element.child(
                        div()
                            .id("reminder-dialog-input")
                            .debug_selector(|| "reminder-dialog-input".to_string())
                            .child(Input::new(&input)),
                    )
                })
                .child(quick_row(cx))
                .when(invalid, |element| {
                    element.child(
                        div()
                            .id("reminder-dialog-invalid")
                            .debug_selector(|| "reminder-dialog-invalid".to_string())
                            .text_size(px(11.0))
                            .text_color(danger)
                            .child(t_global("reminders.dialog.invalid")),
                    )
                })
                .when_some(error, |element, message| {
                    element.child(
                        div()
                            .id("reminder-dialog-error")
                            .debug_selector(|| "reminder-dialog-error".to_string())
                            .text_size(px(11.0))
                            .text_color(danger)
                            .child(message),
                    )
                })
                .child(action_row(initial, cx)),
        )
        .into_any_element()
}

/// The quick due choices.
fn quick_row(cx: &mut Context<RemindersView>) -> impl IntoElement {
    div()
        .flex()
        .flex_wrap()
        .gap(px(6.0))
        .children(QuickDue::ALL.into_iter().enumerate().map(|(index, quick)| {
            dialog_button(
                format!("reminder-dialog-quick-{index}"),
                t_global(quick.label_key()),
                false,
                cx,
                move |view, window, cx| view.dialog_apply_quick(quick, window, cx),
            )
        }))
}

/// The clear/cancel/save action row.
fn action_row(initial: bool, cx: &mut Context<RemindersView>) -> impl IntoElement {
    let theme = cx.theme();
    let muted = theme.colors.muted_foreground;
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(8.0))
        .when(initial, |element| {
            element.child(dialog_button(
                "reminder-dialog-clear",
                t_global("reminders.dialog.clear_reminder"),
                false,
                cx,
                |view, _window, cx| view.dialog_clear(cx),
            ))
        })
        .child(
            div()
                .flex()
                .gap(px(6.0))
                .child(dialog_button(
                    "reminder-dialog-cancel",
                    t_global("reminders.dialog.cancel"),
                    false,
                    cx,
                    |view, _window, cx| view.dismiss_dialog(cx),
                ))
                .child(dialog_button(
                    "reminder-dialog-save",
                    t_global("reminders.dialog.save"),
                    true,
                    cx,
                    |view, _window, cx| view.dialog_save(cx),
                )),
        )
        .child(div().text_color(muted))
}

/// One dialog action button.
fn dialog_button(
    id: impl Into<String>,
    label: String,
    primary: bool,
    cx: &mut Context<RemindersView>,
    action: impl Fn(&mut RemindersView, &mut Window, &mut Context<RemindersView>) + 'static,
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
        .px(px(12.0))
        .py(px(5.0))
        .rounded(px(6.0))
        .bg(background)
        .text_color(color)
        .text_size(px(11.0))
        .cursor_pointer()
        .child(label)
        .on_click(cx.listener(move |view, _, window, cx| action(view, window, cx)))
        .into_any_element()
}
