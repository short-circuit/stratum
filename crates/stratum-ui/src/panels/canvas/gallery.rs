//! Whiteboard gallery: board cards, previews, multi-select and the create/
//! rename/delete controls.
//!
//! The gallery renders the [`super::model::GalleryModel`] loaded through the
//! provider seam. Previews are stored WebP data URLs from the `.excalidraw`
//! metadata; rows without one get a freshly rendered preview in memory. All
//! actions route back to [`super::CanvasView`].

use std::sync::Arc;

use base64::Engine as _;
use gpui_kit::component::input::Input;
use gpui_kit::component::{ActiveTheme, Theme};
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use super::{button, CanvasView};

/// Build the gallery surface.
pub(super) fn gallery(view: &CanvasView, cx: &mut Context<CanvasView>) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .size_full()
        .child(header(view, cx))
        .child(status(view, cx))
        .child(
            div()
                .id("whiteboard-gallery")
                .debug_selector(|| "whiteboard-gallery".to_string())
                .flex()
                .flex_wrap()
                .gap(px(12.0))
                .p(px(24.0))
                .flex_1()
                .overflow_y_scroll()
                .children(view.gallery().rows().iter().map(|row| {
                    card(
                        row.name.clone(),
                        row.element_count,
                        row.preview.clone(),
                        row.selected,
                        cx,
                    )
                }))
                .when(view.gallery().rows().is_empty(), |element| {
                    element.child(
                        div()
                            .w_full()
                            .py(px(48.0))
                            .text_size(px(13.0))
                            .text_color(cx.theme().colors.muted_foreground)
                            .child(t_global(
                                "whiteboard.no_whiteboards_yet_create_one_to_get_started",
                            )),
                    )
                }),
        )
}

/// The title, create field and create action.
fn header(view: &CanvasView, cx: &mut Context<CanvasView>) -> impl IntoElement {
    let theme = cx.theme();
    div()
        .flex()
        .items_center()
        .gap(px(10.0))
        .px(px(24.0))
        .py(px(12.0))
        .border_b_1()
        .border_color(theme.colors.border)
        .child(
            div()
                .text_size(px(18.0))
                .font_weight(FontWeight::BOLD)
                .child(t_global("whiteboard.whiteboards")),
        )
        .child(div().flex_1())
        .child(
            div()
                .w(px(220.0))
                .child(Input::new(view.create_name_input()).appearance(false)),
        )
        .child(button(
            "whiteboard-create",
            t_global("whiteboard.create"),
            true,
            cx.listener(|view, _, window, cx| view.create_board(window, cx)),
        ))
}

/// Selection count, delete and rename controls plus inline messages.
fn status(view: &CanvasView, cx: &mut Context<CanvasView>) -> impl IntoElement {
    let theme = cx.theme();
    let selected = view.gallery().selected_names();
    div()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .px(px(24.0))
        .py(px(8.0))
        .when(!selected.is_empty(), |element| {
            element.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(theme.colors.muted_foreground)
                            .child(format!(
                                "{} {}",
                                selected.len(),
                                t_global("whiteboard.selected_count")
                            )),
                    )
                    .child(button(
                        "whiteboard-delete-selected",
                        t_global("whiteboard.delete"),
                        false,
                        cx.listener(|view, _, _, cx| view.delete_selected(cx)),
                    )),
            )
        })
        .when_some(
            view.rename_target().map(str::to_string),
            |element, target| {
                element.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .child(
                            div()
                                .text_size(px(11.0))
                                .text_color(theme.colors.muted_foreground)
                                .child(format!(
                                    "{} {target}",
                                    t_global("whiteboard.rename_whiteboard_label")
                                )),
                        )
                        .child(
                            div()
                                .w(px(180.0))
                                .child(Input::new(view.rename_input()).appearance(false)),
                        )
                        .child(button(
                            "whiteboard-rename-confirm",
                            t_global("whiteboard.rename"),
                            true,
                            cx.listener(|view, _, _, cx| view.confirm_rename(cx)),
                        ))
                        .child(button(
                            "whiteboard-rename-cancel",
                            t_global("whiteboard.cancel"),
                            false,
                            cx.listener(|view, _, _, cx| view.cancel_rename(cx)),
                        )),
                )
            },
        )
        .when_some(view.message().map(str::to_string), |element, message| {
            element.child(
                div()
                    .text_size(px(11.0))
                    .text_color(theme.colors.muted_foreground)
                    .child(message),
            )
        })
        .when_some(view.error().map(str::to_string), |element, error| {
            element.child(
                div()
                    .text_size(px(11.0))
                    .text_color(theme.colors.warning)
                    .child(error),
            )
        })
}

/// One board card.
fn card(
    name: String,
    element_count: usize,
    preview: Option<String>,
    selected: bool,
    cx: &mut Context<CanvasView>,
) -> impl IntoElement {
    let theme = cx.theme();
    let border = if selected {
        theme.colors.accent
    } else {
        theme.colors.border
    };
    let open_name = name.clone();
    let rename_name = name.clone();
    let toggle_name = name.clone();
    div()
        .id(format!("whiteboard-card-{name}"))
        .debug_selector({
            let name = name.clone();
            move || format!("whiteboard-card-{name}")
        })
        .flex()
        .flex_col()
        .w(px(220.0))
        .rounded(px(8.0))
        .border_1()
        .border_color(border)
        .bg(theme.colors.background)
        .overflow_hidden()
        .child(preview_element(preview, theme))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(2.0))
                .p(px(10.0))
                .child(
                    div()
                        .text_size(px(13.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(name),
                )
                .child(
                    div()
                        .text_size(px(10.0))
                        .text_color(theme.colors.muted_foreground)
                        .child(format!("{element_count}")),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .px(px(10.0))
                .pb(px(10.0))
                .child(button(
                    format!("whiteboard-open-{open_name}"),
                    t_global("whiteboard.open_board"),
                    true,
                    cx.listener(move |view, _, window, cx| {
                        view.open_board(&open_name, None, window, cx);
                    }),
                ))
                .child(button(
                    format!("whiteboard-select-{toggle_name}"),
                    if selected {
                        "✓".to_string()
                    } else {
                        "○".to_string()
                    },
                    selected,
                    cx.listener(move |view, _, _, cx| view.toggle_row(&toggle_name, cx)),
                ))
                .child(button(
                    format!("whiteboard-rename-{rename_name}"),
                    t_global("whiteboard.rename"),
                    false,
                    cx.listener(move |view, _, window, cx| {
                        view.begin_rename(&rename_name, window, cx);
                    }),
                )),
        )
}

/// Render a stored WebP preview.
fn preview_element(preview: Option<String>, theme: &Theme) -> impl IntoElement {
    let image = preview.as_deref().and_then(preview_image);
    div()
        .h(px(120.0))
        .w_full()
        .bg(theme.colors.muted)
        .flex()
        .items_center()
        .justify_center()
        .when_some(image, |element, image| {
            element.child(img(image).max_w_full().max_h(px(120.0)))
        })
}

/// Decode a WebP data URL into a gpui image.
fn preview_image(data_url: &str) -> Option<Arc<Image>> {
    let encoded = data_url.split_once(',').map(|(_, data)| data)?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .ok()?;
    Some(Arc::new(Image::from_bytes(ImageFormat::Webp, bytes)))
}
