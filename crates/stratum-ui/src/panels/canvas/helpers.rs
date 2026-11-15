//! Small presentation helpers shared by every whiteboard surface.

use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_canvas::{FillStyle, StylePatch};

/// The style patch helpers shared by the style panel.
pub(crate) fn stroke_patch(color: &str) -> StylePatch {
    StylePatch {
        stroke_color: Some(color.to_string()),
        ..StylePatch::default()
    }
}

/// Fill-style patch helper.
pub(crate) fn fill_patch(style: FillStyle) -> StylePatch {
    StylePatch {
        fill_style: Some(style),
        ..StylePatch::default()
    }
}

/// Opacity patch helper.
pub(crate) fn opacity_patch(value: f64) -> StylePatch {
    StylePatch {
        opacity: Some(value),
        ..StylePatch::default()
    }
}

/// A compact action button shared by every whiteboard surface.
pub(crate) fn button(
    id: impl Into<String>,
    label: String,
    accent: bool,
    listener: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let id: String = id.into();
    let debug_id = id.clone();
    div()
        .id(id)
        .debug_selector(move || debug_id.clone())
        .px(px(8.0))
        .py(px(3.0))
        .rounded(px(5.0))
        .border_1()
        .text_size(px(10.0))
        .cursor_pointer()
        .when(accent, |element| {
            element.bg(rgb(0x1971c2)).text_color(rgb(0xffffff))
        })
        .child(label)
        .on_click(listener)
}
