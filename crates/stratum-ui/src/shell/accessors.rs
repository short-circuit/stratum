//! Read-only shell accessors shared by the desktop and mobile layouts.
//!
//! These answer questions the chrome and the panel variants ask every frame
//! (active panel, current page, route, sidebar width). They live apart from the
//! mutation methods in [`super`] so the layout modules stay small.

use crate::router::{path_for, Panel};
use crate::shell::{sidebar, Shell};

impl Shell {
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
}
