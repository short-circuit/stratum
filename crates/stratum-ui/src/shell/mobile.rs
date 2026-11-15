//! Mobile shell state transitions on [`Shell`].
//!
//! The mobile layout itself lives in [`crate::mobile`] (safe-area frame, bottom
//! navigation, More sheet and the per-panel variants). This module only owns
//! the shell-side state those surfaces drive: constructing a mobile shell,
//! installing the host services, activating a bottom tab or a More entry, and
//! walking the back stack. Keeping it here stops `shell::mod` from carrying the
//! whole mobile API next to the desktop navigation code.

use std::sync::Arc;

use crate::provider::ShellData;
use crate::router::Panel;
use crate::shell::Shell;

impl Shell {
    /// Create a shell with the mobile layout.
    ///
    /// The mobile shell shares the same provider seam, panel entities and
    /// lifecycle as the desktop shell; only the chrome (safe-area frame, bottom
    /// navigation and per-panel variants) differs. Host services default to
    /// [`crate::mobile::NoopServices`] until [`Shell::set_mobile_services`] is
    /// called by the Android host.
    pub fn new_mobile(provider: Arc<dyn ShellData>) -> Self {
        let mut shell = Self::new(provider);
        shell.mobile = true;
        shell
    }

    /// Create a mobile shell with the OS glue bridge.
    ///
    /// This is the production Android constructor: the shell renders the mobile
    /// layout and keeps the tray/hotkey/deep-link bridge available.
    pub fn with_platform_mobile(
        provider: Arc<dyn ShellData>,
        platform: crate::platform::PlatformBridge,
    ) -> Self {
        let mut shell = Self::with_platform(provider, platform);
        shell.mobile = true;
        shell
    }

    /// Install the host services used by the mobile shell.
    pub fn set_mobile_services(&mut self, services: Arc<dyn crate::mobile::MobileServices>) {
        self.mobile_services = services;
    }

    /// Whether the mobile layout is active.
    pub fn mobile(&self) -> bool {
        self.mobile
    }

    /// The host services the mobile shell can request.
    pub fn mobile_services(&self) -> &Arc<dyn crate::mobile::MobileServices> {
        &self.mobile_services
    }

    /// The mobile navigation state.
    pub fn mobile_nav(&self) -> &crate::mobile::MobileNav {
        &self.mobile_nav
    }

    /// Whether the mobile Settings tab accordion is expanded.
    pub fn mobile_settings_open(&self) -> bool {
        self.mobile_settings_open
    }

    /// Activate a bottom-navigation tab.
    ///
    /// The three primary tabs navigate directly; the More tab opens the sheet.
    pub fn mobile_activate_tab(&mut self, tab: crate::mobile::MobileTabDef) {
        match tab.target {
            Some(panel) => self.mobile_activate(panel),
            None => self.mobile_open_more(),
        }
    }

    /// Activate a panel from the More sheet.
    pub fn mobile_activate_more(&mut self, panel: Panel) {
        self.mobile_activate(panel);
    }

    /// Push the current destination and navigate to `panel`.
    fn mobile_activate(&mut self, panel: Panel) {
        if panel != self.active {
            self.mobile_nav.push(self.active);
        }
        self.navigate(panel);
        self.mobile_nav.close_more();
        self.mobile_settings_open = false;
    }

    /// Open the More sheet.
    pub fn mobile_open_more(&mut self) {
        self.mobile_nav.open_more();
    }

    /// Close the More sheet.
    pub fn mobile_close_more(&mut self) {
        self.mobile_nav.close_more();
    }

    /// Set the More sheet state (used by the `STRATUM_MOBILE_MORE` capture flag).
    pub fn mobile_set_more(&mut self, open: bool) {
        if open {
            self.mobile_nav.open_more();
        } else {
            self.mobile_nav.close_more();
        }
    }

    /// Toggle the mobile Settings tab accordion.
    pub fn mobile_toggle_settings_accordion(&mut self) {
        self.mobile_settings_open = !self.mobile_settings_open;
    }

    /// Collapse the mobile Settings tab accordion.
    pub fn mobile_close_settings_accordion(&mut self) {
        self.mobile_settings_open = false;
    }

    /// Handle the mobile back gesture.
    ///
    /// Closes the More sheet or the Settings accordion first, then pops the
    /// back stack. Returns whether anything was dismissed or navigated.
    pub fn mobile_back(&mut self) -> bool {
        if self.mobile_nav.more_open() {
            self.mobile_nav.close_more();
            return true;
        }
        if self.mobile_settings_open {
            self.mobile_settings_open = false;
            return true;
        }
        match self.mobile_nav.pop() {
            Some(entry) => {
                self.navigate(entry.panel);
                true
            }
            None => false,
        }
    }
}
