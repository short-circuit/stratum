//! In-editor overlays: slash menu, wiki autocomplete, marker menu, save chip,
//! link preview and dead-link popup.
//!
//! Overlays are plain `div()` popups positioned from the virtual layout; they do
//! not open separate windows. Detection is a pure function over the block
//! content and caret byte offset so it can be unit tested without a gpui
//! context.

use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::component::ActiveTheme;
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_editor::menus::{
    filter_slash_items, marker_menu_items, slash_items, slash_row_count, wiki_query, wiki_targets,
    WikiTarget,
};
use stratum_editor::{badge_row, MarkerBadge};
use stratum_i18n::{t_global, tr_global};

use crate::panels::editor::{AiStatus, EditorView, SaveStatus};

/// Maximum number of wiki autocomplete rows rendered at once.
const WIKI_MAX_ROWS: usize = 8;

/// Number of wiki autocomplete rows to render for `target_count` resolved
/// targets. A no-match query still renders one row (the "create page" entry).
pub(crate) fn wiki_row_count(target_count: usize) -> usize {
    target_count.clamp(1, WIKI_MAX_ROWS)
}

/// The overlay currently shown above the editor.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum OverlayState {
    /// No overlay.
    #[default]
    None,
    /// Slash menu with a filter query.
    Slash { query: String, selected: usize },
    /// Wiki-link autocomplete for an unclosed `[[`.
    Wiki { query: String, selected: usize },
    /// Marker/priority menu.
    Marker { selected: usize },
    /// Ctrl+hover link preview.
    LinkPreview { target: String },
    /// A wiki-link whose target does not exist yet.
    DeadLink { target: String },
}

impl OverlayState {
    /// Move the selection within an overlay, wrapping at both ends.
    pub fn move_selection(&mut self, delta: isize, len: usize) {
        let selected = match self {
            OverlayState::Slash { selected, .. }
            | OverlayState::Wiki { selected, .. }
            | OverlayState::Marker { selected } => selected,
            _ => return,
        };
        if len == 0 {
            *selected = 0;
            return;
        }
        let next = (*selected as isize + delta).rem_euclid(len as isize);
        *selected = next as usize;
    }

    /// The selected index, if the overlay has one.
    pub fn selected(&self) -> Option<usize> {
        match self {
            OverlayState::Slash { selected, .. }
            | OverlayState::Wiki { selected, .. }
            | OverlayState::Marker { selected } => Some(*selected),
            _ => None,
        }
    }
}

/// Detect an overlay from the content and caret position.
///
/// `content` is the focused block's full content and `caret_byte` is the UTF-8
/// byte offset. `is_journal` and `has_selection` mirror the slash-menu context
/// rules in the shipped React editor.
pub fn detect_overlay(
    content: &str,
    caret_byte: usize,
    is_journal: bool,
    has_selection: bool,
) -> OverlayState {
    let caret = caret_byte.min(content.len());
    if let Some(query) = wiki_query(content, caret) {
        return OverlayState::Wiki {
            query: query.term,
            selected: 0,
        };
    }
    let before = &content[..caret];
    let token = before
        .rsplit_once(char::is_whitespace)
        .map(|(_, token)| token)
        .unwrap_or(before);
    if let Some(query) = token.strip_prefix('/') {
        if !query.contains('/') {
            let items = filter_slash_items(&slash_items(has_selection, is_journal), query);
            if !items.is_empty() {
                return OverlayState::Slash {
                    query: query.to_string(),
                    selected: 0,
                };
            }
        }
    }
    if token.starts_with(':') {
        return OverlayState::Marker { selected: 0 };
    }
    OverlayState::None
}

/// Render the page marker badge strip.
pub fn badge_strip(view: &EditorView, cx: &mut Context<EditorView>) -> impl IntoElement {
    let theme = cx.theme();
    let badges = badge_row(view.state.document());
    let mut strip = div()
        .flex()
        .flex_wrap()
        .items_center()
        .gap(px(6.0))
        .px(px(12.0))
        .py(px(6.0))
        .border_b_1()
        .border_color(theme.colors.border);
    if badges.is_empty() {
        strip = strip.child(
            div()
                .text_size(px(12.0))
                .text_color(theme.colors.muted_foreground)
                .child(t_global("page.prose_empty")),
        );
    }
    for badge in badges {
        strip = strip.child(badge_chip(&badge, theme));
    }
    strip
}

/// Render the save-status chip.
pub fn save_chip(view: &EditorView, cx: &mut Context<EditorView>) -> impl IntoElement {
    let theme = cx.theme();
    let failed = matches!(view.save.status(), SaveStatus::Failed(_))
        || matches!(view.ai_status.as_ref(), Some(AiStatus::Failed(_)));
    div()
        .absolute()
        .right(px(16.0))
        .bottom(px(16.0))
        .px(px(10.0))
        .py(px(5.0))
        .rounded(px(6.0))
        .bg(if failed {
            theme.colors.danger
        } else {
            theme.colors.secondary
        })
        .border_1()
        .border_color(theme.colors.border)
        .text_size(px(11.0))
        .text_color(if failed {
            gpui::white()
        } else {
            theme.colors.muted_foreground
        })
        .child(view.save_label())
}

/// Render the active overlay popup.
pub fn overlay_popup(view: &EditorView, cx: &mut Context<EditorView>) -> impl IntoElement {
    let theme = cx.theme();
    let y = overlay_y(view);
    let mut popup = div()
        .absolute()
        .left(px(72.0))
        .top(px(y.max(0.0)))
        .w(px(420.0))
        .max_h(px(440.0))
        .rounded(px(8.0))
        .bg(theme.colors.background)
        .border_1()
        .border_color(theme.colors.border)
        .shadow_md()
        .p(px(4.0))
        .overflow_y_scrollbar();
    match view.overlay() {
        OverlayState::None => {}
        OverlayState::Slash { query, selected } => {
            popup = popup.child(slash_list(view, query, *selected, theme));
        }
        OverlayState::Wiki { query, selected } => {
            let targets = wiki_targets(
                query,
                &crate::panels::editor::input::overlay_ops::wiki_pages(view),
            );
            popup = popup.child(wiki_list(&targets, query, *selected, theme));
        }
        OverlayState::Marker { selected } => {
            popup = popup.child(marker_list(*selected, theme));
        }
        OverlayState::LinkPreview { target } => {
            popup = popup.child(preview_row(target, theme));
        }
        OverlayState::DeadLink { target } => {
            popup = popup.child(dead_link_row(target, theme));
        }
    }
    popup
}

/// Render the Ctrl+hover preview target, if any.
pub fn hover_preview(view: &EditorView, cx: &mut Context<EditorView>) -> AnyElement {
    let theme = cx.theme();
    match &view.hover_preview {
        Some(target) => preview_row(target, theme).into_any_element(),
        None => div().into_any_element(),
    }
}

fn badge_chip(badge: &MarkerBadge, theme: &gpui_kit::component::Theme) -> impl IntoElement {
    let marker = badge
        .marker
        .map(|marker| marker.as_str())
        .unwrap_or_default();
    let priority = badge
        .priority
        .map(|priority| priority.as_str())
        .unwrap_or_default();
    let label = if priority.is_empty() {
        marker.to_string()
    } else if marker.is_empty() {
        format!("[{priority}]")
    } else {
        format!("{marker} [{priority}]")
    };
    div()
        .px(px(6.0))
        .py(px(2.0))
        .rounded(px(4.0))
        .bg(theme.colors.accent)
        .text_color(theme.colors.accent_foreground)
        .text_size(px(10.0))
        .font_weight(FontWeight::SEMIBOLD)
        .child(label)
}

fn overlay_y(view: &EditorView) -> f32 {
    let Some(caret) = view.state.caret() else {
        return 0.0;
    };
    let ids = view.state.document().ordered_ids();
    let Some(index) = ids.iter().position(|id| *id == caret.block_id) else {
        return 0.0;
    };
    let top = view.layout.offsets().get(index).copied().unwrap_or(0.0);
    let scroll = f32::from(view.scroll.offset().y);
    top + scroll + view.layout.heights().get(index).copied().unwrap_or(24.0)
}

fn slash_list(
    view: &EditorView,
    query: &str,
    selected: usize,
    theme: &gpui_kit::component::Theme,
) -> impl IntoElement {
    let is_journal = view.path().starts_with("journals/");
    let has_selection = view.state.selection().is_some();
    let items = filter_slash_items(&slash_items(has_selection, is_journal), query);
    let rows = slash_row_count(items.len());
    let mut list = div().flex().flex_col().gap(px(1.0));
    for index in 0..rows {
        let Some(item) = items.get(index) else {
            continue;
        };
        let mut row = div()
            .flex()
            .items_center()
            .justify_between()
            .gap(px(8.0))
            .px(px(10.0))
            .py(px(4.0))
            .rounded(px(4.0))
            .text_size(px(12.0))
            .text_color(theme.colors.foreground);
        if index == selected {
            row = row
                .bg(theme.colors.accent)
                .text_color(theme.colors.accent_foreground);
        }
        row = row.child(t_global(item.title_key));
        if let Some(subtitle) = item.subtitle_key {
            row = row.child(
                div()
                    .text_size(px(10.0))
                    .text_color(theme.colors.muted_foreground)
                    .child(t_global(subtitle)),
            );
        }
        list = list.child(row);
    }
    if items.is_empty() {
        list = list.child(
            div()
                .px(px(10.0))
                .py(px(6.0))
                .text_size(px(12.0))
                .text_color(theme.colors.muted_foreground)
                .child(t_global("page.prose_empty")),
        );
    }
    list
}

fn wiki_list(
    targets: &[WikiTarget],
    query: &str,
    selected: usize,
    theme: &gpui_kit::component::Theme,
) -> impl IntoElement {
    let rows = wiki_row_count(targets.len());
    let mut list = div().flex().flex_col().gap(px(1.0));
    for index in 0..rows {
        let (label, secondary) = match targets.get(index) {
            Some(target) => (
                if target.title.is_empty() {
                    target.slug.clone()
                } else {
                    target.title.clone()
                },
                target.path.clone(),
            ),
            None => (format!("[[{query}]]"), t_global("editor.create_page")),
        };
        let mut row = div()
            .flex()
            .items_center()
            .justify_between()
            .px(px(10.0))
            .py(px(6.0))
            .rounded(px(4.0))
            .text_size(px(12.0))
            .text_color(theme.colors.foreground)
            .child(label)
            .child(
                div()
                    .text_size(px(10.0))
                    .text_color(theme.colors.muted_foreground)
                    .child(secondary),
            );
        if index == selected {
            row = row
                .bg(theme.colors.accent)
                .text_color(theme.colors.accent_foreground);
        }
        list = list.child(row);
    }
    list
}

fn marker_list(selected: usize, theme: &gpui_kit::component::Theme) -> impl IntoElement {
    let items = marker_menu_items();
    let mut list = div().flex().flex_col().gap(px(1.0));
    for (index, item) in items.iter().take(10).enumerate() {
        let vars: Vec<(&str, &str)> = item
            .title_vars
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str()))
            .collect();
        let title = if item.title_key.contains('.') {
            tr_global(item.title_key, &vars)
        } else {
            item.title_key.to_string()
        };
        let mut row = div()
            .flex()
            .items_center()
            .justify_between()
            .px(px(10.0))
            .py(px(6.0))
            .rounded(px(4.0))
            .text_size(px(12.0))
            .text_color(theme.colors.foreground)
            .child(title)
            .child(
                div()
                    .text_size(px(10.0))
                    .text_color(theme.colors.muted_foreground)
                    .child(t_global(item.subtitle_key)),
            );
        if index == selected {
            row = row
                .bg(theme.colors.accent)
                .text_color(theme.colors.accent_foreground);
        }
        list = list.child(row);
    }
    list
}

fn preview_row(target: &str, theme: &gpui_kit::component::Theme) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap(px(2.0))
        .px(px(10.0))
        .py(px(6.0))
        .text_size(px(12.0))
        .text_color(theme.colors.foreground)
        .child(target.to_string())
        .child(
            div()
                .text_size(px(10.0))
                .text_color(theme.colors.muted_foreground)
                .child(t_global("editor.create_page")),
        )
}

fn dead_link_row(target: &str, theme: &gpui_kit::component::Theme) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_between()
        .px(px(10.0))
        .py(px(6.0))
        .text_size(px(12.0))
        .text_color(theme.colors.foreground)
        .child(format!("[[{target}]]"))
        .child(
            div()
                .px(px(6.0))
                .py(px(2.0))
                .rounded(px(4.0))
                .bg(theme.colors.accent)
                .text_color(theme.colors.accent_foreground)
                .text_size(px(10.0))
                .child(t_global("editor.create_page")),
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::core::prelude::v1::test;

    #[test]
    fn slash_detection_is_word_anchored() {
        assert!(matches!(
            detect_overlay("/", 1, false, false),
            OverlayState::Slash { .. }
        ));
        assert!(matches!(
            detect_overlay("hello /head", 11, false, false),
            OverlayState::Slash { .. }
        ));
        assert_eq!(detect_overlay("a/b", 3, false, false), OverlayState::None);
    }

    #[test]
    fn wiki_detection_returns_the_open_query() {
        assert_eq!(
            detect_overlay("see [[Pro", 9, false, false),
            OverlayState::Wiki {
                query: "Pro".to_string(),
                selected: 0
            }
        );
    }

    #[test]
    fn marker_detection_needs_a_non_empty_token() {
        assert_eq!(
            detect_overlay("task :", 6, false, false),
            OverlayState::Marker { selected: 0 }
        );
        assert_eq!(detect_overlay("task:", 5, false, false), OverlayState::None);
    }

    #[test]
    fn selection_wraps_at_both_ends() {
        let mut overlay = OverlayState::Slash {
            query: String::new(),
            selected: 0,
        };
        overlay.move_selection(-1, 3);
        assert_eq!(overlay.selected(), Some(2));
        overlay.move_selection(1, 3);
        assert_eq!(overlay.selected(), Some(0));
    }

    #[test]
    fn wiki_rows_cover_every_selectable_entry() {
        assert_eq!(wiki_row_count(0), 1);
        assert_eq!(wiki_row_count(3), 3);
        assert_eq!(wiki_row_count(20), 8);
    }
}
