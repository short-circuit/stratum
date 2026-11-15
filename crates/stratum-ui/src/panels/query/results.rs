//! Datalog result table for the query panel.
//!
//! Renders the shipped `QueryPanel.desktop` result table: a header row per
//! column and one row per result, or the localized empty-result caption when
//! the last run returned no rows. The result itself comes from
//! [`super::model::PanelState`].

use gpui_kit::component::ActiveTheme;
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use super::QueryView;

/// Render the result table or the empty-result caption.
pub fn results_section(view: &QueryView, cx: &mut Context<QueryView>) -> AnyElement {
    let theme = cx.theme();
    let foreground = theme.colors.foreground;
    let muted = theme.colors.muted_foreground;
    let border = theme.colors.border;
    let _ = cx;

    let Some(table) = view.result() else {
        return div().into_any_element();
    };

    if table.rows.is_empty() {
        return div()
            .text_size(px(12.0))
            .text_color(muted)
            .child(t_global("query.query_returned_no_results"))
            .into_any_element();
    }

    div()
        .id("query-results")
        .debug_selector(|| "query-results".to_string())
        .flex()
        .flex_col()
        .rounded(px(6.0))
        .border_1()
        .border_color(border)
        .overflow_hidden()
        .child(table_header(table, foreground, muted))
        .children(
            table
                .rows
                .iter()
                .enumerate()
                .map(|(index, row)| table_row(index, row, foreground, border)),
        )
        .into_any_element()
}

/// The column header row.
fn table_header(
    table: &crate::provider::QueryTable,
    foreground: Hsla,
    muted: Hsla,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap(px(0.0))
        .bg(muted.opacity(0.12))
        .children(table.columns.iter().map(|column| {
            div()
                .flex_1()
                .px(px(8.0))
                .py(px(5.0))
                .text_size(px(10.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(foreground)
                .child(column.clone())
        }))
}

/// One result row.
fn table_row(index: usize, row: &[String], foreground: Hsla, border: Hsla) -> impl IntoElement {
    div()
        .id(format!("query-result-{index}"))
        .debug_selector(move || format!("query-result-{index}"))
        .flex()
        .items_center()
        .gap(px(0.0))
        .border_t_1()
        .border_color(border)
        .children(row.iter().map(|cell| {
            div()
                .flex_1()
                .px(px(8.0))
                .py(px(4.0))
                .text_size(px(11.0))
                .text_color(foreground)
                .child(cell.clone())
        }))
}
