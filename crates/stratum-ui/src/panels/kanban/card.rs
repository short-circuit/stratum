//! Card rendering for the native Kanban panel.
//!
//! Owns the draggable card surface, the floating drag preview, the inline
//! add-card form and the card context menu. Column placement and marker text
//! come from [`super::model`]; provider calls stay on [`super::KanbanView`].

use gpui_kit::assets::IconName;
use gpui_kit::component::input::Textarea;
use gpui_kit::component::{ActiveTheme, Icon, Sizable};
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use super::model::{self, ColumnId};
use super::widgets::{action_button, menu_item};
use super::KanbanView;
use crate::provider::panels::KanbanCardRow;

/// The payload carried while a card is dragged between columns.
#[derive(Clone)]
pub struct DraggedCard {
    /// The dragged card id.
    pub id: String,
    /// The card text, shown in the drag preview.
    pub label: String,
}

/// The floating preview shown while a card is dragged.
struct DragGhost {
    label: String,
}

impl Render for DragGhost {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        div()
            .px(px(10.0))
            .py(px(6.0))
            .rounded(px(6.0))
            .bg(theme.colors.secondary)
            .border_1()
            .border_color(theme.colors.border)
            .shadow_md()
            .text_size(px(12.0))
            .text_color(theme.colors.foreground)
            .child(self.label.clone())
    }
}

/// Render one card.
pub(super) fn card_element(
    card: &KanbanCardRow,
    _view: &KanbanView,
    cx: &mut Context<KanbanView>,
) -> AnyElement {
    let theme = cx.theme();
    let surface = theme.colors.background;
    let border = theme.colors.border;
    let muted = theme.colors.muted_foreground;
    let foreground = theme.colors.foreground;
    let id = card.id.clone();
    let debug_id = format!("kanban-card-{}", card.id);
    let content = card.content.clone();
    let marker = card.marker.clone().unwrap_or_default();
    let page_label = model::page_label(card);
    let journal = model::is_journal(card);
    let click_id = id.clone();
    let menu_id = id.clone();
    let page_id = id.clone();
    let drag_id = id.clone();
    let drag_label = content.clone();
    let drag_ghost_label = content.clone();

    div()
        .id(format!("kanban-card-{id}"))
        .debug_selector(move || debug_id.clone())
        .flex()
        .flex_col()
        .gap(px(6.0))
        .p(px(10.0))
        .rounded(px(6.0))
        .bg(surface)
        .border_1()
        .border_color(border)
        .text_color(foreground)
        .cursor_pointer()
        .role(Role::Button)
        .aria_label(content.clone())
        .child(div().text_size(px(12.5)).child(content))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .when(!marker.is_empty(), |element| {
                    element.child(
                        div()
                            .px(px(5.0))
                            .py(px(1.0))
                            .rounded(px(4.0))
                            .border_1()
                            .border_color(border)
                            .text_size(px(9.0))
                            .text_color(muted)
                            .child(marker),
                    )
                })
                .child(
                    div()
                        .id(format!("kanban-page-{page_id}"))
                        .text_size(px(9.5))
                        .text_color(muted)
                        .cursor_pointer()
                        .hover(|style| style.text_color(foreground))
                        .child(page_label)
                        .on_click(cx.listener(move |view, _, _, cx| {
                            cx.stop_propagation();
                            view.open_page(&page_id, cx);
                        })),
                )
                .when(journal, |element| {
                    element.child(
                        Icon::new(IconName::FileText)
                            .with_size(px(9.0))
                            .text_color(muted),
                    )
                }),
        )
        .on_click(cx.listener(move |view, _, window, cx| {
            view.begin_edit(&click_id, window, cx);
        }))
        .on_mouse_down(
            MouseButton::Right,
            cx.listener(move |view, _, _, cx| {
                cx.stop_propagation();
                view.open_menu(&menu_id, cx);
            }),
        )
        .on_drag(
            DraggedCard {
                id: drag_id,
                label: drag_label,
            },
            move |_dragged: &DraggedCard, _offset, _window, cx| {
                cx.new(|_| DragGhost {
                    label: drag_ghost_label.clone(),
                })
            },
        )
        .into_any_element()
}

/// Render the inline add-card form or the "Add Card" trigger.
pub(super) fn add_form(
    column: ColumnId,
    view: &KanbanView,
    cx: &mut Context<KanbanView>,
) -> AnyElement {
    let theme = cx.theme();
    let muted = theme.colors.muted_foreground;
    if view.new_card_column() != Some(column) {
        return action_button(
            format!("kanban-add-card-{}", column.slug()),
            t_global("kanban.add_card"),
            Some(IconName::Plus),
            cx,
            move |view, window, cx| view.start_new_card(column, window, cx),
        );
    }
    let empty = view.new_card_text().trim().is_empty();
    div()
        .id(format!("kanban-add-form-{}", column.slug()))
        .flex()
        .flex_col()
        .gap(px(4.0))
        .mt(px(4.0))
        .child(Textarea::new(view.new_card_input()))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(action_button(
                    format!("kanban-add-confirm-{}", column.slug()),
                    t_global("kanban.add"),
                    None,
                    cx,
                    move |view, window, cx| view.add_card(window, cx),
                ))
                .child(action_button(
                    format!("kanban-add-cancel-{}", column.slug()),
                    t_global("kanban.cancel"),
                    None,
                    cx,
                    move |view, window, cx| view.cancel_new_card(window, cx),
                ))
                .when(empty, |element| element.child(div().text_color(muted))),
        )
        .into_any_element()
}

/// Render the card context menu for a column, when one of its cards owns it.
///
/// The shipped panel anchored the MUI menu at the pointer; the native board
/// anchors it under the owning column's header so it is never clipped by the
/// scrolling card list and stays deterministic for captures.
pub(super) fn context_menu(
    card_ids: &[String],
    view: &KanbanView,
    cx: &mut Context<KanbanView>,
) -> Option<AnyElement> {
    let id = view.menu_card()?;
    if !card_ids.iter().any(|card_id| card_id == id) {
        return None;
    }
    let theme = cx.theme();
    let surface = theme.colors.background;
    let border = theme.colors.border;
    let danger = theme.colors.danger;
    let foreground = theme.colors.foreground;
    let card = view.cards().iter().find(|card| card.id == id)?;
    let label = card.content.clone();
    let edit_id = id.to_string();
    let delete_id = id.to_string();
    let menu = div()
        .id("kanban-card-menu")
        .debug_selector(|| "kanban-card-menu".to_string())
        .absolute()
        .top(px(44.0))
        .right(px(8.0))
        .w(px(180.0))
        .rounded(px(6.0))
        .bg(surface)
        .border_1()
        .border_color(border)
        .shadow_md()
        .p(px(4.0))
        .flex()
        .flex_col()
        .gap(px(1.0))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|_, _, _, cx| cx.stop_propagation()),
        )
        .on_mouse_down(
            MouseButton::Right,
            cx.listener(|_, _, _, cx| cx.stop_propagation()),
        )
        .child(
            div()
                .px(px(8.0))
                .py(px(4.0))
                .text_size(px(9.5))
                .text_color(foreground.opacity(0.6))
                .child(label),
        )
        .child(menu_item(
            "kanban-menu-edit",
            t_global("kanban.edit"),
            foreground,
            cx,
            move |view, window, cx| view.begin_edit(&edit_id, window, cx),
        ))
        .child(menu_item(
            "kanban-menu-delete",
            t_global("kanban.delete"),
            danger,
            cx,
            move |view, _window, cx| view.delete_card(&delete_id, cx),
        ));
    Some(menu.into_any_element())
}
