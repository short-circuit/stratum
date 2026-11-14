//! Board chrome for the native Kanban panel.
//!
//! Renders the header (title, Today's Journal, Refresh), the error/message bar
//! and the three columns with their cards. Column placement, page labels and
//! the target marker live in [`super::model`]; card bodies and the context menu
//! live in [`super::card`]; provider calls stay on [`super::KanbanView`].

use gpui_kit::assets::IconName;
use gpui_kit::component::ActiveTheme;
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use super::card::{add_form, card_element, context_menu, DraggedCard};
use super::model::{self, ColumnId};
use super::widgets::{action_button, icon_button};
use super::KanbanView;
use crate::provider::panels::KanbanCardRow;

/// Render the panel header.
pub fn header(_view: &KanbanView, cx: &mut Context<KanbanView>) -> AnyElement {
    let theme = cx.theme();
    let border = theme.colors.border;
    let foreground = theme.colors.foreground;
    div()
        .id("kanban-header")
        .debug_selector(|| "kanban-header".to_string())
        .flex()
        .items_center()
        .justify_between()
        .px(px(16.0))
        .py(px(10.0))
        .border_b_1()
        .border_color(border)
        .child(
            div()
                .text_size(px(16.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(foreground)
                .child(t_global("kanban.kanban")),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(action_button(
                    "kanban-today-journal",
                    t_global("kanban.today_apos_s_journal"),
                    None,
                    cx,
                    |view, _window, cx| view.open_today_journal(cx),
                ))
                .child(action_button(
                    "kanban-refresh",
                    t_global("kanban.refresh"),
                    Some(IconName::RefreshCw),
                    cx,
                    |view, _window, cx| view.refresh(cx),
                )),
        )
        .into_any_element()
}

/// Render the load-error and action-message bar.
pub fn message_bar(view: &KanbanView, cx: &mut Context<KanbanView>) -> AnyElement {
    let theme = cx.theme();
    let danger = theme.colors.danger;
    let muted = theme.colors.muted_foreground;
    let message = view
        .load_error()
        .map(str::to_string)
        .or_else(|| view.message().map(str::to_string));
    let Some(message) = message else {
        return div().into_any_element();
    };
    let load_failure = view.load_error().is_some();
    div()
        .id("kanban-message")
        .debug_selector(|| "kanban-message".to_string())
        .flex()
        .items_center()
        .justify_between()
        .gap(px(8.0))
        .px(px(16.0))
        .py(px(6.0))
        .bg(if load_failure {
            danger.opacity(0.12)
        } else {
            muted.opacity(0.12)
        })
        .child(
            div()
                .text_size(px(11.0))
                .text_color(if load_failure { danger } else { muted })
                .child(message),
        )
        .when(!load_failure, |element| {
            element.child(action_button(
                "kanban-message-dismiss",
                t_global("kanban.cancel"),
                Some(IconName::X),
                cx,
                |view, _window, cx| view.dismiss_message(cx),
            ))
        })
        .into_any_element()
}

/// Render the three-column board.
pub fn board(view: &KanbanView, cx: &mut Context<KanbanView>) -> AnyElement {
    let grouped = model::group(view.cards());
    div()
        .id("kanban-board")
        .debug_selector(|| "kanban-board".to_string())
        .flex()
        .items_start()
        .gap(px(16.0))
        .p(px(16.0))
        .size_full()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|view, _, _, cx| view.close_menu(cx)),
        )
        .children(
            grouped
                .into_iter()
                .map(|(column_id, cards)| column_element(column_id, cards, view, cx)),
        )
        .into_any_element()
}

/// Render one board column.
fn column_element(
    column: ColumnId,
    cards: Vec<KanbanCardRow>,
    view: &KanbanView,
    cx: &mut Context<KanbanView>,
) -> AnyElement {
    let theme = cx.theme();
    let surface = theme.colors.secondary;
    let border = theme.colors.border;
    let muted = theme.colors.muted_foreground;
    let foreground = theme.colors.foreground;
    let accent: Hsla = rgb(column.accent()).into();
    let collapsed = column == ColumnId::Done && view.done_collapsed();
    let hovered = view.drag_target() == Some(column);
    let count = cards.len();
    let slug = column.slug().to_string();
    let body_id = format!("kanban-column-body-{slug}");
    let body_debug = body_id.clone();
    let card_ids: Vec<String> = cards.iter().map(|card| card.id.clone()).collect();

    let header = div()
        .flex()
        .items_center()
        .justify_between()
        .px(px(12.0))
        .py(px(8.0))
        .border_b_1()
        .border_color(border)
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .when(column == ColumnId::Done, |element| {
                    element.child(icon_button(
                        "kanban-done-toggle",
                        if collapsed {
                            IconName::ChevronRight
                        } else {
                            IconName::ChevronDown
                        },
                        t_global("kanban.done"),
                        cx,
                        move |view, _window, cx| view.toggle_done(cx),
                    ))
                })
                .child(div().w(px(8.0)).h(px(8.0)).rounded_full().bg(accent))
                .child(
                    div()
                        .text_size(px(12.5))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(foreground)
                        .child(t_global(column.label_key())),
                )
                .child(
                    div()
                        .px(px(5.0))
                        .py(px(1.0))
                        .rounded(px(4.0))
                        .border_1()
                        .border_color(border)
                        .text_size(px(9.5))
                        .text_color(muted)
                        .child(count.to_string()),
                ),
        );

    let body = div()
        .id(body_id)
        .debug_selector(move || body_debug.clone())
        .flex()
        .flex_col()
        .flex_1()
        .gap(px(6.0))
        .p(px(8.0))
        .overflow_y_scroll()
        .on_drop::<DraggedCard>(
            cx.listener(move |view, dragged: &DraggedCard, _window, cx| {
                view.move_card(&dragged.id, column, cx);
                view.set_drag_target(None, cx);
            }),
        )
        .on_drag_move::<DraggedCard>(cx.listener(move |view, _event, _window, cx| {
            view.set_drag_target(Some(column), cx);
        }))
        .children(cards.iter().map(|card| card_element(card, view, cx)))
        .when(
            cards.is_empty() && view.new_card_column() != Some(column),
            |element| {
                element.child(
                    div()
                        .py(px(16.0))
                        .text_size(px(10.5))
                        .text_color(muted)
                        .child(t_global("kanban.no_tasks_yet")),
                )
            },
        )
        .child(add_form(column, view, cx));

    let mut container = div()
        .id(format!("kanban-column-{slug}"))
        .debug_selector({
            let slug = slug.clone();
            move || format!("kanban-column-{slug}")
        })
        .flex()
        .flex_col()
        .flex_1()
        .min_w(px(240.0))
        .max_w(px(360.0))
        .h_full()
        .rounded(px(8.0))
        .bg(surface)
        .border_1()
        .border_color(if hovered { accent } else { border })
        .overflow_hidden()
        .child(header);

    if !collapsed {
        container = container.child(body);
    }

    if let Some(menu) = context_menu(&card_ids, view, cx) {
        container = container.child(menu);
    }
    container.into_any_element()
}
