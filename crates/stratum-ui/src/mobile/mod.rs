//! Mobile shell layout: safe-area frame, bottom navigation, More sheet and one
//! variant per shipped panel.
//!
//! The mobile shell reuses the desktop provider seam, panel entities and
//! lifecycle ([`crate::shell::Shell::begin_frame`]); only the chrome differs.
//! [`render`] composes the frame in the shipped mobile order:
//!
//! 1. the top safe-area inset,
//! 2. the active panel variant,
//! 3. the error bar,
//! 4. the success toaster,
//! 5. the bottom navigation bar above the bottom safe-area inset,
//! 6. the More sheet and the Quick Capture overlay.
//!
//! [`insets`] owns the pure safe-area/IME geometry, [`nav`] owns the tab and
//! More-sheet tables plus the back stack, [`services`] is the host capability
//! seam the Android host implements, and [`variants`] wraps the live panel
//! entities rather than duplicating the desktop views.
//!
//! ## Capture affordance
//!
//! `STRATUM_MOBILE=1` selects this shell in the binary and `STRATUM_INSETS`
//! overrides the platform insets for headless captures. `STRATUM_MOBILE_MORE=1`
//! opens the More sheet on start.

use gpui_kit::component::ActiveTheme;
use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::shell::Shell;

pub mod insets;
pub mod nav;
pub mod services;
pub mod variants;

#[cfg(test)]
mod tests;

pub use insets::{
    frame_for_window, frame_from_insets, parse_insets, with_env_override, MobileFrame,
};
pub use nav::{
    bottom_nav, more_sheet, tab_for_panel, MobileNav, MobileTabDef, MoreEntryDef, MAX_BACK_STACK,
    MIN_TOUCH_TARGET, MOBILE_TABS, MORE_ENTRIES, NAV_ITEM_HEIGHT, SHEET_ROW_HEIGHT,
};
pub use services::{MobileServices, NoopServices};

/// Every translation key the mobile shell renders.
///
/// The release gate walks this list against all eight catalogs, so adding a
/// visible mobile string without a catalog entry fails the build instead of
/// leaking a raw key into the UI. Panel bodies declare their own keys in their
/// `*_I18N_KEYS` lists; this list covers the mobile chrome and the two panels
/// the mobile shell renders directly (the Kanban placeholder and the Query run
/// action).
pub const MOBILE_I18N_KEYS: [&str; 17] = [
    "kanban.kanban",
    "kanban.no_tasks_yet",
    "mobile_layout.pages",
    "mobile_nav.ask_notes",
    "mobile_nav.flashcards",
    "mobile_nav.graph",
    "mobile_nav.journal",
    "mobile_nav.kanban",
    "mobile_nav.more",
    "mobile_nav.plugins",
    "mobile_nav.query",
    "mobile_nav.reminders",
    "mobile_nav.search",
    "mobile_nav.settings",
    "mobile_nav.templates",
    "mobile_nav.whiteboards",
    "query.run_query",
];

/// Render the mobile shell frame.
///
/// `shell` must already have run its per-frame lifecycle (the caller does this
/// in [`Render`](crate::shell::Shell)). The layout reserves the platform
/// safe-area and IME geometry from [`frame_for_window`] and composes the active
/// panel variant, the error bar, the toaster, the bottom navigation bar and the
/// overlays.
pub(crate) fn render(
    shell: &mut Shell,
    window: &mut Window,
    cx: &mut Context<Shell>,
) -> AnyElement {
    let frame = insets::frame_for_window(window);
    let theme = cx.theme();
    let background = theme.colors.background;
    let foreground = theme.colors.foreground;

    let content = variants::render(shell.active_panel(), shell, window, &frame, cx);

    let mut root = div()
        .relative()
        .flex()
        .flex_col()
        .size_full()
        .bg(background)
        .text_color(foreground)
        .pl(px(frame.left))
        .pr(px(frame.right))
        .pt(px(frame.top))
        .on_action(cx.listener(|this, _: &crate::MobileBack, _window, cx| {
            this.mobile_back();
            cx.notify();
        }))
        // The Android host delivers the system back action as a committed
        // `BackGesture` platform input (see `vendor/gpui-mobile` VENDOR.md), so
        // the hardware/gesture back button walks the mobile back stack exactly
        // like the `MobileBack` shortcut does on a keyboard.
        .on_back_gesture(
            cx.listener(|this, _: &gpui::BackGestureEvent, _window, cx| {
                if this.mobile_back() {
                    cx.notify();
                }
            }),
        )
        .child(div().flex_1().overflow_hidden().child(content));

    if shell.error().is_some() {
        root = root.child(shell.render_error_bar(cx).into_any_element());
    }
    if shell.toast_message().is_some() {
        // Keep the toaster above the bottom navigation bar and the gesture area.
        let bottom = frame.bottom + nav::NAV_ITEM_HEIGHT + 8.0;
        root = root.child(shell.render_toast_at(cx, bottom).into_any_element());
    }

    root = root.child(nav::bottom_nav(shell, &frame, cx));

    if let Some(sheet) = nav::more_sheet(shell, &frame, cx) {
        root = root.child(sheet);
    }
    if shell.quick_capture_open() {
        root = root.child(shell.render_quick_capture_overlay(cx).into_any_element());
    }
    if shell.onboarding_open() {
        root = root.child(crate::shell::onboarding_overlay(shell, cx));
    }

    root.into_any_element()
}
