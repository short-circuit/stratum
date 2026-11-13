//! Today and stacked past-day journal rows.
//!
//! The today entry renders first. Older days appear as a localized date heading
//! followed by their lazily materialised editor, or a loading placeholder until
//! the stack model loads that section. Rendering is thin; the stacking policy
//! and editor entities live on [`super::JournalView`] and [`super::stack`].

use gpui_kit::assets::IconName;
use gpui_kit::component::{ActiveTheme, Icon, Sizable};
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use super::dates::display_date;
use super::JournalView;

/// Render today's journal section.
pub fn today_section(view: &JournalView, _cx: &mut Context<JournalView>) -> impl IntoElement {
    let theme = _cx.theme();
    let muted = theme.colors.muted_foreground;
    if view.loading() {
        return div().child(loading_placeholder(muted));
    }
    match view.today_editor().cloned() {
        Some(editor) => div().child(editor),
        None => div().child(loading_placeholder(muted)),
    }
}

/// Render one stacked past-day section.
pub fn section_row(
    view: &JournalView,
    index: usize,
    _cx: &mut Context<JournalView>,
) -> impl IntoElement {
    let theme = _cx.theme();
    let muted = theme.colors.muted_foreground;
    let border = theme.colors.border;
    let date = view.section_date(index).unwrap_or_default().to_string();
    let label = display_date(&date);

    let mut row = div()
        .flex()
        .flex_col()
        .px(px(12.0))
        .py(px(8.0))
        .border_t_1()
        .border_color(border)
        .child(
            div()
                .text_size(px(13.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(muted)
                .child(label),
        );
    row = match view.section_editor(index).cloned() {
        Some(editor) => row.child(editor),
        None => row.child(loading_placeholder(muted)),
    };
    row
}

/// A centered loading placeholder for a section that has not materialised.
fn loading_placeholder(muted: Hsla) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_center()
        .gap(px(6.0))
        .py(px(16.0))
        .child(
            Icon::new(IconName::LoaderCircle)
                .with_size(px(14.0))
                .text_color(muted),
        )
        .child(
            div()
                .text_size(px(12.0))
                .text_color(muted)
                .child(t_global("common.loading")),
        )
}
