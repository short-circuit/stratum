//! Attachment panel for the native block editor.
//!
//! Parity with the shipped `src/components/AttachmentPanel.tsx`: the header
//! shows the translated title, the attachment count and the summed size, with a
//! collapse toggle and an attach button that opens the platform file picker.
//! The body lists one row per attachment (name, MIME chip, size, delete) inside
//! a bounded, scrollable area.
//!
//! Delete uses the shipped inline confirmation instead of a modal: the first
//! click arms the row and shows `asset.delete_confirm`, a second click on the
//! same row deletes, and arming another row moves the pending target. Every
//! label resolves through [`stratum_i18n::t_global`]; the upload error mapping
//! keeps the `file_too_large` sentinel distinct from a generic failure.
//!
//! The panel is part of the editor page: [`crate::panels::editor::view`] renders
//! it under the virtual list. It never edits the document directly; imports and
//! deletes go through [`EditorView`](crate::panels::editor::EditorView).

use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::component::{ActiveTheme, Icon, Sizable, Theme};
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_editor::AttachmentRow;
use stratum_i18n::t_global;

use crate::panels::editor::asset_state::{total_size_label, upload_error_key};
use crate::panels::editor::EditorView;

/// Maximum body height before the attachment list scrolls.
pub const ATTACHMENT_BODY_MAX_HEIGHT: f32 = 180.0;

// Compile-time bound: the panel body must stay a small, scrollable strip.
const _: () = assert!(ATTACHMENT_BODY_MAX_HEIGHT > 0.0 && ATTACHMENT_BODY_MAX_HEIGHT <= 200.0);

/// The inline confirmation label key for an armed delete, if armed.
pub fn delete_confirm_key(pending: bool) -> Option<&'static str> {
    pending.then_some("asset.delete_confirm")
}

/// Render the attachment panel for the open page.
pub fn attachments_panel(view: &EditorView, cx: &mut Context<EditorView>) -> AnyElement {
    // Clone the theme so the immutable borrow of `cx` ends before the
    // `cx.listener` closures take `cx` mutably.
    let theme = cx.theme().clone();
    let count = view.attachments.len();
    let title = format!("{} ({count})", t_global("asset.attachments"));
    let mut header = div()
        .flex()
        .items_center()
        .gap(px(8.0))
        .px(px(12.0))
        .py(px(6.0))
        .child(collapse_toggle(view, &theme, cx))
        .child(
            div()
                .text_size(px(11.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.colors.muted_foreground)
                .child(title),
        );
    if let Some(total) = total_size_label(&view.attachments) {
        header = header.child(
            div()
                .text_size(px(11.0))
                .text_color(theme.colors.muted_foreground)
                .child(total),
        );
    }
    header = header
        .child(div().flex_1())
        .child(attach_button(view, &theme, cx));

    let mut panel = div()
        .id("attachments-panel")
        .flex()
        .flex_col()
        .border_t_1()
        .border_color(theme.colors.border)
        .bg(theme.colors.background)
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|_, _, _, cx| cx.stop_propagation()),
        )
        .child(header);

    if view.attachments_open {
        panel = panel.child(body(view, &theme, cx));
    }
    if let Some(error) = &view.attachment_error {
        panel = panel.child(
            div()
                .px(px(12.0))
                .py(px(4.0))
                .text_size(px(11.0))
                .text_color(theme.colors.danger)
                .child(t_global(upload_error_key(error))),
        );
    }
    panel.into_any_element()
}

/// Collapse/expand control for the attachment body.
fn collapse_toggle(
    view: &EditorView,
    theme: &Theme,
    cx: &mut Context<EditorView>,
) -> impl IntoElement {
    let icon = if view.attachments_open {
        gpui_kit::assets::IconName::ChevronDown
    } else {
        gpui_kit::assets::IconName::ChevronRight
    };
    div()
        .id("attachment-collapse")
        .flex()
        .items_center()
        .justify_center()
        .w(px(18.0))
        .h(px(18.0))
        .rounded(px(4.0))
        .cursor_pointer()
        .hover(|style| style.bg(theme.colors.secondary))
        .on_click(cx.listener(|this, _, _, cx| {
            cx.stop_propagation();
            this.attachments_open = !this.attachments_open;
            cx.notify();
        }))
        .child(
            Icon::new(icon)
                .with_size(px(14.0))
                .text_color(theme.colors.muted_foreground),
        )
}

/// Attach button: shows the compression label while an import is running.
fn attach_button(
    view: &EditorView,
    theme: &Theme,
    cx: &mut Context<EditorView>,
) -> impl IntoElement {
    let label = if view.attachment_busy {
        t_global("asset.compressing")
    } else {
        t_global("asset.attach")
    };
    div()
        .id("attachment-attach")
        .flex()
        .items_center()
        .gap(px(4.0))
        .px(px(8.0))
        .py(px(3.0))
        .rounded(px(4.0))
        .border_1()
        .border_color(theme.colors.border)
        .cursor_pointer()
        .hover(|style| style.bg(theme.colors.secondary))
        .on_click(cx.listener(|this, _, _, cx| {
            cx.stop_propagation();
            this.attach_files(cx);
        }))
        .child(
            Icon::new(gpui_kit::assets::IconName::Paperclip)
                .with_size(px(12.0))
                .text_color(theme.colors.foreground),
        )
        .child(
            div()
                .text_size(px(11.0))
                .text_color(theme.colors.foreground)
                .child(label),
        )
}

/// Scrollable attachment list body.
fn body(view: &EditorView, theme: &Theme, cx: &mut Context<EditorView>) -> impl IntoElement {
    let mut list = div()
        .flex()
        .flex_col()
        .max_h(px(ATTACHMENT_BODY_MAX_HEIGHT))
        .overflow_y_scrollbar();
    if view.attachments.is_empty() {
        list = list.child(
            div()
                .px(px(12.0))
                .py(px(4.0))
                .text_size(px(11.0))
                .text_color(theme.colors.muted_foreground)
                .child(t_global("asset.empty")),
        );
    }
    for (index, row) in view.attachments.iter().enumerate() {
        list = list.child(attachment_row(view, index, row, theme, cx));
    }
    list
}

/// One attachment row with the two-step delete control.
fn attachment_row(
    view: &EditorView,
    index: usize,
    row: &AttachmentRow,
    theme: &Theme,
    cx: &mut Context<EditorView>,
) -> impl IntoElement {
    let pending = view.pending_delete.as_deref() == Some(row.ref_path.as_str());
    let mut element = div()
        .flex()
        .flex_col()
        .gap(px(2.0))
        .px(px(12.0))
        .py(px(4.0))
        .border_b_1()
        .border_color(theme.colors.border)
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(
                    div()
                        .flex_1()
                        .text_size(px(12.0))
                        .text_color(theme.colors.foreground)
                        .child(row.name.clone()),
                )
                .child(mime_chip(&row.mime, theme))
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(theme.colors.muted_foreground)
                        .child(row.size_label.clone()),
                )
                .child(delete_control(index, &row.ref_path, pending, theme, cx)),
        );
    if let Some(key) = delete_confirm_key(pending) {
        element = element.child(
            div()
                .text_size(px(10.0))
                .text_color(theme.colors.warning)
                .child(t_global(key)),
        );
    }
    element
}

/// MIME chip matching the shipped `AttachmentPanel` row.
fn mime_chip(mime: &str, theme: &Theme) -> impl IntoElement {
    div()
        .px(px(4.0))
        .py(px(1.0))
        .rounded(px(3.0))
        .border_1()
        .border_color(theme.colors.border)
        .text_size(px(10.0))
        .text_color(theme.colors.muted_foreground)
        .child(mime.to_string())
}

/// Delete control; arms the row on the first click, deletes on the second.
fn delete_control(
    index: usize,
    ref_path: &str,
    pending: bool,
    theme: &Theme,
    cx: &mut Context<EditorView>,
) -> impl IntoElement {
    let ref_path = ref_path.to_string();
    let color = if pending {
        theme.colors.danger
    } else {
        theme.colors.muted_foreground
    };
    div()
        .id(("attachment-delete", index))
        .flex()
        .items_center()
        .gap(px(3.0))
        .px(px(4.0))
        .py(px(1.0))
        .rounded(px(3.0))
        .cursor_pointer()
        .text_size(px(11.0))
        .text_color(color)
        .hover(|style| style.bg(theme.colors.secondary))
        .on_click(cx.listener(move |this, _, _, cx| {
            cx.stop_propagation();
            this.click_delete(&ref_path, cx);
        }))
        .child(
            Icon::new(gpui_kit::assets::IconName::Trash)
                .with_size(px(12.0))
                .text_color(color),
        )
        .child(t_global("asset.delete"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::core::prelude::v1::test;
    use stratum_i18n::{t, Locale};

    #[test]
    fn confirm_label_only_appears_when_armed() {
        assert_eq!(delete_confirm_key(false), None);
        assert_eq!(delete_confirm_key(true), Some("asset.delete_confirm"));
        for locale in Locale::ALL {
            assert_ne!(
                t(locale, "asset.delete_confirm"),
                "asset.delete_confirm",
                "{} missing in {}",
                "asset.delete_confirm",
                locale.code()
            );
        }
    }
}
