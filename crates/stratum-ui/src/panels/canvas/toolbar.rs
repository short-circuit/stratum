//! Whiteboard toolbar and style panel.
//!
//! The toolbar mirrors the shipped Excalidraw tool rail: tool chips, undo/redo,
//! selection actions and zoom controls. The style panel exposes stroke,
//! background, fill, width, opacity, align and distribute actions. Every action
//! routes through [`super::CanvasView`] so the document edit and autosave policy
//! stay in one place.

use gpui_kit::component::{ActiveTheme, Theme};
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_canvas::{Align, Axis, FillStyle, Tool};
use stratum_i18n::t_global;

use super::model::tool_label_key;
use super::{button, CanvasView};

/// Tool chips in the shipped rail order.
const TOOLS: &[Tool] = &[
    Tool::Select,
    Tool::Hand,
    Tool::Rectangle,
    Tool::Diamond,
    Tool::Ellipse,
    Tool::Arrow,
    Tool::Line,
    Tool::Freedraw,
    Tool::Text,
    Tool::Eraser,
    Tool::Frame,
];

/// Stroke colour presets.
const STROKE_COLORS: &[&str] = &["#1e1e1e", "#e03131", "#2f9e44", "#1971c2", "#f08c00"];

/// Background colour presets (the first entry is transparent).
const BACKGROUND_COLORS: &[&str] = &["transparent", "#ffc9c9", "#b2f2bb", "#a5d8ff", "#ffec99"];

/// Build the tool rail.
pub(super) fn toolbar(view: &CanvasView, cx: &mut Context<CanvasView>) -> impl IntoElement {
    let theme = cx.theme();
    let current = view.session().map(|session| session.state().tool());
    let zoom = view
        .session()
        .map(|session| session.state().viewport().zoom)
        .unwrap_or(1.0);
    div()
        .flex()
        .items_center()
        .flex_wrap()
        .gap(px(6.0))
        .px(px(16.0))
        .py(px(6.0))
        .border_b_1()
        .border_color(theme.colors.border)
        .child(
            div()
                .text_size(px(10.0))
                .text_color(theme.colors.muted_foreground)
                .child(t_global("whiteboard.tool_label")),
        )
        .children(TOOLS.iter().map(|tool| {
            let tool = *tool;
            button(
                tool_button_id(tool),
                t_global(tool_label_key(tool)),
                current == Some(tool),
                cx.listener(move |view, _, _, cx| view.set_tool(tool, cx)),
            )
        }))
        .child(div().w(px(8.0)))
        .child(button(
            "whiteboard-undo",
            t_global("whiteboard.undo"),
            false,
            cx.listener(|view, _, _, cx| view.undo(cx)),
        ))
        .child(button(
            "whiteboard-redo",
            t_global("whiteboard.redo"),
            false,
            cx.listener(|view, _, _, cx| view.redo(cx)),
        ))
        .child(button(
            "whiteboard-duplicate",
            t_global("whiteboard.duplicate_selection"),
            false,
            cx.listener(|view, _, _, cx| view.duplicate_selection(cx)),
        ))
        .child(button(
            "whiteboard-delete",
            t_global("whiteboard.delete_selection"),
            false,
            cx.listener(|view, _, _, cx| view.delete_selection(cx)),
        ))
        .child(div().flex_1())
        // Zoom controls wrap as one cluster: on a narrow window the rail wraps
        // to a second row and a split cluster (zoom-out on one row, the rest on
        // the next) reads as a layout bug.
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .flex_shrink_0()
                .child(button(
                    "whiteboard-zoom-out",
                    t_global("whiteboard.zoom_out"),
                    false,
                    cx.listener(|view, _, _, cx| view.zoom(0.9, cx)),
                ))
                .child(
                    div()
                        .text_size(px(10.0))
                        .text_color(theme.colors.muted_foreground)
                        .child(format!("{:.0}%", zoom * 100.0)),
                )
                .child(button(
                    "whiteboard-zoom-in",
                    t_global("whiteboard.zoom_in"),
                    false,
                    cx.listener(|view, _, _, cx| view.zoom(1.1, cx)),
                ))
                .child(button(
                    "whiteboard-zoom-reset",
                    t_global("whiteboard.zoom_reset"),
                    false,
                    cx.listener(|view, _, _, cx| view.reset_zoom(cx)),
                )),
        )
}

/// Build the style side panel.
pub(super) fn style_panel(_view: &CanvasView, cx: &mut Context<CanvasView>) -> impl IntoElement {
    let theme = cx.theme();
    div()
        .id("whiteboard-style")
        .debug_selector(|| "whiteboard-style".to_string())
        .flex()
        .flex_col()
        .gap(px(8.0))
        .w(px(220.0))
        .p(px(12.0))
        .border_l_1()
        .border_color(theme.colors.border)
        .overflow_y_scroll()
        .child(section_label(t_global("whiteboard.style"), theme))
        .child(section_label(t_global("whiteboard.stroke_color"), theme))
        .child(
            div()
                .flex()
                .gap(px(4.0))
                .children(STROKE_COLORS.iter().enumerate().map(|(index, color)| {
                    let color = *color;
                    swatch(
                        format!("whiteboard-stroke-{index}"),
                        color,
                        cx.listener(move |view, _, _, cx| {
                            view.apply_style(super::stroke_patch(color), cx);
                        }),
                    )
                })),
        )
        .child(section_label(
            t_global("whiteboard.background_color"),
            theme,
        ))
        .child(
            div()
                .flex()
                .gap(px(4.0))
                .children(BACKGROUND_COLORS.iter().enumerate().map(|(index, color)| {
                    let color = *color;
                    swatch(
                        format!("whiteboard-background-{index}"),
                        color,
                        cx.listener(move |view, _, _, cx| {
                            view.apply_style(
                                stratum_canvas::StylePatch {
                                    background_color: Some(color.to_string()),
                                    ..Default::default()
                                },
                                cx,
                            );
                        }),
                    )
                })),
        )
        .child(section_label(t_global("whiteboard.fill_style"), theme))
        .child(
            div().flex().flex_wrap().gap(px(4.0)).children(
                [
                    FillStyle::Hachure,
                    FillStyle::CrossHatch,
                    FillStyle::Solid,
                    FillStyle::ZigZag,
                    FillStyle::Dots,
                ]
                .into_iter()
                .map(|fill| {
                    button(
                        format!("whiteboard-fill-{}", fill.as_wire()),
                        fill.as_wire().to_string(),
                        false,
                        cx.listener(move |view, _, _, cx| {
                            view.apply_style(super::fill_patch(fill), cx);
                        }),
                    )
                }),
            ),
        )
        .child(section_label(t_global("whiteboard.stroke_width"), theme))
        .child(
            div()
                .flex()
                .gap(px(4.0))
                .children([1.0, 2.0, 4.0].into_iter().map(|width| {
                    button(
                        format!("whiteboard-width-{width:.0}"),
                        format!("{width:.0}"),
                        false,
                        cx.listener(move |view, _, _, cx| {
                            view.apply_style(
                                stratum_canvas::StylePatch {
                                    stroke_width: Some(width),
                                    ..Default::default()
                                },
                                cx,
                            );
                        }),
                    )
                })),
        )
        .child(section_label(t_global("whiteboard.opacity"), theme))
        .child(
            div()
                .flex()
                .gap(px(4.0))
                .children([25.0, 50.0, 100.0].into_iter().map(|value| {
                    button(
                        format!("whiteboard-opacity-{value:.0}"),
                        format!("{value:.0}%"),
                        false,
                        cx.listener(move |view, _, _, cx| {
                            view.apply_style(super::opacity_patch(value), cx);
                        }),
                    )
                })),
        )
        .child(section_label(t_global("whiteboard.align"), theme))
        .child(
            div().flex().flex_wrap().gap(px(4.0)).children(
                [
                    (Align::Left, "whiteboard.align_left"),
                    (Align::CenterX, "whiteboard.align_center"),
                    (Align::Right, "whiteboard.align_right"),
                    (Align::Top, "whiteboard.align_top"),
                    (Align::CenterY, "whiteboard.align_middle"),
                    (Align::Bottom, "whiteboard.align_bottom"),
                ]
                .into_iter()
                .map(|(align, key)| {
                    button(
                        format!("whiteboard-align-{}", key.rsplit('.').next().unwrap_or(key)),
                        t_global(key),
                        false,
                        cx.listener(move |view, _, _, cx| view.align(align, cx)),
                    )
                }),
            ),
        )
        .child(
            div()
                .flex()
                .gap(px(4.0))
                .child(button(
                    "whiteboard-distribute-horizontal",
                    t_global("whiteboard.distribute_horizontal"),
                    false,
                    cx.listener(|view, _, _, cx| view.distribute(Axis::Horizontal, cx)),
                ))
                .child(button(
                    "whiteboard-distribute-vertical",
                    t_global("whiteboard.distribute_vertical"),
                    false,
                    cx.listener(|view, _, _, cx| view.distribute(Axis::Vertical, cx)),
                )),
        )
}

/// Stable debug id for a tool chip.
fn tool_button_id(tool: Tool) -> &'static str {
    match tool {
        Tool::Select => "whiteboard-tool-select",
        Tool::Hand => "whiteboard-tool-hand",
        Tool::Rectangle => "whiteboard-tool-rectangle",
        Tool::Diamond => "whiteboard-tool-diamond",
        Tool::Ellipse => "whiteboard-tool-ellipse",
        Tool::Arrow => "whiteboard-tool-arrow",
        Tool::Line => "whiteboard-tool-line",
        Tool::Freedraw => "whiteboard-tool-freedraw",
        Tool::Text => "whiteboard-tool-text",
        Tool::Eraser => "whiteboard-tool-eraser",
        Tool::Frame => "whiteboard-tool-frame",
    }
}

/// A section caption.
fn section_label(label: String, theme: &Theme) -> impl IntoElement {
    div()
        .text_size(px(10.0))
        .text_color(theme.colors.muted_foreground)
        .child(label)
}

/// A colour swatch button.
fn swatch(
    id: impl Into<String>,
    color: &'static str,
    listener: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let id: String = id.into();
    let debug_id = id.clone();
    div()
        .id(id)
        .debug_selector(move || debug_id.clone())
        .w(px(20.0))
        .h(px(20.0))
        .rounded(px(4.0))
        .border_1()
        .border_color(rgb(0xced4da))
        .bg(rgb(if color == "transparent" {
            0xffffff
        } else {
            hex_color(color)
        }))
        .cursor_pointer()
        .on_click(listener)
}

/// Parse a `#rrggbb` colour into an rgb value.
fn hex_color(color: &str) -> u32 {
    u32::from_str_radix(color.trim_start_matches('#'), 16).unwrap_or(0xffffff)
}
