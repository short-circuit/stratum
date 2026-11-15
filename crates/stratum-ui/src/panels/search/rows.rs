//! Search result rows.
//!
//! Each row mirrors the shipped `SearchPanel.desktop` list item: the page path
//! plus relevance caption on the first line and the highlighted snippet below.
//! Clicking a row records a pending navigation; the shell resolves the page.

use gpui_kit::component::ActiveTheme;
use gpui_kit::prelude::*;
use gpui_kit::*;

use super::model::score_label;
use super::SearchView;

/// Render one result row per hit.
pub fn result_rows(view: &SearchView, cx: &mut Context<SearchView>) -> Vec<AnyElement> {
    let theme = cx.theme();
    let foreground = theme.colors.foreground;
    let muted = theme.colors.muted_foreground;
    let border = theme.colors.border;

    view.results()
        .iter()
        .enumerate()
        .map(|(index, hit)| {
            div()
                .id(format!("search-result-{index}"))
                .debug_selector(move || format!("search-result-{index}"))
                .flex()
                .flex_col()
                .gap(px(2.0))
                .p(px(10.0))
                .rounded(px(6.0))
                .border_1()
                .border_color(border)
                .cursor_pointer()
                .child(div().text_size(px(11.0)).text_color(muted).child(format!(
                    "{} {}",
                    hit.page_path,
                    score_label(hit.score)
                )))
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(foreground)
                        .child(hit.snippet.clone()),
                )
                .on_click(cx.listener(move |view, _, _, cx| {
                    view.open_result(index);
                    cx.notify();
                }))
                .into_any_element()
        })
        .collect()
}
