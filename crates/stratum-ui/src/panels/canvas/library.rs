//! Whiteboard library drawer: personal `.excalidrawlib` items, insertion and
//! saving the current selection as a new item.
//!
//! The drawer renders the rows loaded through the provider seam; the library
//! JSON round-trip and element insertion stay in [`stratum_canvas::Library`]
//! behind [`super::model::EditorSession`].

use gpui_kit::component::input::Input;
use gpui_kit::component::ActiveTheme;
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use super::{button, CanvasView};

/// Build the library drawer.
pub(super) fn drawer(view: &CanvasView, cx: &mut Context<CanvasView>) -> impl IntoElement {
    let theme = cx.theme();
    div()
        .id("whiteboard-library")
        .debug_selector(|| "whiteboard-library".to_string())
        .flex()
        .flex_col()
        .gap(px(8.0))
        .w(px(240.0))
        .p(px(12.0))
        .border_l_1()
        .border_color(theme.colors.border)
        .overflow_y_scroll()
        .child(
            div()
                .text_size(px(12.0))
                .font_weight(FontWeight::SEMIBOLD)
                .child(t_global("whiteboard.library")),
        )
        .child(
            div()
                .w_full()
                .child(Input::new(view.library_name_input()).appearance(false)),
        )
        .child(button(
            "whiteboard-save-selection",
            t_global("whiteboard.save_selection_to_library"),
            false,
            cx.listener(|view, _, _, cx| view.save_selection(cx)),
        ))
        .when(view.library_items().is_empty(), |element| {
            element.child(
                div()
                    .text_size(px(10.0))
                    .text_color(theme.colors.muted_foreground)
                    .child(t_global("whiteboard.library_empty")),
            )
        })
        .children(view.library_items().iter().map(|row| {
            let id = row.id.clone();
            let label = row
                .name
                .clone()
                .unwrap_or_else(|| t_global("whiteboard.library_item"));
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(
                    div()
                        .flex_1()
                        .text_size(px(10.0))
                        .text_color(theme.colors.foreground)
                        .child(label),
                )
                .child(button(
                    format!("whiteboard-insert-{id}"),
                    t_global("whiteboard.insert"),
                    true,
                    cx.listener(move |view, _, _, cx| view.insert_library_item(&id, cx)),
                ))
        }))
}
