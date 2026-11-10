//! Shell chrome: sidebar, header, error bar and toaster.
//!
//! [`Shell`] owns the navigation state and the data provider. Rendering is
//! split across [`sidebar`], [`footer`] and [`chrome`] so the layout contract
//! stays readable and testable. The shell deliberately renders one panel stub
//! at a time: panel bodies are the next milestone, while this crate is the
//! chrome that hosts them.
//!
//! The toaster is manual-dismiss only. The React shell auto-dismisses success
//! messages after a timeout; wiring a GPUI timer is possible but the task
//! allows the simpler behaviour, and the dismiss button keeps the toast from
//! blocking the window.

use std::sync::Arc;

use gpui_kit::component::ActiveTheme;
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::{t_global, tr_global};

use crate::panels::editor::EditorView;
use crate::panels::journal::JournalView;
use crate::panels::page::{PageStart, PageView};
use crate::panels::panel_view;
use crate::provider::ShellData;
use crate::router::{path_for, Panel, ShortcutAction};

use chrome::{error_bar, header, toast_overlay};

mod chrome;
mod footer;
mod page;
mod sidebar;

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
pub const SHELL_I18N_KEYS: [&str; 30] = [
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
    "sidebar.move_page_success",
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
    page: Option<Entity<PageView>>,
    page_editor: Option<Entity<EditorView>>,
    page_start: PageStart,
    page_read_mode: bool,
    journal: Option<Entity<JournalView>>,
    journal_start_date: Option<String>,
    journal_calendar: bool,
    journal_read_mode: bool,
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
            page: None,
            page_editor: None,
            page_start: PageStart::default(),
            page_read_mode: false,
            journal: None,
            journal_start_date: None,
            journal_calendar: false,
            journal_read_mode: false,
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

    /// Navigate to a panel, clearing the page entity when leaving it.
    pub fn navigate(&mut self, panel: Panel) {
        if panel != Panel::Page {
            self.current_page = None;
            self.page = None;
            self.page_editor = None;
            self.page_start = PageStart::default();
        }
        self.active = panel;
        self.read_mode = false;
    }

    /// The live journal entity, if the journal panel has created one.
    pub fn journal_view(&self) -> Option<&Entity<JournalView>> {
        self.journal.as_ref()
    }

    /// Set the page/journal read mode explicitly.
    ///
    /// The capture harness uses `STRATUM_READ_MODE=1` to start in read mode
    /// without synthesising a keyboard shortcut.
    pub fn set_read_mode(&mut self, read_mode: bool) {
        self.read_mode = read_mode;
    }

    /// Set the initial journal day and calendar state before the panel exists.
    ///
    /// Called by the capture harness from `STRATUM_JOURNAL_DATE` and
    /// `STRATUM_CALENDAR`. Recreating the journal entity makes a later call take
    /// effect deterministically.
    pub fn set_journal_start(&mut self, start_date: Option<String>, calendar_open: bool) {
        self.journal_start_date = start_date;
        self.journal_calendar = calendar_open;
        self.journal = None;
        self.journal_read_mode = false;
    }

    /// Route a Ctrl+click navigation request from the editor.
    ///
    /// Tags open the search panel. Wiki-links refresh the snapshot first, then
    /// resolve to a recent page path; a create-intent path (one ending in
    /// `.md`, produced by the dead-link popup) opens that page directly even
    /// when it is not in the snapshot yet. Bare unresolved slugs fall back to
    /// search.
    fn handle_editor_navigation(&mut self, target: &str) {
        if target.starts_with('#') {
            self.navigate(Panel::Search);
            return;
        }
        self.refresh();
        let resolved = self
            .snapshot
            .recents
            .iter()
            .find(|page| {
                page.path == target
                    || page.path.trim_end_matches(".md").ends_with(target)
                    || page.title == target
            })
            .map(|page| page.path.clone());
        match resolved {
            Some(path) => self.open_page(path),
            None if target.ends_with(".md") => self.open_page(target.to_string()),
            None => self.navigate(Panel::Search),
        }
    }

    /// Create the journal entity lazily, forward shell read-mode changes and
    /// route a child editor's pending navigation.
    ///
    /// The journal header owns its own read-mode toggle, so the shell only
    /// forwards a change when its own read-mode flag differs from the last value
    /// it pushed; otherwise the header toggle would be overwritten every frame.
    fn sync_journal(&mut self, cx: &mut Context<Self>) {
        if self.journal.is_none() {
            let provider = Arc::clone(&self.provider);
            let start_date = self.journal_start_date.clone();
            let calendar_open = self.journal_calendar;
            let read_mode = self.read_mode;
            self.journal = Some(cx.new(move |cx| {
                JournalView::new(provider, start_date, calendar_open, read_mode, cx)
            }));
            self.journal_read_mode = read_mode;
        }

        let journal = self.journal.clone();
        let Some(journal) = journal else {
            return;
        };

        if self.read_mode != self.journal_read_mode {
            let read_mode = self.read_mode;
            journal.update(cx, |view, cx| view.set_read_mode(read_mode, cx));
            self.journal_read_mode = read_mode;
        }

        let navigation =
            journal.update(cx, |view, _| view.pending_navigation().map(str::to_string));
        if let Some(target) = navigation {
            journal.update(cx, |view, _| view.clear_pending_navigation());
            self.handle_editor_navigation(&target);
        }
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
        if self.active == Panel::Page {
            self.sync_page(cx);
        }
        if self.active == Panel::Journal {
            self.sync_journal(cx);
        }
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
                        self.page.clone(),
                        self.journal.clone(),
                        cx,
                    ))),
            )
            .when(self.toast.is_some(), |element| {
                element.child(toast_overlay(self, cx))
            })
    }
}
