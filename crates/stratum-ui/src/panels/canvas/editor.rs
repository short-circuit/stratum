//! Whiteboard editor surface: header, toolbar, canvas, style panel and library
//! drawer.
//!
//! The editor composes the sibling modules and renders the save status from the
//! [`super::model::EditorSession`]. It owns no document behaviour: every action
//! routes back to [`super::CanvasView`], which schedules the 800 ms autosave and
//! flushes on back navigation.

use gpui_kit::component::ActiveTheme;
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use super::model::SaveState;
use super::{button, canvas, library, toolbar, CanvasView};

/// Build the editor surface.
pub(super) fn editor(view: &CanvasView, cx: &mut Context<CanvasView>) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .size_full()
        .child(header(view, cx))
        .child(toolbar::toolbar(view, cx))
        .child(
            div()
                .flex()
                .flex_1()
                .overflow_hidden()
                .child(canvas::canvas_element(view, cx))
                .when(view.style_open(), |element| {
                    element.child(toolbar::style_panel(view, cx))
                })
                .when(view.library_open(), |element| {
                    element.child(library::drawer(view, cx))
                }),
        )
        .when_some(view.message().map(str::to_string), |element, message| {
            element.child(status_row(message, cx, false))
        })
        .when_some(view.error().map(str::to_string), |element, error| {
            element.child(status_row(error, cx, true))
        })
}

/// The editor title bar.
fn header(view: &CanvasView, cx: &mut Context<CanvasView>) -> impl IntoElement {
    let theme = cx.theme();
    let name = view
        .session()
        .map(|session| session.name().to_string())
        .unwrap_or_else(|| t_global("whiteboard.whiteboards"));
    div()
        .flex()
        .items_center()
        .gap(px(8.0))
        .px(px(24.0))
        .py(px(10.0))
        .border_b_1()
        .border_color(theme.colors.border)
        .child(button(
            "whiteboard-back",
            t_global("whiteboard.back_to_gallery"),
            false,
            cx.listener(|view, _, _, cx| view.back_to_gallery(cx)),
        ))
        .child(
            div()
                .text_size(px(16.0))
                .font_weight(FontWeight::BOLD)
                .child(name),
        )
        .child(save_chip(view, cx))
        .child(div().flex_1())
        .child(button(
            "whiteboard-copy-link",
            t_global("whiteboard.copy_link"),
            false,
            cx.listener(|view, _, _, cx| view.copy_link(cx)),
        ))
        .child(button(
            "whiteboard-export-svg",
            t_global("whiteboard.export_svg"),
            false,
            cx.listener(|view, _, _, cx| {
                view.export_board(super::ExportFormat::Svg, cx);
            }),
        ))
        .child(button(
            "whiteboard-export-png",
            t_global("whiteboard.export_png"),
            false,
            cx.listener(|view, _, _, cx| {
                view.export_board(super::ExportFormat::Png, cx);
            }),
        ))
        .child(button(
            "whiteboard-library-toggle",
            t_global("whiteboard.library"),
            view.library_open(),
            cx.listener(|view, _, _, cx| view.toggle_library(cx)),
        ))
        .child(button(
            "whiteboard-style-toggle",
            t_global("whiteboard.style"),
            view.style_open(),
            cx.listener(|view, _, _, cx| view.toggle_style(cx)),
        ))
}

/// The save-status chip.
fn save_chip(view: &CanvasView, cx: &mut Context<CanvasView>) -> impl IntoElement {
    let theme = cx.theme();
    let (label, color) = match view.session().map(|session| session.save_state()) {
        Some(SaveState::Saved) => (t_global("whiteboard.saved"), theme.colors.muted_foreground),
        Some(SaveState::Pending) => (t_global("whiteboard.unsaved"), theme.colors.warning),
        Some(SaveState::Failed(message)) => (message.clone(), theme.colors.warning),
        _ => (String::new(), theme.colors.muted_foreground),
    };
    div().text_size(px(10.0)).text_color(color).child(label)
}

/// An inline message/error row.
fn status_row(message: String, cx: &mut Context<CanvasView>, error: bool) -> impl IntoElement {
    let color = if error {
        cx.theme().colors.warning
    } else {
        cx.theme().colors.muted_foreground
    };
    div()
        .px(px(24.0))
        .py(px(6.0))
        .text_size(px(11.0))
        .text_color(color)
        .child(message)
}
