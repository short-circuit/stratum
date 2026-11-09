//! Sidebar rendering: brand header, navigation rows, recents and footer.
//!
//! The geometry mirrors `src/components/Sidebar/index.tsx`: 224 px expanded,
//! 52 px collapsed, a 56 px minimum header, 40 px navigation rows, and a
//! bottom footer. The collapsed rail keeps the navigation icons reachable and
//! hides the recents list, exactly like the React sidebar.

use gpui_kit::assets::IconName;
use gpui_kit::component::{ActiveTheme, Icon, Sizable};
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use crate::router::NAV_ITEMS;
use crate::shell::footer::footer;
use crate::shell::Shell;

/// Expanded sidebar width, matching the shipped `DRAWER_WIDTH`.
pub const SIDEBAR_WIDTH: f32 = 224.0;
/// Collapsed sidebar width, matching the shipped `DRAWER_COLLAPSED`.
pub const SIDEBAR_COLLAPSED_WIDTH: f32 = 52.0;
/// Alias used by the shell geometry helpers.
pub const EXPANDED_WIDTH: f32 = SIDEBAR_WIDTH;
/// Alias used by the shell geometry helpers.
pub const COLLAPSED_WIDTH: f32 = SIDEBAR_COLLAPSED_WIDTH;
/// Minimum height of the brand header.
pub const HEADER_MIN_HEIGHT: f32 = 56.0;
/// Height of one navigation row.
pub const NAV_ROW_HEIGHT: f32 = 40.0;

/// Render the complete sidebar.
pub fn sidebar(shell: &Shell, cx: &mut Context<Shell>) -> impl IntoElement {
    let theme = cx.theme();
    let background = theme.colors.secondary;
    let border = theme.colors.border;

    div()
        .flex()
        .flex_col()
        .h_full()
        .w(px(shell.sidebar_width()))
        .bg(background)
        .border_r_1()
        .border_color(border)
        .child(header(shell, cx))
        .child(nav_list(shell, cx))
        .child(div().h(px(1.0)).mx(px(16.0)).bg(border))
        .when(!shell.sidebar_collapsed(), |element| {
            element.child(recents(shell, cx))
        })
        .child(footer(shell, cx))
}

/// The brand header: app mark, vault counters and the collapse control.
fn header(shell: &Shell, cx: &mut Context<Shell>) -> impl IntoElement {
    let theme = cx.theme();
    let primary = theme.colors.primary;
    let border = theme.colors.border;
    let foreground = theme.colors.foreground;
    let muted = theme.colors.muted_foreground;
    let collapsed = shell.sidebar_collapsed();

    div()
        .flex()
        .items_center()
        .justify_between()
        .min_h(px(HEADER_MIN_HEIGHT))
        .px(px(if collapsed { 8.0 } else { 16.0 }))
        .py(px(12.0))
        .border_b_1()
        .border_color(border)
        .when(!collapsed, |element| {
            element.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(12.0))
                    .flex_1()
                    .min_w(px(0.0))
                    .child(
                        Icon::new(IconName::Library)
                            .with_size(px(48.0))
                            .text_color(primary),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(2.0))
                            .child(
                                div()
                                    .text_size(px(16.0))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(foreground)
                                    .child("stratum"),
                            )
                            .when_some(shell.snapshot().vault, |element, vault| {
                                // Matches the React caption exactly:
                                // `{blocks}{t('sidebar.b')}{pages}p`.
                                let caption = format!(
                                    "{}{}{}p",
                                    vault.block_count,
                                    t_global("sidebar.b"),
                                    vault.page_count
                                );
                                element.child(
                                    div().text_size(px(11.0)).text_color(muted).child(caption),
                                )
                            }),
                    ),
            )
        })
        .child(
            div()
                .flex()
                .items_center()
                .justify_center()
                .id("sidebar-toggle")
                .size(px(28.0))
                .rounded(px(4.0))
                .cursor_pointer()
                .on_click(cx.listener(|this, _, _, cx| {
                    this.toggle_sidebar();
                    cx.notify();
                }))
                .child(
                    Icon::new(if collapsed {
                        IconName::ChevronRight
                    } else {
                        IconName::ChevronLeft
                    })
                    .with_size(px(20.0))
                    .text_color(muted),
                ),
        )
}

/// The twelve shipped navigation rows in shipped order.
fn nav_list(shell: &Shell, cx: &mut Context<Shell>) -> impl IntoElement {
    let theme = cx.theme();
    let accent = theme.colors.accent;
    let accent_foreground = theme.colors.accent_foreground;
    let foreground = theme.colors.foreground;
    let collapsed = shell.sidebar_collapsed();
    let active = shell.active_panel();

    let mut list = div().flex().flex_col().py(px(4.0));
    for item in NAV_ITEMS {
        let selected = item.panel == active;
        let label = t_global(item.label_key);
        let row = div()
            .flex()
            .items_center()
            .id(format!("sidebar-nav-{}", item.id))
            .h(px(NAV_ROW_HEIGHT))
            .px(px(if collapsed { 8.0 } else { 16.0 }))
            .gap(px(12.0))
            .when(collapsed, |element| element.justify_center())
            .when(!collapsed, |element| element.mx(px(4.0)).rounded(px(4.0)))
            .when(selected, |element| element.bg(accent))
            .text_color(if selected {
                accent_foreground
            } else {
                foreground
            })
            .cursor_pointer()
            .on_click(cx.listener(move |this, _, _, cx| {
                this.navigate(item.panel);
                cx.notify();
            }))
            .child((item.icon)())
            .when(!collapsed, |element| {
                element.child(div().text_size(px(14.0)).child(label))
            });
        list = list.child(row);
    }

    list
}

/// The recent-page list, matching the React `PageTree` flat view.
fn recents(shell: &Shell, cx: &mut Context<Shell>) -> impl IntoElement {
    let theme = cx.theme();
    let border = theme.colors.border;
    let foreground = theme.colors.foreground;
    let muted = theme.colors.muted_foreground;

    let mut rows = div().flex().flex_col();
    if shell.snapshot().recents.is_empty() {
        rows = rows.child(
            div()
                .px(px(16.0))
                .py(px(8.0))
                .text_size(px(12.0))
                .text_color(muted)
                .child(t_global("sidebar.no_pages_yet")),
        );
    }
    for page in &shell.snapshot().recents {
        let path = page.path.clone();
        rows = rows.child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap(px(8.0))
                .id(format!("sidebar-recent-{}", page.path))
                .h(px(40.0))
                .px(px(12.0))
                .mx(px(4.0))
                .rounded(px(4.0))
                .cursor_pointer()
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.open_page(path.clone());
                    cx.notify();
                }))
                .child(
                    div()
                        .flex_1()
                        .text_size(px(13.0))
                        .text_color(foreground)
                        .child(page.title.clone()),
                )
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(muted)
                        .child(format!("{}b", page.block_count)),
                ),
        );
    }

    div()
        .flex()
        .flex_col()
        .flex_1()
        .overflow_hidden()
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .px(px(8.0))
                .pt(px(8.0))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .px(px(8.0))
                        .py(px(4.0))
                        .child(
                            Icon::new(IconName::FileText)
                                .with_size(px(12.0))
                                .text_color(muted),
                        )
                        .child(
                            div()
                                .text_size(px(11.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(muted)
                                .child(t_global("sidebar.recent").to_uppercase()),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .id("sidebar-new-page")
                        .size(px(24.0))
                        .rounded(px(4.0))
                        .cursor_pointer()
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.request_new_page();
                            cx.notify();
                        }))
                        .child(
                            Icon::new(IconName::Plus)
                                .with_size(px(16.0))
                                .text_color(muted),
                        ),
                ),
        )
        .child(div().h(px(1.0)).mx(px(8.0)).bg(border))
        .child(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    // `gpui_kit::*` re-exports gpui's `test` attribute macro, which shadows
    // the built-in attribute. Import the built-in explicitly so these plain
    // unit tests stay cheap (no GPUI app context is created).
    use crate::router::Panel;
    use ::core::prelude::v1::test;

    #[test]
    fn widths_match_the_shipped_sidebar() {
        assert_eq!(SIDEBAR_WIDTH, 224.0);
        assert_eq!(SIDEBAR_COLLAPSED_WIDTH, 52.0);
        assert_eq!(EXPANDED_WIDTH, SIDEBAR_WIDTH);
        assert_eq!(COLLAPSED_WIDTH, SIDEBAR_COLLAPSED_WIDTH);
    }

    #[test]
    fn header_and_rows_match_the_shipped_geometry() {
        assert_eq!(HEADER_MIN_HEIGHT, 56.0);
        assert_eq!(NAV_ROW_HEIGHT, 40.0);
    }

    #[test]
    fn nav_items_are_rendered_from_the_shared_table() {
        assert_eq!(NAV_ITEMS.len(), 12);
        assert!(NAV_ITEMS.iter().any(|item| item.panel == Panel::Journal));
    }
}
