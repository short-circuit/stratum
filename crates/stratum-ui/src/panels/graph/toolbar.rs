//! Graph toolbar: refresh, view modes, component selector, search, settings
//! toggle and the node/edge counter.
//!
//! This mirrors the shipped `GraphToolbar` layout with native gpui primitives.
//! The toolbar owns no state; every action routes through [`GraphView`] so the
//! filter and cached frame are rebuilt deterministically.

use gpui_kit::component::input::Input;
use gpui_kit::component::ActiveTheme;
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_graph::ViewMode;
use stratum_i18n::t_global;

use super::GraphView;

/// Render the toolbar row.
pub(super) fn toolbar(view: &GraphView, cx: &mut Context<GraphView>) -> impl IntoElement {
    let (accent, accent_foreground, background, muted_foreground, border) = {
        let theme = cx.theme();
        (
            theme.colors.accent,
            theme.colors.accent_foreground,
            theme.colors.background,
            theme.colors.muted_foreground,
            theme.colors.border,
        )
    };
    div()
        .flex()
        .items_center()
        .flex_wrap()
        .gap(px(8.0))
        .px(px(16.0))
        .py(px(8.0))
        .border_b_1()
        .border_color(border)
        .bg(background)
        .child(refresh_button(view, cx))
        .child(mode_chip(
            "graph-mode-full",
            t_global("graph.full"),
            view.view_mode == ViewMode::Full,
            (accent, accent_foreground),
            (background, muted_foreground),
            border,
            cx.listener(|view, _, _, cx| view.set_view_mode(ViewMode::Full, cx)),
        ))
        .child(mode_chip(
            "graph-mode-component",
            t_global("graph.components"),
            view.view_mode == ViewMode::Component,
            (accent, accent_foreground),
            (background, muted_foreground),
            border,
            cx.listener(|view, _, _, cx| view.set_view_mode(ViewMode::Component, cx)),
        ))
        .child(mode_chip(
            "graph-mode-orphans",
            t_global("graph.orphans"),
            view.view_mode == ViewMode::Orphans,
            (accent, accent_foreground),
            (background, muted_foreground),
            border,
            cx.listener(|view, _, _, cx| view.set_view_mode(ViewMode::Orphans, cx)),
        ))
        .when(view.view_mode == ViewMode::Component, |element| {
            element.child(component_selector(view, cx))
        })
        .child(div().flex_1())
        .child(
            div()
                .w(px(180.0))
                .child(Input::new(&view.search_input).appearance(false)),
        )
        .child(settings_button(view, cx))
        .child(stats(view, cx))
}

/// The refresh action.
fn refresh_button(view: &GraphView, cx: &mut Context<GraphView>) -> impl IntoElement {
    let theme = cx.theme();
    let label = if view.loading {
        t_global("common.loading")
    } else {
        t_global("graph.refresh")
    };
    div()
        .id("graph-refresh")
        .debug_selector(|| "graph-refresh".to_string())
        .px(px(10.0))
        .py(px(4.0))
        .rounded(px(6.0))
        .bg(theme.colors.accent)
        .text_color(theme.colors.accent_foreground)
        .text_size(px(11.0))
        .cursor_pointer()
        .child(label)
        .on_click(cx.listener(|view, _, _, cx| view.reload(cx)))
}

/// One view-mode chip.
fn mode_chip(
    id: &'static str,
    label: String,
    selected: bool,
    selected_colors: (Hsla, Hsla),
    idle_colors: (Hsla, Hsla),
    border: Hsla,
    listener: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let (background, foreground) = if selected {
        selected_colors
    } else {
        idle_colors
    };
    div()
        .id(id)
        .debug_selector(move || id.to_string())
        .px(px(10.0))
        .py(px(4.0))
        .rounded(px(6.0))
        .border_1()
        .border_color(border)
        .bg(background)
        .text_color(foreground)
        .text_size(px(11.0))
        .cursor_pointer()
        .child(label)
        .on_click(listener)
}

/// The component selector chip: clicking cycles to the next component.
fn component_selector(view: &GraphView, cx: &mut Context<GraphView>) -> impl IntoElement {
    let (border, muted) = {
        let theme = cx.theme();
        (theme.colors.border, theme.colors.muted_foreground)
    };
    let count = view
        .data
        .as_ref()
        .map(|data| data.components.len())
        .unwrap_or(0);
    let index = view.selected_component.min(count.saturating_sub(1));
    let size = view
        .data
        .as_ref()
        .and_then(|data| data.components.get(index))
        .map(|component| component.size)
        .unwrap_or(0);
    let label = component_label(index, size);
    div()
        .flex()
        .items_center()
        .gap(px(8.0))
        .child(
            div()
                .id("graph-component")
                .debug_selector(|| "graph-component".to_string())
                .px(px(10.0))
                .py(px(4.0))
                .rounded(px(6.0))
                .border_1()
                .border_color(border)
                .text_size(px(11.0))
                .cursor_pointer()
                .child(label)
                .on_click(cx.listener(|view, _, _, cx| view.cycle_component(cx))),
        )
        .child(
            div()
                .text_size(px(10.0))
                .text_color(muted)
                .child(format!("{count} {}", t_global("graph.groups_total"))),
        )
}

/// The shipped component selector label: `{component} {i+1} ({size} {notes})`.
///
/// The `graph.notes` catalog entry already carries the closing paren in every
/// locale, so the format adds the opening paren and the space; a defensive
/// closing paren is appended only when a locale omits it. This reproduces the
/// shipped `GraphToolbar` text exactly instead of the previous `12Notes` shape.
pub fn component_label(index: usize, size: usize) -> String {
    let notes = t_global("graph.notes");
    let closing = if notes.ends_with(')') { "" } else { ")" };
    format!(
        "{} {} ({} {}{}",
        t_global("graph.component"),
        index + 1,
        size,
        notes,
        closing
    )
}

/// The settings toggle.
fn settings_button(view: &GraphView, cx: &mut Context<GraphView>) -> impl IntoElement {
    let theme = cx.theme();
    let color = if view.settings_open {
        theme.colors.accent
    } else {
        theme.colors.muted_foreground
    };
    div()
        .id("graph-settings-toggle")
        .debug_selector(|| "graph-settings-toggle".to_string())
        .px(px(8.0))
        .py(px(4.0))
        .rounded(px(6.0))
        .text_size(px(11.0))
        .text_color(color)
        .cursor_pointer()
        .child(t_global("graph.graph_settings_label"))
        .on_click(cx.listener(|view, _, _, cx| view.toggle_settings(cx)))
}

/// The node/edge/orphan counter.
fn stats(view: &GraphView, cx: &mut Context<GraphView>) -> impl IntoElement {
    let theme = cx.theme();
    let total = view
        .data
        .as_ref()
        .map(|data| data.graph.node_count)
        .unwrap_or(0);
    let orphans = view
        .data
        .as_ref()
        .map(|data| data.orphans.len())
        .unwrap_or(0);
    let text = match view.filtered.as_ref() {
        Some(filtered) => super::model::stats_text(filtered, total, orphans),
        None => String::new(),
    };
    div()
        .text_size(px(10.0))
        .text_color(theme.colors.muted_foreground)
        .child(text)
}

#[cfg(test)]
mod tests {
    use super::component_label;
    use ::core::prelude::v1::test;
    use stratum_i18n::t_global;

    #[test]
    fn component_label_matches_the_shipped_shape() {
        let label = component_label(0, 12);
        assert_eq!(label, "Component 1 (12 notes)", "{label}");
        // The catalog `graph.notes` entry already ends with the paren; the
        // label must not double it.
        assert!(!label.ends_with("))"), "{label}");
        assert!(label.contains("(12 "), "{label}");
    }

    #[test]
    fn component_label_uses_the_catalog_words() {
        let label = component_label(4, 3);
        assert_eq!(
            label,
            format!("Component 5 (3 {}", t_global("graph.notes")),
            "{label}"
        );
    }
}
