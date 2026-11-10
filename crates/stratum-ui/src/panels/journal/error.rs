//! Journal error card with retry and repair actions.
//!
//! Mirrors the shipped `Alert` + outlined buttons in the desktop journal panel:
//! the localized failure message, a retry that re-runs the ensure-today flow and
//! a repair that reconciles the block database from disk first. Both actions are
//! owned by [`super::JournalView`]; this module only renders the controls.

use gpui_kit::component::ActiveTheme;
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use super::JournalView;

/// Render the localized journal error card.
pub fn error_card(view: &JournalView, cx: &mut Context<JournalView>) -> impl IntoElement {
    let theme = cx.theme();
    let background = theme.colors.secondary;
    let danger = theme.colors.danger;
    let muted = theme.colors.muted_foreground;
    let accent = theme.colors.accent;
    let accent_foreground = theme.colors.accent_foreground;

    let message = view.error().unwrap_or_default().to_string();

    div()
        .mx(px(12.0))
        .my(px(12.0))
        .p(px(12.0))
        .rounded(px(8.0))
        .bg(background)
        .border_1()
        .border_color(danger)
        .flex()
        .flex_col()
        .gap(px(8.0))
        .child(div().text_size(px(13.0)).text_color(danger).child(message))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(action_button(
                    "journal-retry",
                    t_global("common.retry"),
                    accent,
                    accent_foreground,
                    cx,
                    |view, cx| view.retry(cx),
                ))
                .child(action_button(
                    "journal-repair",
                    t_global("journal.repair_database"),
                    muted.opacity(0.15),
                    muted,
                    cx,
                    |view, cx| view.repair(cx),
                )),
        )
}

/// One action button with a background and text colour.
fn action_button(
    id: &'static str,
    label: String,
    background: Hsla,
    color: Hsla,
    cx: &mut Context<JournalView>,
    action: impl Fn(&mut JournalView, &mut Context<JournalView>) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .px(px(10.0))
        .py(px(5.0))
        .rounded(px(6.0))
        .bg(background)
        .text_color(color)
        .text_size(px(12.0))
        .cursor_pointer()
        .child(label)
        .on_click(cx.listener(move |view, _, _, cx| action(view, cx)))
}
