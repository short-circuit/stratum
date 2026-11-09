//! Shell chrome: sidebar, header, status bar, error bar and toaster.
//!
//! [`Shell`] owns the navigation state and the data provider. Rendering is
//! split across [`sidebar`], [`footer`] and [`status`] so the layout contract
//! stays readable and testable. The shell deliberately renders one panel stub
//! at a time: panel bodies are the next milestone, while this crate is the
//! chrome that hosts them.
//!
//! The toaster is manual-dismiss only. The React shell auto-dismisses success
//! messages after a timeout; wiring a GPUI timer is possible but the task
//! allows the simpler behaviour, and the dismiss button keeps the toast from
//! blocking the window.

use std::sync::Arc;

use gpui_kit::assets::IconName;
use gpui_kit::component::{ActiveTheme, Icon, Sizable};
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::{t_global, tr_global};

use crate::panels::panel_view;
use crate::provider::ShellData;
use crate::router::{path_for, Panel, ShortcutAction};

mod footer;
mod sidebar;
mod status;

pub use sidebar::{COLLAPSED_WIDTH, EXPANDED_WIDTH, SIDEBAR_COLLAPSED_WIDTH, SIDEBAR_WIDTH};

/// Shell-level alias for the expanded sidebar width.
pub const SHELL_WIDTH: f32 = sidebar::SIDEBAR_WIDTH;
/// Shell-level alias for the collapsed sidebar width.
pub const SHELL_COLLAPSED_WIDTH: f32 = sidebar::SIDEBAR_COLLAPSED_WIDTH;

#[cfg(test)]
mod tests;

/// Every translation key the shell renders.
///
/// The release gate walks this list against all eight catalogs, so adding a
/// visible string without a catalog entry fails the build instead of leaking a
/// raw key into the UI.
pub const SHELL_I18N_KEYS: [&str; 29] = [
    "common.refresh",
    "export.success_html",
    "mobile_nav.ask_notes",
    "mobile_nav.flashcards",
    "mobile_nav.graph",
    "mobile_nav.journal",
    "mobile_nav.kanban",
    "mobile_nav.plugins",
    "mobile_nav.query",
    "mobile_nav.reminders",
    "mobile_nav.search",
    "mobile_nav.settings",
    "mobile_nav.templates",
    "mobile_nav.whiteboards",
    "page.edit_mode",
    "page.note",
    "page.read_mode",
    "page.select_a_page_from_the_sidebar_or_create_a_new_one",
    "page.stratum_pkm",
    "sidebar.b",
    "sidebar.collapse_sidebar",
    "sidebar.expand_sidebar",
    "sidebar.export",
    "sidebar.export_failed",
    "sidebar.export_html",
    "sidebar.new_page",
    "sidebar.no_pages_yet",
    "sidebar.recent",
    "sidebar.refresh",
];

/// A transient success message shown in the toaster.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Toast {
    /// Localized message body.
    pub message: String,
}

/// The Stratum gpui shell.
pub struct Shell {
    provider: Arc<dyn ShellData>,
    active: Panel,
    current_page: Option<String>,
    sidebar_collapsed: bool,
    snapshot: crate::provider::ShellSnapshot,
    read_mode: bool,
    error: Option<String>,
    toast: Option<Toast>,
}

impl Shell {
    /// Create a shell for `provider` and read the initial snapshot.
    pub fn new(provider: Arc<dyn ShellData>) -> Self {
        let snapshot = provider.snapshot();
        Self {
            provider,
            active: Panel::Journal,
            current_page: None,
            sidebar_collapsed: false,
            snapshot,
            read_mode: false,
            error: None,
            toast: None,
        }
    }

    /// The active panel.
    pub fn active_panel(&self) -> Panel {
        self.active
    }

    /// The vault-relative path of the page open in the editor stub.
    pub fn current_page(&self) -> Option<&str> {
        self.current_page.as_deref()
    }

    /// Whether the sidebar is collapsed to its icon rail.
    pub fn sidebar_collapsed(&self) -> bool {
        self.sidebar_collapsed
    }

    /// The current snapshot.
    pub fn snapshot(&self) -> &crate::provider::ShellSnapshot {
        &self.snapshot
    }

    /// Whether the page stub is in read mode.
    pub fn read_mode(&self) -> bool {
        self.read_mode
    }

    /// The current error-bar message, if any.
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// The current toast message, if any.
    pub fn toast_message(&self) -> Option<&str> {
        self.toast.as_ref().map(|toast| toast.message.as_str())
    }

    /// The current route path, derived from the active panel and page.
    pub fn route(&self) -> String {
        path_for(self.active, self.current_page.as_deref())
    }

    /// The sidebar width for the current collapsed state.
    pub fn sidebar_width(&self) -> f32 {
        if self.sidebar_collapsed {
            sidebar::COLLAPSED_WIDTH
        } else {
            sidebar::EXPANDED_WIDTH
        }
    }

    /// Navigate to a panel, clearing the editor page when leaving it.
    pub fn navigate(&mut self, panel: Panel) {
        if panel != Panel::Page {
            self.current_page = None;
        }
        self.active = panel;
        self.read_mode = false;
    }

    /// Open a page in the editor stub.
    pub fn open_page(&mut self, path: impl Into<String>) {
        self.current_page = Some(path.into());
        self.active = Panel::Page;
        self.read_mode = false;
    }

    /// Toggle the sidebar. Returns the new collapsed state.
    pub fn toggle_sidebar(&mut self) -> bool {
        self.sidebar_collapsed = !self.sidebar_collapsed;
        self.sidebar_collapsed
    }

    /// Toggle read mode. Returns the new read-mode state.
    pub fn toggle_read_mode(&mut self) -> bool {
        self.read_mode = !self.read_mode;
        self.read_mode
    }

    /// Dismiss the error bar first, then the toast. Returns whether anything
    /// was dismissed.
    pub fn dismiss_overlay(&mut self) -> bool {
        if self.error.take().is_some() {
            return true;
        }
        self.toast.take().is_some()
    }

    /// Refresh the snapshot from the provider.
    pub fn refresh(&mut self) {
        self.snapshot = self.provider.refresh();
    }

    /// Export the vault as HTML, updating the error bar or toaster.
    ///
    /// Returns `true` when the export succeeded.
    pub fn export(&mut self) -> bool {
        match self.provider.export_html() {
            Ok(count) => {
                let count = count.to_string();
                self.toast = Some(Toast {
                    message: tr_global(
                        "export.success_html",
                        &[("count", count.as_str()), ("path", "/tmp/stratum-export")],
                    ),
                });
                true
            }
            Err(error) => {
                self.error = Some(format!("{} {error}", t_global("sidebar.export_failed")));
                false
            }
        }
    }

    /// Acknowledge the new-page button.
    ///
    /// Page creation belongs to the editor milestone; until then the shell
    /// shows the localized action name in the toaster so the button is not
    /// silently inert.
    pub fn request_new_page(&mut self) {
        self.toast = Some(Toast {
            message: t_global("sidebar.new_page"),
        });
    }

    /// Push a success toast.
    pub fn push_toast(&mut self, message: impl Into<String>) {
        self.toast = Some(Toast {
            message: message.into(),
        });
    }

    /// Show an error in the error bar.
    pub fn show_error(&mut self, message: impl Into<String>) {
        self.error = Some(message.into());
    }

    /// Apply a keyboard shortcut. Returns whether the shortcut changed state.
    pub fn apply_shortcut(&mut self, action: ShortcutAction) -> bool {
        match action {
            ShortcutAction::ToggleReadMode => {
                self.toggle_read_mode();
                true
            }
            ShortcutAction::DismissOverlay => self.dismiss_overlay(),
            ShortcutAction::ToggleSidebar => {
                self.toggle_sidebar();
                true
            }
        }
    }
}

impl Render for Shell {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let background = theme.colors.background;
        let foreground = theme.colors.foreground;

        div()
            .relative()
            .flex()
            .size_full()
            .bg(background)
            .text_color(foreground)
            .on_action(cx.listener(|this, _: &crate::ToggleReadMode, _window, cx| {
                this.toggle_read_mode();
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &crate::DismissOverlay, _window, cx| {
                this.dismiss_overlay();
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &crate::ToggleSidebar, _window, cx| {
                this.toggle_sidebar();
                cx.notify();
            }))
            .child(sidebar::sidebar(self, cx))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .h_full()
                    .overflow_hidden()
                    .child(header(self, cx))
                    .when(self.error.is_some(), |element| {
                        element.child(error_bar(self, cx))
                    })
                    .child(div().flex_1().overflow_hidden().child(panel_view(
                        self.active,
                        self.current_page.as_deref(),
                        cx,
                    )))
                    .child(status::status(self, cx)),
            )
            .when(self.toast.is_some(), |element| {
                element.child(toast_overlay(self, cx))
            })
    }
}

/// The content header. It mirrors the React page header: the localized panel
/// title on the left and the read-mode control on the right.
fn header(shell: &Shell, cx: &mut Context<Shell>) -> impl IntoElement {
    let theme = cx.theme();
    let background = theme.colors.secondary;
    let border = theme.colors.border;
    let foreground = theme.colors.foreground;
    let muted = theme.colors.muted_foreground;
    let accent = theme.colors.accent;
    let accent_foreground = theme.colors.accent_foreground;

    let title = t_global(shell.active.title_key());
    let subtitle = shell.current_page().map(str::to_string);
    let read_label = if shell.read_mode() {
        t_global("page.edit_mode")
    } else {
        t_global("page.read_mode")
    };

    div()
        .flex()
        .items_center()
        .justify_between()
        .h(px(56.0))
        .px(px(20.0))
        .bg(background)
        .border_b_1()
        .border_color(border)
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(2.0))
                .child(
                    div()
                        .text_size(px(18.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(foreground)
                        .child(title),
                )
                .when_some(subtitle, |element, path| {
                    element.child(div().text_size(px(12.0)).text_color(muted).child(path))
                }),
        )
        .when(shell.active_panel() == Panel::Page, |element| {
            element.child(
                div()
                    .id("header-read-mode")
                    .px(px(12.0))
                    .py(px(6.0))
                    .rounded(px(6.0))
                    .bg(accent)
                    .text_color(accent_foreground)
                    .text_size(px(12.0))
                    .font_weight(FontWeight::MEDIUM)
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.toggle_read_mode();
                        cx.notify();
                    }))
                    .child(read_label),
            )
        })
}

/// The error bar shown below the header. Dismissing it is also bound to Escape.
fn error_bar(shell: &Shell, cx: &mut Context<Shell>) -> impl IntoElement {
    let theme = cx.theme();
    let danger = theme.colors.danger;
    let border = theme.colors.border;

    div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(12.0))
        .px(px(20.0))
        .py(px(10.0))
        .bg(danger)
        .text_color(gpui::white())
        .border_b_1()
        .border_color(border)
        .child(
            div()
                .flex_1()
                .text_size(px(13.0))
                .child(shell.error().unwrap_or_default().to_string()),
        )
        .child(
            div()
                .id("error-dismiss")
                .flex()
                .items_center()
                .justify_center()
                .size(px(22.0))
                .rounded(px(4.0))
                .cursor_pointer()
                .on_click(cx.listener(|this, _, _, cx| {
                    this.dismiss_overlay();
                    cx.notify();
                }))
                .child(Icon::new(IconName::X).with_size(px(16.0))),
        )
}

/// The success toaster in the lower-right corner.
fn toast_overlay(shell: &Shell, cx: &mut Context<Shell>) -> impl IntoElement {
    let theme = cx.theme();
    let background = theme.colors.secondary;
    let border = theme.colors.border;
    let foreground = theme.colors.foreground;
    let muted = theme.colors.muted_foreground;

    div()
        .absolute()
        .right(px(16.0))
        .bottom(px(40.0))
        .flex()
        .items_center()
        .gap(px(12.0))
        .w(px(360.0))
        .px(px(16.0))
        .py(px(12.0))
        .rounded(px(8.0))
        .bg(background)
        .border_1()
        .border_color(border)
        .child(
            div()
                .flex_1()
                .text_size(px(13.0))
                .text_color(foreground)
                .child(shell.toast_message().unwrap_or_default().to_string()),
        )
        .child(
            div()
                .id("toast-dismiss")
                .flex()
                .items_center()
                .justify_center()
                .size(px(22.0))
                .rounded(px(4.0))
                .cursor_pointer()
                .on_click(cx.listener(|this, _, _, cx| {
                    this.toast = None;
                    cx.notify();
                }))
                .child(Icon::new(IconName::X).with_size(px(16.0)).text_color(muted)),
        )
}
