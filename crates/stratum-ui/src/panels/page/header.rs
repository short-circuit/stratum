//! Page header row.
//!
//! Mirrors the shipped desktop page header: the page title, the vault-relative
//! path chip, voice-capture and attachment controls, a read-mode toggle and the
//! Note menu trigger. Rendering is thin; every action lives on
//! [`super::PageView`], so the same state machine is exercised by the
//! window-free contract tests.

use gpui_kit::assets::IconName;
use gpui_kit::component::{ActiveTheme, Icon, Sizable};
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use super::PageView;

/// Render the page header row.
pub fn header(view: &PageView, cx: &mut Context<PageView>) -> impl IntoElement {
    let theme = cx.theme();
    let border = theme.colors.border;
    let muted = theme.colors.muted_foreground;
    let accent = theme.colors.accent;
    let accent_foreground = theme.colors.accent_foreground;
    let foreground = theme.colors.foreground;

    let compact = view.compact();

    let mut title_column = div().flex().flex_col().min_w(px(0.0)).child(
        div()
            .text_size(px(16.0))
            .font_weight(FontWeight::BOLD)
            .text_color(foreground)
            .when(compact, |element| {
                element.whitespace_nowrap().text_ellipsis()
            })
            .child(page_title(view)),
    );
    if !compact {
        title_column = title_column.child(
            div()
                .mt(px(2.0))
                .px(px(6.0))
                .py(px(1.0))
                .rounded(px(4.0))
                .border_1()
                .border_color(border)
                .text_size(px(10.0))
                .text_color(muted)
                .child(view.path().to_string()),
        );
    }

    div()
        .flex()
        .items_center()
        .gap(px(8.0))
        .px(px(16.0))
        .py(px(8.0))
        .border_b_1()
        .border_color(border)
        .child(title_column)
        .child(div().flex_1())
        .child(view.voice().clone())
        .child(icon_button(
            "page-attach",
            IconName::Paperclip,
            muted,
            t_global("asset.attach"),
            compact,
            cx,
            |view, cx| view.attach_files(cx),
        ))
        .child(read_mode_button(
            view,
            accent,
            accent_foreground,
            muted,
            compact,
            cx,
        ))
        .when(view.read_mode(), |element| {
            element.child(edit_mode_button(accent, accent_foreground, compact, cx))
        })
        .child(note_menu_button(view, muted, accent, compact, cx))
}

/// The page title with the same fallback chain as the shipped panel.
fn page_title(view: &PageView) -> String {
    if let Some(info) = view.page_info() {
        if let Some(title) = &info.title {
            return title.clone();
        }
        return info.slug.clone();
    }
    view.path()
        .trim_end_matches(".md")
        .rsplit('/')
        .next()
        .unwrap_or(view.path())
        .to_string()
}

/// Read-mode toggle, mirroring the shipped prose button.
///
/// In the compact (mobile) header the label is dropped and only the icon is
/// rendered, so four controls fit beside the title at a phone width.
fn read_mode_button(
    view: &PageView,
    accent: Hsla,
    accent_foreground: Hsla,
    muted: Hsla,
    compact: bool,
    cx: &mut Context<PageView>,
) -> impl IntoElement {
    let read_mode = view.read_mode();
    div()
        .id("page-read-mode")
        .flex()
        .items_center()
        .gap(px(4.0))
        .px(px(8.0))
        .py(px(4.0))
        .rounded(px(6.0))
        .bg(if read_mode {
            accent
        } else {
            muted.opacity(0.15)
        })
        .text_color(if read_mode { accent_foreground } else { muted })
        .text_size(px(12.0))
        .cursor_pointer()
        .child(Icon::new(IconName::BookOpen).with_size(px(14.0)))
        .when(!compact, |element| {
            element.child(div().child(t_global("page.read_mode")))
        })
        .on_click(cx.listener(|view, _, _, cx| {
            view.set_read_mode(!view.read_mode(), cx);
        }))
}

/// The explicit edit-mode button shown while the page is in read mode.
fn edit_mode_button(
    accent: Hsla,
    accent_foreground: Hsla,
    compact: bool,
    cx: &mut Context<PageView>,
) -> impl IntoElement {
    div()
        .id("page-edit-mode")
        .flex()
        .items_center()
        .justify_center()
        .px(px(10.0))
        .py(px(4.0))
        .rounded(px(6.0))
        .border_1()
        .border_color(accent)
        .text_color(accent_foreground)
        .text_size(px(12.0))
        .cursor_pointer()
        .child(if compact {
            Icon::new(IconName::Pencil)
                .with_size(px(14.0))
                .into_any_element()
        } else {
            t_global("page.edit_mode").into_any_element()
        })
        .on_click(cx.listener(|view, _, _, cx| view.set_read_mode(false, cx)))
}

/// The Note menu trigger.
fn note_menu_button(
    view: &PageView,
    muted: Hsla,
    accent: Hsla,
    compact: bool,
    cx: &mut Context<PageView>,
) -> impl IntoElement {
    let label = if view.reindexing() {
        "…".to_string()
    } else {
        t_global("page.note")
    };
    div()
        .id("page-note-menu")
        .flex()
        .items_center()
        .gap(px(4.0))
        .px(px(8.0))
        .py(px(4.0))
        .rounded(px(6.0))
        .border_1()
        .border_color(muted.opacity(0.4))
        .text_color(if view.menu_open() { accent } else { muted })
        .text_size(px(12.0))
        .cursor_pointer()
        .when(!compact, |element| element.child(div().child(label)))
        .child(Icon::new(IconName::ChevronDown).with_size(px(12.0)))
        .on_click(cx.listener(|view, _, _, cx| view.toggle_menu(cx)))
}

/// A square icon button with a tooltip-like label and click handler.
fn icon_button(
    id: &'static str,
    icon: IconName,
    color: Hsla,
    label: String,
    compact: bool,
    cx: &mut Context<PageView>,
    action: impl Fn(&mut PageView, &mut Context<PageView>) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .flex()
        .items_center()
        .gap(px(4.0))
        .px(px(6.0))
        .py(px(4.0))
        .rounded(px(6.0))
        .text_size(px(11.0))
        .text_color(color)
        .cursor_pointer()
        .child(Icon::new(icon).with_size(px(14.0)))
        .when(!compact, |element| element.child(div().child(label)))
        .on_click(cx.listener(move |view, _, _, cx| action(view, cx)))
}
