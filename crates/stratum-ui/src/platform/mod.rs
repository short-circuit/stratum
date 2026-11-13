//! Platform bridge: the OS glue the shell owns.
//!
//! [`PlatformBridge`] wraps [`stratum_platform::runtime::PlatformRuntime`] so the
//! shell never touches a platform API directly. It reads the tray, shortcut and
//! notification preferences from the settings provider, forwards the localized
//! labels into the OS layer, and exposes the event stream the shell polls once
//! per frame.
//!
//! The bridge is optional: tests and the capture harness run without one (the
//! shell then behaves exactly as before), and [`PlatformBridge::bootstrap`]
//! returns `None` when another Stratum instance is already running.

use std::sync::Arc;

use stratum_i18n::t_global;
use stratum_platform::notify::reminder_notice;
use stratum_platform::runtime::{PlatformPreferences, PlatformRuntime};
use stratum_platform::tray::{TrayIconImage, TrayLabels, TRAY_ICON_PNG};
use stratum_platform::PlatformEvent;

use crate::provider::{ReminderRow, SettingsDto, ShellData};

/// The tooltip shown on the tray icon.
pub const TRAY_TOOLTIP: &str = "Stratum";

/// The single-instance socket name.
///
/// Includes the version so an upgraded binary does not try to talk to an
/// instance of a different build; the user suffix is added by the OS layer.
pub fn instance_name() -> String {
    format!("stratum-{}", env!("CARGO_PKG_VERSION"))
}

/// The localized tray menu labels.
pub fn tray_labels() -> TrayLabels {
    TrayLabels {
        open: t_global("tray.open"),
        quick_capture: t_global("quickcapture.title"),
        voice_capture: t_global("quickcapture.voice"),
        sync_now: t_global("tray.sync_now"),
        quit: t_global("tray.quit"),
    }
}

/// The localized label of the reminder notification action.
pub fn notification_open_label() -> String {
    t_global("reminders.toast.open")
}

/// Map the settings DTO onto the platform preferences.
///
/// Mirrors `stratum_app::tray::get_effective_close_to_tray` and the shipped
/// reminder config: close-to-tray needs the tray, notifications follow
/// `reminders.enabled`, and the sound flag follows `reminders.sound`.
pub fn preferences_from(settings: &SettingsDto) -> PlatformPreferences {
    PlatformPreferences {
        tray_enabled: settings.tray.enabled,
        close_to_tray: settings.tray.close_to_tray,
        quick_capture_shortcut: settings.tray.quick_capture_shortcut.clone(),
        notifications_enabled: settings.reminders.enabled,
        notification_sound: settings.reminders.sound,
    }
}

/// The shell-owned platform bridge.
#[derive(Default)]
pub struct PlatformBridge {
    runtime: Option<PlatformRuntime>,
}

impl PlatformBridge {
    /// Start every OS capability for this process.
    ///
    /// Returns `None` when another instance already holds the single-instance
    /// slot: that instance received our argv and the caller must exit without
    /// opening a window.
    pub fn bootstrap(provider: &Arc<dyn ShellData>, args: Vec<String>) -> Option<Self> {
        Self::bootstrap_named(provider, &instance_name(), args)
    }

    /// Start every OS capability under an explicit single-instance name.
    ///
    /// The capture harness passes a per-process name so a fixture run never
    /// contends with a developer's running app.
    pub fn bootstrap_named(
        provider: &Arc<dyn ShellData>,
        instance: &str,
        args: Vec<String>,
    ) -> Option<Self> {
        let preferences = match provider.settings() {
            Ok(settings) => preferences_from(&settings),
            Err(error) => {
                log::warn!("platform: settings unavailable ({error}); using defaults");
                PlatformPreferences::default()
            }
        };
        let icon = match TrayIconImage::from_png(TRAY_ICON_PNG) {
            Ok(icon) => icon,
            Err(error) => {
                log::warn!("platform: tray icon unavailable ({error})");
                return None;
            }
        };
        if let Err(error) = stratum_platform::deep_link::register_scheme() {
            log::warn!("platform: deep-link scheme not registered ({error})");
        }
        let runtime = PlatformRuntime::start(
            &tray_labels(),
            &notification_open_label(),
            TRAY_TOOLTIP,
            icon,
            preferences,
            instance,
            args,
        )?;
        Some(Self {
            runtime: Some(runtime),
        })
    }

    /// A bridge without OS handles (tests and headless captures).
    pub fn inert() -> Self {
        Self { runtime: None }
    }

    /// Drain the OS events produced since the last poll.
    pub fn poll(&self) -> Vec<PlatformEvent> {
        match &self.runtime {
            Some(runtime) => runtime.poll(),
            None => Vec::new(),
        }
    }

    /// Apply new preferences (a tray settings save).
    pub fn apply_settings(&mut self, settings: &SettingsDto) {
        if let Some(runtime) = self.runtime.as_mut() {
            runtime.apply(preferences_from(settings));
        }
    }

    /// Post an OS notification for a due reminder.
    ///
    /// Returns whether the OS accepted it; `false` also covers a disabled
    /// notification preference and a bridge without a runtime.
    pub fn notify_reminder(&self, row: &ReminderRow) -> bool {
        let Some(runtime) = &self.runtime else {
            return false;
        };
        let sound = runtime.preferences().notification_sound;
        let notice = reminder_notice(
            &t_global("reminders.toast.due_title"),
            &row.content,
            row.page_title.as_deref(),
            &row.page_path,
            &row.block_id,
            sound,
        );
        runtime.notify_reminder(&notice)
    }

    /// Whether closing the window keeps the app resident in the tray.
    pub fn close_to_tray(&self) -> bool {
        self.runtime
            .as_ref()
            .is_some_and(PlatformRuntime::close_to_tray)
    }

    /// Whether the tray icon exists.
    pub fn tray_present(&self) -> bool {
        self.runtime
            .as_ref()
            .is_some_and(PlatformRuntime::tray_present)
    }

    /// The registered quick-capture shortcut.
    pub fn shortcut(&self) -> Option<&str> {
        self.runtime.as_ref().and_then(PlatformRuntime::shortcut)
    }

    /// The reason the shortcut could not be registered, if it failed.
    pub fn shortcut_error(&self) -> Option<&str> {
        self.runtime
            .as_ref()
            .and_then(PlatformRuntime::shortcut_error)
    }
}
