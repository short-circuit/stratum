//! Saved-query list for the query panel.
//!
//! Reproduces the shipped `SavedQueryRow` list: a heading with the saved count,
//! a refresh action, the load error, and one row per saved query with load,
//! rename and delete controls. The list state and provider calls live on
//! [`QueryView`].

use gpui_kit::component::spinner::Spinner;
use gpui_kit::component::{ActiveTheme, Sizable};
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use super::model::PanelState;
use super::QueryView;

/// Render the saved-query section.
pub fn saved_section(view: &QueryView, cx: &mut Context<QueryView>) -> impl IntoElement {
    let theme = cx.theme();
    let muted = theme.colors.muted_foreground;
    let border = theme.colors.border;
    let danger = theme.colors.danger;
    let accent = theme.colors.accent;

    let saved = view.saved();
    div()
        .flex()
        .flex_col()
        .gap(px(8.0))
        .pt(px(12.0))
        .border_t_1()
        .border_color(border)
        .child(heading(saved.len(), theme.colors.foreground, accent, cx))
        .when_some(view.saved_error().map(str::to_string), |element, error| {
            element.child(div().text_size(px(11.0)).text_color(danger).child(error))
        })
        .when(view.saved_loading(), |element| {
            element.child(loading(muted))
        })
        .when(saved.is_empty() && !view.saved_loading(), |element| {
            element.child(empty(muted))
        })
        .children(
            saved
                .iter()
                .enumerate()
                .map(|(index, row)| saved_row(index, row, muted, border, danger, cx)),
        )
}

/// The saved-query heading with the count and refresh action.
fn heading(
    count: usize,
    foreground: Hsla,
    accent: Hsla,
    cx: &mut Context<QueryView>,
) -> impl IntoElement {
    let heading = format!("{}{})", t_global("query.saved_queries"), count);
    div()
        .flex()
        .items_center()
        .justify_between()
        .child(
            div()
                .text_size(px(13.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(foreground)
                .child(heading),
        )
        .child(
            div()
                .id("query-refresh-saved")
                .debug_selector(|| "query-refresh-saved".to_string())
                .text_size(px(11.0))
                .text_color(accent)
                .cursor_pointer()
                .child(t_global("query.refresh_list"))
                .on_click(cx.listener(|view, _, _, cx| view.refresh_saved(cx))),
        )
}

/// The refreshing spinner row.
fn loading(muted: Hsla) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap(px(6.0))
        .child(Spinner::new().with_size(px(14.0)))
        .child(
            div()
                .text_size(px(11.0))
                .text_color(muted)
                .child(t_global("query.refresh_list")),
        )
}

/// The empty saved-query hint.
fn empty(muted: Hsla) -> impl IntoElement {
    div().text_size(px(11.0)).text_color(muted).child(t_global(
        "query.no_saved_queries_yet_write_a_query_above_and_click_save_quer",
    ))
}

/// Render one saved query row.
fn saved_row(
    index: usize,
    row: &crate::provider::SavedQueryRow,
    muted: Hsla,
    border: Hsla,
    danger: Hsla,
    cx: &mut Context<QueryView>,
) -> impl IntoElement {
    let load_row = row.clone();
    div()
        .id(format!("query-saved-{index}"))
        .debug_selector(move || format!("query-saved-{index}"))
        .flex()
        .items_center()
        .justify_between()
        .gap(px(8.0))
        .p(px(8.0))
        .rounded(px(6.0))
        .border_1()
        .border_color(border)
        .cursor_pointer()
        .on_click(cx.listener(move |view, _, window, cx| {
            view.load_saved(&load_row, window, cx);
        }))
        .child(saved_row_details(row, muted))
        .child(saved_row_actions(index, row, muted, danger, cx))
}

/// The name and update subtitle for a saved query.
fn saved_row_details(row: &crate::provider::SavedQueryRow, muted: Hsla) -> impl IntoElement {
    let subtitle = PanelState::saved_query_subtitle(row);
    div()
        .flex()
        .flex_col()
        .gap(px(1.0))
        .child(div().text_size(px(12.0)).child(row.name.clone()))
        .child(div().text_size(px(10.0)).text_color(muted).child(subtitle))
}

/// The rename/delete controls for a saved query.
fn saved_row_actions(
    index: usize,
    row: &crate::provider::SavedQueryRow,
    muted: Hsla,
    danger: Hsla,
    cx: &mut Context<QueryView>,
) -> impl IntoElement {
    let rename_name = row.name.clone();
    let delete_name = row.name.clone();
    div()
        .flex()
        .items_center()
        .gap(px(4.0))
        .child(row_action(
            format!("query-rename-{index}"),
            t_global("common.rename"),
            muted,
            cx,
            move |view, cx| {
                cx.stop_propagation();
                view.open_rename_dialog(&rename_name, cx);
            },
        ))
        .child(row_action(
            format!("query-delete-{index}"),
            t_global("common.delete"),
            danger,
            cx,
            move |view, cx| {
                cx.stop_propagation();
                view.open_delete_dialog(&delete_name, cx);
            },
        ))
}

/// One inline row action.
fn row_action(
    id: String,
    label: String,
    color: Hsla,
    cx: &mut Context<QueryView>,
    action: impl Fn(&mut QueryView, &mut Context<QueryView>) + 'static,
) -> impl IntoElement {
    let selector = id.clone();
    div()
        .id(id)
        .debug_selector(move || selector.clone())
        .px(px(5.0))
        .py(px(2.0))
        .rounded(px(4.0))
        .text_size(px(10.0))
        .text_color(color)
        .cursor_pointer()
        .child(label)
        .on_click(cx.listener(move |view, _, _, cx| action(view, cx)))
}
