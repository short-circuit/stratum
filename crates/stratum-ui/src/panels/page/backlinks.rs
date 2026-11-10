//! Backlinks card for the page surface.
//!
//! Mirrors the shipped desktop `BacklinksPanel`: a collapsible card with the
//! backlink count, linked references, unlinked mentions, row navigation and the
//! bounded snippet card. The partition helper lives in the provider seam; this
//! module only renders the rows and dispatches to [`super::PageView`].

use gpui_kit::assets::IconName;
use gpui_kit::component::{ActiveTheme, Icon, Sizable};
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use super::PageView;
use crate::provider::{group_backlinks, BacklinkRow};

/// Render the backlinks card.
pub fn backlinks_section(view: &PageView, cx: &mut Context<PageView>) -> impl IntoElement {
    let theme = cx.theme();
    let border = theme.colors.border;
    let muted = theme.colors.muted_foreground;
    let foreground = theme.colors.foreground;
    let link = theme.colors.link;
    let panel = theme.colors.secondary;
    let danger = theme.colors.danger;

    let count = view.backlinks().len();
    let mut section = div()
        .border_t_1()
        .border_color(border)
        .bg(panel)
        .flex()
        .flex_col()
        .child(
            div()
                .id("page-backlinks-toggle")
                .flex()
                .items_center()
                .justify_between()
                .px(px(12.0))
                .py(px(5.0))
                .cursor_pointer()
                .child(
                    div()
                        .text_size(px(11.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(muted)
                        .child(format!("{}{})", t_global("backlinks.backlinks"), count)),
                )
                .child(
                    Icon::new(if view.backlinks_open() {
                        IconName::ChevronDown
                    } else {
                        IconName::ChevronRight
                    })
                    .with_size(px(12.0))
                    .text_color(muted),
                )
                .on_click(cx.listener(|view, _, _, cx| view.toggle_backlinks(cx))),
        );

    if !view.backlinks_open() {
        return section;
    }

    let (linked, unlinked) = group_backlinks(view.backlinks().to_vec());
    section = section
        .child(backlink_group(
            &linked,
            "backlinks.linked_references",
            foreground,
            link,
            muted,
            cx,
        ))
        .child(backlink_group(
            &unlinked,
            "backlinks.unlinked_mentions",
            foreground,
            link,
            muted,
            cx,
        ));

    if linked.is_empty() && unlinked.is_empty() {
        section = section.child(
            div()
                .px(px(12.0))
                .pb(px(6.0))
                .text_size(px(11.0))
                .text_color(muted)
                .child(t_global("backlinks.no_backlinks_found")),
        );
    }

    if let Some(snippet) = view.snippet() {
        section = section.child(snippet_card(
            snippet.note_title.clone(),
            snippet.anchor_content.clone(),
            snippet.context.clone(),
            None,
            foreground,
            muted,
            link,
            cx,
        ));
    } else if let Some(error) = view.snippet_error() {
        section = section.child(snippet_card(
            String::new(),
            String::new(),
            Vec::new(),
            Some(error.to_string()),
            foreground,
            muted,
            danger,
            cx,
        ));
    }

    section
}

/// One linked/unlinked group.
fn backlink_group(
    rows: &[BacklinkRow],
    heading_key: &'static str,
    foreground: Hsla,
    link: Hsla,
    muted: Hsla,
    cx: &mut Context<PageView>,
) -> impl IntoElement {
    let mut group = div().flex().flex_col().px(px(12.0)).pb(px(4.0)).child(
        div().text_size(px(10.0)).text_color(muted).child(format!(
            "{}{})",
            t_global(heading_key),
            rows.len()
        )),
    );
    for row in rows {
        group = group.child(backlink_row(row, foreground, link, muted, cx));
    }
    group
}

/// One backlink row with navigation and snippet controls.
fn backlink_row(
    row: &BacklinkRow,
    foreground: Hsla,
    link: Hsla,
    muted: Hsla,
    cx: &mut Context<PageView>,
) -> impl IntoElement {
    let source_page = row.source_page.clone();
    let note_id = row.source_page.clone();
    let anchor_id = row.source_id.clone();
    div()
        .id(format!("page-backlink-{}", row.source_id))
        .flex()
        .flex_col()
        .gap(px(2.0))
        .px(px(4.0))
        .py(px(3.0))
        .rounded(px(4.0))
        .cursor_pointer()
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(link)
                        .child(row.source_page.clone()),
                )
                .child(
                    div()
                        .id(format!("page-snippet-{}", row.source_id))
                        .px(px(4.0))
                        .rounded(px(3.0))
                        .text_size(px(9.0))
                        .text_color(muted)
                        .cursor_pointer()
                        .child(t_global("backlinks.backlinked_snippet"))
                        .on_click(cx.listener(move |view, _, _, cx| {
                            view.load_snippet(&note_id, &anchor_id, cx);
                        })),
                ),
        )
        .child(
            div()
                .text_size(px(11.0))
                .text_color(foreground.opacity(0.8))
                .child(row.context.clone()),
        )
        .on_click(cx.listener(move |view, _, _, _cx| {
            view.navigate_to(&source_page);
        }))
}

/// The bounded snippet card, or an error variant.
#[allow(clippy::too_many_arguments)]
fn snippet_card(
    note_title: String,
    anchor_content: String,
    context: Vec<String>,
    error: Option<String>,
    foreground: Hsla,
    muted: Hsla,
    accent: Hsla,
    cx: &mut Context<PageView>,
) -> impl IntoElement {
    let mut card = div()
        .mx(px(12.0))
        .mb(px(6.0))
        .p(px(8.0))
        .rounded(px(6.0))
        .border_1()
        .border_color(muted.opacity(0.3))
        .flex()
        .flex_col()
        .gap(px(3.0))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_size(px(10.0))
                        .text_color(muted)
                        .child(t_global("backlinks.backlinked_snippet")),
                )
                .child(
                    div()
                        .id("page-snippet-close")
                        .text_size(px(10.0))
                        .text_color(muted)
                        .cursor_pointer()
                        .child(t_global("backlinks.close_snippet"))
                        .on_click(cx.listener(|view, _, _, cx| view.clear_snippet(cx))),
                ),
        );

    if let Some(error) = error {
        card = card.child(div().text_size(px(11.0)).text_color(accent).child(error));
        return card;
    }

    card = card
        .child(div().text_size(px(10.0)).text_color(muted).child(format!(
            "{} {note_title}",
            t_global("backlink_snippet_body.backlinked_note")
        )))
        .child(
            div()
                .text_size(px(11.0))
                .text_color(foreground)
                .child(anchor_content),
        );
    for row in context {
        card = card.child(div().text_size(px(10.0)).text_color(muted).child(row));
    }
    card
}
