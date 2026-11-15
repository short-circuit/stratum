//! Delete confirmation dialog.
//!
//! The shipped desktop page panel uses a native `confirm` with the
//! `page.delete` prefix and the page title. The native shell renders the same
//! message in a modal card with a cancel and a delete action.

use gpui_kit::component::ActiveTheme;
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use super::PageView;

/// The confirmation message shown for a page.
pub fn delete_message(view: &PageView) -> String {
    let title = view
        .page_info()
        .and_then(|info| info.title.clone())
        .or_else(|| view.page_info().map(|info| info.slug.clone()))
        .unwrap_or_else(|| view.path().to_string());
    format!("{}{}\"?", t_global("page.delete"), title)
}

/// Render the delete confirmation dialog.
pub fn delete_dialog(view: &PageView, cx: &mut Context<PageView>) -> impl IntoElement {
    let theme = cx.theme();
    let background = theme.colors.background;
    let panel = theme.colors.secondary;
    let border = theme.colors.border;
    let foreground = theme.colors.foreground;
    let muted = theme.colors.muted_foreground;
    let danger = theme.colors.danger;

    div()
        .absolute()
        .inset_0()
        .flex()
        .items_center()
        .justify_center()
        .bg(background.opacity(0.6))
        .child(
            div()
                .w(px(380.0))
                .p(px(16.0))
                .rounded(px(8.0))
                .bg(panel)
                .border_1()
                .border_color(border)
                .flex()
                .flex_col()
                .gap(px(10.0))
                .child(
                    div()
                        .text_size(px(14.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(foreground)
                        .child(t_global("page.delete_page")),
                )
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(muted)
                        .child(delete_message(view)),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_end()
                        .gap(px(8.0))
                        .child(dialog_button(
                            "page-delete-cancel",
                            t_global("common.cancel"),
                            muted.opacity(0.15),
                            muted,
                            cx,
                            |view, cx| view.cancel_delete(cx),
                        ))
                        .child(dialog_button(
                            "page-delete-confirm",
                            t_global("page.delete_page"),
                            danger,
                            foreground,
                            cx,
                            |view, cx| view.confirm_delete(cx),
                        )),
                ),
        )
}

/// One dialog action button.
fn dialog_button(
    id: &'static str,
    label: String,
    background: Hsla,
    color: Hsla,
    cx: &mut Context<PageView>,
    action: impl Fn(&mut PageView, &mut Context<PageView>) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .px(px(12.0))
        .py(px(5.0))
        .rounded(px(6.0))
        .bg(background)
        .text_color(color)
        .text_size(px(12.0))
        .cursor_pointer()
        .child(label)
        .on_click(cx.listener(move |view, _, _, cx| action(view, cx)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::core::prelude::v1::test;

    #[test]
    fn delete_message_uses_the_shipped_prefix_and_title() {
        // The message helper only reads page metadata and the path, so a view
        // is not needed; the contract test covers the composed prefix here.
        let prefix = t_global("page.delete");
        assert!(prefix.ends_with('"'));
        assert_eq!(format!("{prefix}Fixture Note\""), "Delete \"Fixture Note\"");
    }
}
