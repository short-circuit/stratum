//! The platform runtime the shell owns.
//!
//! [`PlatformRuntime`] owns every OS handle (tray icon, global shortcut,
//! single-instance guard) and turns their events into [`crate::PlatformEvent`]s
//! that the shell polls. No UI code touches a platform API directly: the shell
//! calls [`PlatformRuntime::poll`], applies each event to its own state, and
//! calls [`PlatformRuntime::apply`] when the tray settings change.
//!
//! Every OS handle is best effort. A headless CI box, a Wayland session without
//! XWayland, or a desktop without a StatusNotifier host logs a warning and the
//! app keeps running — the tray and the shortcut are conveniences, not
//! requirements.

use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;

use crate::hotkey::{HotkeyManager, DEFAULT_SHORTCUT};
use crate::instance::InstanceGuard;
use crate::notify::{Notifier, ReminderNotice};
use crate::tray::{TrayHandle, TrayIconImage, TrayLabels};
use crate::{PlatformEvent, EVENT_POLL_INTERVAL_MS};

/// The tray, shortcut and notification preferences the shell applies.
///
/// Mirrors the effective values of `stratum_app::settings::TraySettingsDto` and
/// `ReminderSettingsDto`; the shell reads them from the settings provider.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlatformPreferences {
    /// Whether the tray icon and the global shortcut are active.
    pub tray_enabled: bool,
    /// Whether closing the window keeps the app resident in the tray.
    pub close_to_tray: bool,
    /// The quick-capture binding (`CmdOrCtrl+Shift+Space` by default).
    pub quick_capture_shortcut: String,
    /// Whether due reminders post OS notifications.
    pub notifications_enabled: bool,
    /// Whether those notifications play a sound.
    pub notification_sound: bool,
}

impl Default for PlatformPreferences {
    fn default() -> Self {
        Self {
            tray_enabled: true,
            close_to_tray: true,
            quick_capture_shortcut: DEFAULT_SHORTCUT.to_string(),
            notifications_enabled: true,
            notification_sound: true,
        }
    }
}

#[cfg(desktop)]
mod desktop {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::thread::JoinHandle;

    use crate::tray::TrayAction;

    /// The live platform runtime.
    pub struct PlatformRuntime {
        events: Receiver<PlatformEvent>,
        prefs: PlatformPreferences,
        tray: Option<TrayHandle>,
        hotkey: Option<HotkeyManager>,
        instance: Option<InstanceGuard>,
        notifier: Box<dyn Notifier>,
        shortcut_error: Option<String>,
        shutdown: Arc<AtomicBool>,
        poller: Option<JoinHandle<()>>,
    }

    impl PlatformRuntime {
        /// Start every OS capability.
        ///
        /// `labels` are the localized tray menu labels, `notification_open_label`
        /// is the localized notification action, `instance_name` is the
        /// single-instance socket name and `args` the launch argv (a second
        /// launch forwards it to the running instance and exits).
        ///
        /// Returns `None` when another instance is already running: the caller
        /// must exit without opening a window.
        pub fn start(
            labels: &TrayLabels,
            notification_open_label: &str,
            tooltip: &str,
            icon: TrayIconImage,
            prefs: PlatformPreferences,
            instance_name: &str,
            args: Vec<String>,
        ) -> Option<PlatformRuntime> {
            let (tx, rx): (Sender<PlatformEvent>, Receiver<PlatformEvent>) = channel();

            // Claim the single-instance slot before anything else: a second
            // launch forwards its argv (usually a deep link) and exits without
            // creating a second tray icon or window.
            let instance = match crate::instance::claim(instance_name, args, tx.clone()) {
                Ok(crate::instance::InstanceRole::Primary(guard)) => Some(guard),
                Ok(crate::instance::InstanceRole::Secondary) => return None,
                Err(e) => {
                    log::warn!("single instance: {e}");
                    None
                }
            };

            let notifier: Box<dyn Notifier> = Box::new(crate::notify::SystemNotifier::new(
                tx.clone(),
                notification_open_label.to_string(),
            ));

            let tray = match TrayHandle::new(labels, &icon, tooltip) {
                Ok(handle) => {
                    if let Err(e) = handle.set_visible(prefs.tray_enabled) {
                        log::warn!("tray: {e}");
                    }
                    Some(handle)
                }
                Err(e) => {
                    log::warn!("tray unavailable: {e}");
                    None
                }
            };

            let hotkey = match HotkeyManager::new() {
                Ok(manager) => Some(manager),
                Err(e) => {
                    log::warn!("global shortcut unavailable: {e}");
                    None
                }
            };

            let shutdown = Arc::new(AtomicBool::new(false));
            let poller = spawn_poller(tx.clone(), shutdown.clone());

            let mut runtime = Self {
                events: rx,
                prefs,
                tray,
                hotkey,
                instance,
                notifier,
                shortcut_error: None,
                shutdown,
                poller,
            };
            runtime.register_shortcut();
            Some(runtime)
        }

        /// Replace the preferences at runtime (the tray settings tab saved).
        pub fn apply(&mut self, prefs: PlatformPreferences) {
            self.prefs = prefs;
            if let Some(tray) = &self.tray {
                if let Err(e) = tray.set_visible(self.prefs.tray_enabled) {
                    log::warn!("tray: {e}");
                }
            }
            self.register_shortcut();
        }

        /// Drain the events the OS layer produced since the last poll.
        pub fn poll(&self) -> Vec<PlatformEvent> {
            let mut events = Vec::new();
            while let Ok(event) = self.events.try_recv() {
                events.push(event);
            }
            events
        }

        /// Post a reminder notification when notifications are enabled.
        ///
        /// Returns whether the OS accepted the notification.
        pub fn notify_reminder(&self, notice: &ReminderNotice) -> bool {
            if !self.prefs.notifications_enabled {
                return false;
            }
            match self.notifier.notify(notice) {
                Ok(()) => {
                    log::info!("notification posted: {}", notice.id);
                    true
                }
                Err(e) => {
                    log::warn!("notification: {e}");
                    false
                }
            }
        }

        /// Whether closing the window keeps the process resident.
        pub fn close_to_tray(&self) -> bool {
            crate::tray::close_to_tray(
                self.prefs.tray_enabled && self.tray.is_some(),
                self.prefs.close_to_tray,
            )
        }

        /// Whether the tray icon exists for this process.
        pub fn tray_present(&self) -> bool {
            self.tray.is_some()
        }

        /// The registered shortcut, if registration succeeded.
        pub fn shortcut(&self) -> Option<&str> {
            self.hotkey
                .as_ref()
                .and_then(|manager| manager.registered())
        }

        /// The reason the shortcut could not be registered, if it failed.
        pub fn shortcut_error(&self) -> Option<&str> {
            self.shortcut_error.as_deref()
        }

        /// The current preferences.
        pub fn preferences(&self) -> &PlatformPreferences {
            &self.prefs
        }

        fn register_shortcut(&mut self) {
            self.shortcut_error = None;
            let Some(hotkey) = self.hotkey.as_mut() else {
                return;
            };
            hotkey.unregister_all();
            if !self.prefs.tray_enabled {
                return;
            }
            match hotkey.register(&self.prefs.quick_capture_shortcut) {
                Ok(()) => log::info!(
                    "global shortcut registered: {}",
                    self.prefs.quick_capture_shortcut
                ),
                Err(e) => {
                    self.shortcut_error = Some(e.to_string());
                    log::warn!("global shortcut not registered: {e}");
                }
            }
        }
    }

    impl Drop for PlatformRuntime {
        fn drop(&mut self) {
            self.shutdown.store(true, Ordering::Relaxed);
            if let Some(poller) = self.poller.take() {
                let _ = poller.join();
            }
            if let Some(hotkey) = self.hotkey.as_mut() {
                hotkey.unregister_all();
            }
            let _ = self.instance.take();
        }
    }

    /// Poll the OS channels on a worker thread and publish shell events.
    fn spawn_poller(
        events: Sender<PlatformEvent>,
        shutdown: Arc<AtomicBool>,
    ) -> Option<JoinHandle<()>> {
        std::thread::Builder::new()
            .name("stratum-platform-events".into())
            .spawn(move || {
                while !shutdown.load(Ordering::Relaxed) {
                    drain_tray_menu(&events);
                    drain_tray_icon(&events);
                    drain_hotkeys(&events);
                    std::thread::sleep(Duration::from_millis(EVENT_POLL_INTERVAL_MS));
                }
            })
            .ok()
    }

    fn drain_tray_menu(events: &Sender<PlatformEvent>) {
        while let Ok(menu_event) = tray_icon::menu::MenuEvent::receiver().try_recv() {
            let Some(action) = TrayAction::from_id(menu_event.id.as_ref()) else {
                continue;
            };
            let event = match action {
                TrayAction::Open => PlatformEvent::ShowWindow,
                TrayAction::QuickCapture => PlatformEvent::QuickCapture { voice: false },
                TrayAction::VoiceCapture => PlatformEvent::QuickCapture { voice: true },
                TrayAction::SyncNow => PlatformEvent::SyncNow,
                TrayAction::Quit => PlatformEvent::Quit,
            };
            let _ = events.send(event);
        }
    }

    fn drain_tray_icon(events: &Sender<PlatformEvent>) {
        use tray_icon::{MouseButton, MouseButtonState, TrayIconEvent};
        while let Ok(event) = TrayIconEvent::receiver().try_recv() {
            let activate = match event {
                TrayIconEvent::DoubleClick { .. } => true,
                // The KSNI backend reports activation as a left click, so the
                // Linux tray opens the window on a single click (the shipped
                // Windows tray used a double click).
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } => true,
                _ => false,
            };
            if activate {
                let _ = events.send(PlatformEvent::ShowWindow);
            }
        }
    }

    fn drain_hotkeys(events: &Sender<PlatformEvent>) {
        use global_hotkey::{GlobalHotKeyEvent, HotKeyState};
        while let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
            if event.state() == HotKeyState::Pressed {
                let _ = events.send(PlatformEvent::QuickCapture { voice: false });
            }
        }
    }
}

#[cfg(desktop)]
pub use desktop::PlatformRuntime;

/// Non-desktop hosts (Android) have no tray, shortcut or OS notification: the
/// runtime keeps the same API and reports every capability as inactive.
#[cfg(not(desktop))]
pub struct PlatformRuntime {
    events: Receiver<PlatformEvent>,
    prefs: PlatformPreferences,
}

#[cfg(not(desktop))]
impl PlatformRuntime {
    /// Start the inert runtime.
    pub fn start(
        _labels: &TrayLabels,
        _notification_open_label: &str,
        _tooltip: &str,
        _icon: TrayIconImage,
        prefs: PlatformPreferences,
        _instance_name: &str,
        _args: Vec<String>,
    ) -> Option<PlatformRuntime> {
        let (_tx, rx) = channel();
        Some(Self { events: rx, prefs })
    }

    /// Replace the preferences (nothing to re-register on mobile).
    pub fn apply(&mut self, prefs: PlatformPreferences) {
        self.prefs = prefs;
    }

    /// Drain events (mobile has none).
    pub fn poll(&self) -> Vec<PlatformEvent> {
        let mut events = Vec::new();
        while let Ok(event) = self.events.try_recv() {
            events.push(event);
        }
        events
    }

    /// Mobile notifications are posted by the platform layer in E10.
    pub fn notify_reminder(&self, _notice: &ReminderNotice) -> bool {
        false
    }

    /// Mobile windows are never hidden into a tray.
    pub fn close_to_tray(&self) -> bool {
        false
    }

    /// Mobile has no tray.
    pub fn tray_present(&self) -> bool {
        false
    }

    /// Mobile has no global shortcut.
    pub fn shortcut(&self) -> Option<&str> {
        None
    }

    /// Mobile has no global shortcut, so there is no error to report.
    pub fn shortcut_error(&self) -> Option<&str> {
        None
    }

    /// The current preferences.
    pub fn preferences(&self) -> &PlatformPreferences {
        &self.prefs
    }
}

#[cfg(all(test, desktop))]
mod tests {
    use super::*;

    fn labels() -> TrayLabels {
        TrayLabels {
            open: "Open Stratum".into(),
            quick_capture: "Quick Capture".into(),
            voice_capture: "Voice Capture".into(),
            sync_now: "Sync Now".into(),
            quit: "Quit".into(),
        }
    }

    #[test]
    fn runtime_reports_close_to_tray_only_when_configured() {
        let icon = TrayIconImage::from_png(crate::tray::TRAY_ICON_PNG).expect("icon");
        let prefs = PlatformPreferences {
            tray_enabled: false,
            close_to_tray: true,
            quick_capture_shortcut: DEFAULT_SHORTCUT.into(),
            notifications_enabled: false,
            notification_sound: false,
        };
        let mut runtime = expect_running(PlatformRuntime::start(
            &labels(),
            "Open",
            "Stratum",
            icon,
            prefs,
            &format!("stratum-runtime-test-{}", std::process::id()),
            Vec::new(),
        ));
        assert!(
            !runtime.close_to_tray(),
            "a disabled tray cannot hide the window"
        );
        assert!(!runtime.notify_reminder(&crate::notify::reminder_notice(
            "Reminder due",
            "content",
            None,
            "p.md",
            "b",
            true
        )));
        assert!(runtime.poll().is_empty());

        runtime.apply(PlatformPreferences::default());
        assert!(runtime.preferences().tray_enabled);
    }

    #[test]
    fn primary_guard_is_owned_by_the_runtime() {
        let icon = TrayIconImage::from_png(crate::tray::TRAY_ICON_PNG).expect("icon");
        let runtime = expect_running(PlatformRuntime::start(
            &labels(),
            "Open",
            "Stratum",
            icon,
            PlatformPreferences::default(),
            &format!("stratum-runtime-owner-{}", std::process::id()),
            vec!["stratum".into()],
        ));
        assert!(runtime.poll().is_empty());
    }

    #[test]
    fn a_second_start_with_the_same_name_is_secondary() {
        let name = format!("stratum-runtime-secondary-{}", std::process::id());
        let icon = TrayIconImage::from_png(crate::tray::TRAY_ICON_PNG).expect("icon");
        let _primary = expect_running(PlatformRuntime::start(
            &labels(),
            "Open",
            "Stratum",
            icon.clone(),
            PlatformPreferences::default(),
            &name,
            Vec::new(),
        ));
        let second = PlatformRuntime::start(
            &labels(),
            "Open",
            "Stratum",
            icon,
            PlatformPreferences::default(),
            &name,
            vec!["stratum".into(), "stratum://graph".into()],
        );
        assert!(
            second.is_none(),
            "the second launch is a secondary instance"
        );
    }

    fn expect_running(start: Option<PlatformRuntime>) -> PlatformRuntime {
        start.expect("expected a running runtime")
    }
}
