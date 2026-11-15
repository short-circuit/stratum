//! Shared button and menu widgets for the native Kanban panel.
//!
//! The header, the columns and the card menu all need the same three controls,
//! so they live here instead of being duplicated per surface.

use gpui_kit::assets::IconName;
use gpui_kit::component::{ActiveTheme, Icon, Sizable};
use gpui_kit::prelude::*;
use gpui_kit::*;

use super::KanbanView;

/// One context-menu entry.
pub(super) fn menu_item(
    id: impl Into<String>,
    label: String,
    color: Hsla,
    cx: &mut Context<KanbanView>,
    action: impl Fn(&mut KanbanView, &mut Window, &mut Context<KanbanView>) + 'static,
) -> AnyElement {
    let theme = cx.theme();
    let hover = theme.colors.secondary;
    let id = id.into();
    let debug_id = id.clone();
    div()
        .id(id)
        .debug_selector(move || debug_id.clone())
        .px(px(8.0))
        .py(px(5.0))
        .rounded(px(4.0))
        .text_size(px(11.0))
        .text_color(color)
        .cursor_pointer()
        .role(Role::Button)
        .aria_label(label.clone())
        .hover(move |style| style.bg(hover))
        .child(label)
        .on_click(cx.listener(move |view, _, window, cx| {
            cx.stop_propagation();
            action(view, window, cx);
        }))
        .into_any_element()
}

/// A text action button (optionally with a leading icon).
pub(super) fn action_button(
    id: impl Into<String>,
    label: String,
    icon: Option<IconName>,
    cx: &mut Context<KanbanView>,
    action: impl Fn(&mut KanbanView, &mut Window, &mut Context<KanbanView>) + 'static,
) -> AnyElement {
    let theme = cx.theme();
    let background = theme.colors.muted_foreground.opacity(0.12);
    let foreground = theme.colors.foreground;
    let id = id.into();
    let debug_id = id.clone();
    div()
        .id(id)
        .debug_selector(move || debug_id.clone())
        .flex()
        .items_center()
        .gap(px(4.0))
        .px(px(8.0))
        .py(px(4.0))
        .rounded(px(5.0))
        .bg(background)
        .text_color(foreground)
        .text_size(px(10.5))
        .cursor_pointer()
        .role(Role::Button)
        .aria_label(label.clone())
        .when_some(icon, |element, icon| {
            element.child(Icon::new(icon).with_size(px(11.0)))
        })
        .child(label)
        .on_click(cx.listener(move |view, _, window, cx| action(view, window, cx)))
        .into_any_element()
}

/// A square icon-only button.
pub(super) fn icon_button(
    id: impl Into<String>,
    icon: IconName,
    label: String,
    cx: &mut Context<KanbanView>,
    action: impl Fn(&mut KanbanView, &mut Window, &mut Context<KanbanView>) + 'static,
) -> AnyElement {
    let theme = cx.theme();
    let foreground = theme.colors.muted_foreground;
    let id = id.into();
    let debug_id = id.clone();
    div()
        .id(id)
        .debug_selector(move || debug_id.clone())
        .flex()
        .items_center()
        .justify_center()
        .p(px(2.0))
        .rounded(px(4.0))
        .text_color(foreground)
        .cursor_pointer()
        .role(Role::Button)
        .aria_label(label)
        .child(Icon::new(icon).with_size(px(12.0)))
        .on_click(cx.listener(move |view, _, window, cx| action(view, window, cx)))
        .into_any_element()
}
