//! Shell chrome: sidebar, header, error bar and toaster.
//!
//! [`Shell`] owns the navigation state and the data provider. Rendering is
//! split across [`sidebar`], [`footer`] and [`chrome`] so the layout contract
//! stays readable and testable. The shell deliberately renders one panel stub
//! at a time: panel bodies are the next milestone, while this crate is the
//! chrome that hosts them.
//!
//! The toaster state, due-reminder polling and toast actions live in
//! [`reminder_toast`]; the shell only wires the overlay into the layout.

use std::sync::Arc;
use std::time::Instant;

use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::{t_global, tr_global};

use crate::panels::ask::AskView;
use crate::panels::canvas::CanvasView;
use crate::panels::editor::EditorView;
use crate::panels::flashcards::FlashcardsView;
use crate::panels::graph::{GraphStart, GraphView};
use crate::panels::journal::JournalView;
use crate::panels::kanban::KanbanView;
use crate::panels::page::{PageStart, PageView};
use crate::panels::plugins::PluginsView;
use crate::panels::query::QueryView;
use crate::panels::reminders::RemindersView;
use crate::panels::search::SearchView;
use crate::panels::settings::SettingsView;
use crate::panels::templates::TemplatesView;
use crate::provider::ShellData;
use crate::router::{Panel, ShortcutAction};

use chrome::error_bar;

mod accessors;
mod chrome;
mod desktop;
mod footer;
mod mobile;
mod onboarding;
pub(crate) use onboarding::overlay as onboarding_overlay;
mod page;
mod panels;
mod platform;
mod quick_capture;
mod reminder_toast;
mod sidebar;

pub use onboarding::{Onboarding, Step as OnboardingStep, ONBOARDING_I18N_KEYS};
pub use panels::PanelStarts;
pub use platform::settings_fingerprint;
pub use quick_capture::QuickCapture;
pub use reminder_toast::{Toast, ToastAction};
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
pub const SHELL_I18N_KEYS: [&str; 32] = [
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
    "reminders.toast.due_title",
    "reminders.toast.open",
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
    search: Option<Entity<SearchView>>,
    query: Option<Entity<QueryView>>,
    ask: Option<Entity<AskView>>,
    templates: Option<Entity<TemplatesView>>,
    flashcards: Option<Entity<FlashcardsView>>,
    reminders: Option<Entity<RemindersView>>,
    plugins: Option<Entity<PluginsView>>,
    graph: Option<Entity<GraphView>>,
    whiteboards: Option<Entity<CanvasView>>,
    settings: Option<Entity<SettingsView>>,
    kanban: Option<Entity<KanbanView>>,
    panel_starts: PanelStarts,
    error: Option<String>,
    /// Reminder toaster state: the current toast, the announce-once set and the
    /// 30 s poll gate.
    toaster: reminder_toast::ReminderToaster,
    /// The OS glue bridge. Absent in tests and headless captures, where the
    /// shell behaves exactly as it did before the platform layer existed.
    platform: Option<crate::platform::PlatformBridge>,
    /// Quick Capture dialog state.
    quick_capture: quick_capture::QuickCapture,
    /// A pending Quick Capture request, opened on the next render.
    pending_quick_capture: Option<bool>,
    /// The applied Settings generation, used to re-apply platform preferences.
    platform_generation: u64,
    /// The last platform-relevant settings fingerprint we applied.
    platform_fingerprint: Option<String>,
    /// When the platform preferences were last re-read from the settings.
    platform_settings_check: Option<Instant>,
    /// Whether the close-to-tray hook and the platform wake-up tick are live.
    platform_watch_started: bool,
    /// OS events drained by the wake-up tick, applied on the next render (the
    /// tick has no window, and `activate_window` needs one).
    pending_platform: Vec<stratum_platform::PlatformEvent>,
    /// Whether the mobile layout is active.
    mobile: bool,
    /// Host capabilities the mobile shell can request (mic, picker, notify,
    /// clipboard, external links). Defaults to [`crate::mobile::NoopServices`].
    mobile_services: Arc<dyn crate::mobile::MobileServices>,
    /// Mobile navigation state: the back stack and the More sheet.
    mobile_nav: crate::mobile::MobileNav,
    /// Whether the mobile Settings tab accordion is expanded.
    mobile_settings_open: bool,
    /// First-run onboarding wizard state.
    onboarding: onboarding::Onboarding,
}

impl Shell {
    /// Create a shell for `provider` and read the initial snapshot.
    pub fn new(provider: Arc<dyn ShellData>) -> Self {
        let snapshot = provider.snapshot();
        // The first-run wizard is seeded before the shell owns the provider, so
        // the query runs while the handle is still borrowed here.
        let onboarding_pending = provider.onboarding_pending();
        let vault_path = provider.vault_path();
        let mut shell = Self {
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
            search: None,
            query: None,
            ask: None,
            templates: None,
            flashcards: None,
            reminders: None,
            plugins: None,
            graph: None,
            whiteboards: None,
            settings: None,
            kanban: None,
            panel_starts: PanelStarts::default(),
            error: None,
            toaster: reminder_toast::ReminderToaster::default(),
            platform: None,
            quick_capture: quick_capture::QuickCapture::default(),
            pending_quick_capture: None,
            platform_generation: 0,
            platform_fingerprint: None,
            platform_settings_check: None,
            platform_watch_started: false,
            pending_platform: Vec::new(),
            mobile: false,
            mobile_services: Arc::new(crate::mobile::NoopServices),
            mobile_nav: crate::mobile::MobileNav::default(),
            mobile_settings_open: false,
            onboarding: onboarding::Onboarding::default(),
        };
        shell.seed_onboarding(vault_path, onboarding_pending);
        shell
    }

    /// Create a shell with the OS glue bridge.
    ///
    /// Production uses this constructor; [`Shell::new`] stays the plain shell
    /// for tests and the capture harness.
    pub fn with_platform(
        provider: Arc<dyn ShellData>,
        platform: crate::platform::PlatformBridge,
    ) -> Self {
        let mut shell = Self::new(provider);
        shell.platform = Some(platform);
        shell
    }

    /// Whether the Quick Capture dialog is open.
    pub fn quick_capture_open(&self) -> bool {
        self.quick_capture.is_open()
    }

    /// Navigate to a panel, clearing the page entity when leaving it.
    pub fn navigate(&mut self, panel: Panel) {
        if panel != Panel::Page {
            self.current_page = None;
            self.page = None;
            self.page_editor = None;
            self.page_start = PageStart::default();
        }
        if panel != Panel::Search {
            self.search = None;
            self.panel_starts.search = crate::panels::search::SearchStart::default();
        }
        if panel != Panel::Query {
            self.query = None;
            self.panel_starts.query = crate::panels::query::QueryStart::default();
        }
        if panel != Panel::AskNotes {
            self.ask = None;
            self.panel_starts.ask = crate::panels::ask::AskStart::default();
        }
        if panel != Panel::Templates {
            self.templates = None;
        }
        if panel != Panel::Flashcards {
            self.flashcards = None;
            self.panel_starts.flashcards = crate::panels::flashcards::FlashcardsStart::default();
        }
        if panel != Panel::Reminders {
            self.reminders = None;
            self.panel_starts.reminders = crate::panels::reminders::RemindersStart::default();
        }
        if panel != Panel::Plugins {
            self.plugins = None;
            self.panel_starts.plugins = crate::panels::plugins::PluginsStart::default();
        }
        if panel != Panel::Graph {
            self.graph = None;
            self.panel_starts.graph = GraphStart::default();
        }
        if panel != Panel::Whiteboards {
            self.whiteboards = None;
            self.panel_starts.canvas = crate::panels::canvas::CanvasStart::default();
        }
        if panel != Panel::Settings {
            self.settings = None;
            self.panel_starts.settings = crate::panels::settings::SettingsStart::default();
        }
        if panel != Panel::Kanban {
            self.kanban = None;
            self.panel_starts.kanban = crate::panels::kanban::KanbanStart::default();
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
            self.set_search_start(Some(target.to_string()));
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

    /// Dismiss the topmost overlay: the Quick Capture dialog, then the error
    /// bar, then the toast. Returns whether anything was dismissed.
    pub fn dismiss_overlay(&mut self) -> bool {
        if self.dismiss_quick_capture() {
            return true;
        }
        if self.error.take().is_some() {
            return true;
        }
        self.toaster.toast.take().is_some()
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
                self.toaster.toast = Some(Toast::message(tr_global(
                    "export.success_html",
                    &[("count", count.as_str()), ("path", "/tmp/stratum-export")],
                )));
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
        self.toaster.toast = Some(Toast::message(t_global("sidebar.new_page")));
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

impl Shell {
    /// Run the per-frame lifecycle shared by the desktop and mobile shells.
    ///
    /// This drains the platform event queue, opens a pending Quick Capture
    /// dialog, creates the active panel entities and polls due reminders. Both
    /// shells call it before laying out their chrome so the panel contract stays
    /// identical.
    pub(crate) fn begin_frame(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.platform.is_some() {
            self.install_platform_watch(window, cx);
            self.poll_platform(window, cx);
            self.sync_platform_settings(cx);
        }
        if let Some(voice) = self.pending_quick_capture.take() {
            self.open_quick_capture(voice, window, cx);
        }
        if self.active == Panel::Page {
            self.sync_page(cx);
        }
        if self.active == Panel::Journal {
            self.sync_journal(cx);
        }
        self.sync_panels(window, cx);
        if self.onboarding.open {
            self.sync_onboarding_input(window, cx);
        }
        self.ensure_reminder_watch(cx);
        self.maybe_poll_reminders(cx);
    }

    /// Render the error bar (shared by the desktop and mobile shells).
    pub(crate) fn render_error_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        error_bar(self, cx)
    }

    /// Render the success toaster at `bottom` logical pixels from the viewport
    /// bottom (shared by the desktop and mobile shells).
    pub(crate) fn render_toast_at(&self, cx: &mut Context<Self>, bottom: f32) -> impl IntoElement {
        chrome::toast_overlay_at(self, cx, bottom)
    }

    /// Render the Quick Capture overlay (shared by both shells).
    pub(crate) fn render_quick_capture_overlay(&self, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_quick_capture(cx)
    }
}

impl Render for Shell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.begin_frame(window, cx);
        if self.mobile {
            return crate::mobile::render(self, window, cx).into_any_element();
        }
        self.render_desktop(cx)
    }
}
