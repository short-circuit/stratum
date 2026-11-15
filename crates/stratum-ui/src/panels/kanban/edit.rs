//! Edit dialog for the native Kanban panel.
//!
//! [`overlay`] reproduces the shipped `KanbanEditDialog`: the card content
//! editor, the marker and priority toggle rows, the source line, and the
//! cancel/save actions plus the provider error line. Validation and provider
//! calls stay on [`super::KanbanView`].

use gpui_kit::component::input::Textarea;
use gpui_kit::component::ActiveTheme;
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use super::model;
use super::KanbanView;
use crate::provider::panels::KanbanCardRow;

/// Markers the dialog offers, in the shipped order.
pub const MARKERS: [&str; 7] = [
    "TODO",
    "DOING",
    "DONE",
    "NOW",
    "LATER",
    "WAITING",
    "CANCELLED",
];

/// Priorities the dialog offers.
pub const PRIORITIES: [&str; 3] = ["A", "B", "C"];

/// The open edit dialog's state.
#[derive(Clone, Debug)]
pub struct EditState {
    /// The card being edited; its `content` tracks the input.
    pub card: KanbanCardRow,
    /// The selected marker (`None` clears it).
    pub marker: Option<String>,
    /// The selected priority (`None` clears it).
    pub priority: Option<String>,
    /// The provider error from the last save attempt.
    pub error: Option<String>,
}

impl EditState {
    /// Open the dialog for `card`, seeding the marker and priority selections.
    pub fn new(card: KanbanCardRow) -> Self {
        Self {
            marker: card.marker.clone(),
            priority: card.priority.clone(),
            card,
            error: None,
        }
    }
}

/// Render the open edit dialog.
pub fn overlay(view: &KanbanView, cx: &mut Context<KanbanView>) -> AnyElement {
    let theme = cx.theme();
    let scrim = theme.colors.background.opacity(0.6);
    let surface = theme.colors.secondary;
    let border = theme.colors.border;
    let foreground = theme.colors.foreground;
    let muted = theme.colors.muted_foreground;
    let danger = theme.colors.danger;
    let Some(editing) = view.editing() else {
        return div().into_any_element();
    };
    let source = model::page_label(&editing.card);
    let error = editing.error.clone();
    let input = view.edit_input().clone();

    div()
        .id("kanban-edit-dialog")
        .debug_selector(|| "kanban-edit-dialog".to_string())
        .absolute()
        .inset_0()
        .flex()
        .items_center()
        .justify_center()
        .bg(scrim)
        .child(
            div()
                .w(px(420.0))
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
                        .child(t_global("kanban_edit_dialog.edit_card")),
                )
                .when_some(Some(input), |element, input| {
                    element.child(
                        div()
                            .id("kanban-edit-content")
                            .debug_selector(|| "kanban-edit-content".to_string())
                            .child(Textarea::new(&input)),
                    )
                })
                .child(toggle_row(
                    "kanban-edit-marker",
                    t_global("kanban_edit_dialog.marker"),
                    MARKERS
                        .iter()
                        .map(|marker| (marker.to_string(), marker.to_string()))
                        .collect(),
                    editing.marker.clone(),
                    cx,
                    |view, value, cx| {
                        // The shipped ToggleButtonGroup clears the marker when the
                        // selected button is clicked again.
                        let selected = view.editing().and_then(|state| state.marker.clone());
                        if selected.as_deref() == Some(value.as_str()) {
                            view.select_marker(None, cx);
                        } else {
                            view.select_marker(Some(&value), cx);
                        }
                    },
                ))
                .child(toggle_row(
                    "kanban-edit-priority",
                    t_global("kanban_edit_dialog.priority"),
                    std::iter::once((String::new(), t_global("kanban_edit_dialog.none")))
                        .chain(
                            PRIORITIES
                                .iter()
                                .map(|priority| (priority.to_string(), priority.to_string())),
                        )
                        .collect(),
                    editing.priority.clone(),
                    cx,
                    |view, value, cx| {
                        if value.is_empty() {
                            view.select_priority(None, cx);
                        } else {
                            view.select_priority(Some(&value), cx);
                        }
                    },
                ))
                .when_some(error, |element, message| {
                    element.child(
                        div()
                            .id("kanban-edit-error")
                            .debug_selector(|| "kanban-edit-error".to_string())
                            .text_size(px(11.0))
                            .text_color(danger)
                            .child(message),
                    )
                })
                .child(
                    div()
                        .text_size(px(10.0))
                        .text_color(muted)
                        .child(format!("{} {source}", t_global("flashcards.source"))),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_end()
                        .gap(px(6.0))
                        .child(dialog_button(
                            "kanban-edit-cancel",
                            t_global("kanban_edit_dialog.cancel"),
                            false,
                            cx,
                            |view, _window, cx| view.cancel_edit(cx),
                        ))
                        .child(dialog_button(
                            "kanban-edit-save",
                            t_global("kanban_edit_dialog.save"),
                            true,
                            cx,
                            |view, _window, cx| view.save_edit(cx),
                        )),
                ),
        )
        .into_any_element()
}

/// Render one labelled toggle row.
///
/// `options` pairs each stored value with its display label; `selected` is the
/// current value (`None` clears the selection).
fn toggle_row(
    id: &str,
    label: String,
    options: Vec<(String, String)>,
    selected: Option<String>,
    cx: &mut Context<KanbanView>,
    action: impl Fn(&mut KanbanView, String, &mut Context<KanbanView>) + Clone + 'static,
) -> AnyElement {
    let theme = cx.theme();
    let muted = theme.colors.muted_foreground;
    let border = theme.colors.border;
    let foreground = theme.colors.foreground;
    let accent = theme.colors.accent;
    let accent_foreground = theme.colors.accent_foreground;
    let selected = selected.unwrap_or_default();
    let row_id = format!("{id}-row");
    div()
        .id(row_id)
        .flex()
        .flex_col()
        .gap(px(4.0))
        .child(div().text_size(px(10.5)).text_color(muted).child(label))
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap(px(4.0))
                .children(
                    options
                        .into_iter()
                        .enumerate()
                        .map(|(index, (value, text))| {
                            let active = selected == value;
                            let click_value = value.clone();
                            let action = action.clone();
                            div()
                                .id(format!("{id}-{index}"))
                                .debug_selector({
                                    let id = format!("{id}-{index}");
                                    move || id.clone()
                                })
                                .px(px(7.0))
                                .py(px(3.0))
                                .rounded(px(4.0))
                                .border_1()
                                .border_color(if active { accent } else { border })
                                .bg(if active { accent } else { border.opacity(0.25) })
                                .text_size(px(10.0))
                                .text_color(if active {
                                    accent_foreground
                                } else {
                                    foreground
                                })
                                .cursor_pointer()
                                .role(Role::Button)
                                .aria_label(text.clone())
                                .child(text)
                                .on_click(cx.listener(move |view, _, _, cx| {
                                    cx.stop_propagation();
                                    action(view, click_value.clone(), cx);
                                }))
                        }),
                ),
        )
        .into_any_element()
}

/// One dialog action button.
fn dialog_button(
    id: impl Into<String>,
    label: String,
    primary: bool,
    cx: &mut Context<KanbanView>,
    action: impl Fn(&mut KanbanView, &mut Window, &mut Context<KanbanView>) + 'static,
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
        .role(Role::Button)
        .aria_label(label.clone())
        .child(label)
        .on_click(cx.listener(move |view, _, window, cx| {
            cx.stop_propagation();
            action(view, window, cx);
        }))
        .into_any_element()
}
