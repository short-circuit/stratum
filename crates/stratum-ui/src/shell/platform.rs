//! Platform event handling for the shell.
//!
//! The shell owns a [`crate::platform::PlatformBridge`] in production. Once per
//! frame it drains the OS event queue ([`Shell::poll_platform`]) and applies each
//! event to its own navigation/overlay state, so no UI code talks to a platform
//! API. A background tick wakes the shell every
//! [`stratum_platform::EVENT_POLL_INTERVAL_MS`] so tray clicks, shortcut presses,
//! deep links and notification activations are handled promptly even when the
//! window is idle.
//!
//! Close-to-tray is installed on the window itself: when the tray settings ask
//! for it, closing the window minimizes it (and hides the app on macOS) instead
//! of quitting, exactly like the shipped Tauri app.

use std::time::{Duration, Instant};

use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;
use stratum_platform::deep_link::{route, DeepLink};
use stratum_platform::{PlatformEvent, EVENT_POLL_INTERVAL_MS};

use crate::panels::canvas::CanvasStart;
use crate::provider::SettingsDto;
use crate::router::{parse_target, Panel};
use crate::shell::{Shell, Toast};

/// How often the shell re-reads the settings to pick up changes made outside the
/// Settings panel (for example after a vault switch).
const SETTINGS_REFRESH: Duration = Duration::from_secs(30);

impl Shell {
    /// Install the close-to-tray hook and the platform wake-up tick.
    ///
    /// Only called when the shell has a platform bridge, so tests and headless
    /// captures keep the plain window behaviour.
    pub(super) fn install_platform_watch(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.platform_watch_started {
            return;
        }
        self.platform_watch_started = true;
        self.install_close_hook(window, cx);
        // A shortcut that another app already owns is otherwise silent: the tray
        // tab only holds the binding text, so surface the registration failure in
        // the error bar like the shipped app surfaced tray errors.
        if let Some(error) = self.platform_shortcut_error() {
            self.show_error(format!("{} — {error}", t_global("settings.tray_shortcut")));
        }
        cx.spawn(
            async move |this: WeakEntity<Shell>, cx: &mut AsyncApp| loop {
                cx.background_executor()
                    .timer(Duration::from_millis(EVENT_POLL_INTERVAL_MS))
                    .await;
                // The tick has no window, so it only queues the events and wakes
                // the shell when there is something to apply; an idle app does
                // not repaint just because the timer fired.
                let queued = this.update(cx, |shell, cx| {
                    let queued = shell.queue_platform_events();
                    if queued {
                        cx.notify();
                    }
                    queued
                });
                if queued.is_err() {
                    break;
                }
            },
        )
        .detach();
    }

    /// Keep the process resident when the window is closed and close-to-tray is
    /// enabled.
    fn install_close_hook(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let this = cx.entity().downgrade();
        window.on_window_should_close(cx, move |window, cx| {
            let close_to_tray = this
                .read_with(cx, |shell, _| {
                    shell
                        .platform
                        .as_ref()
                        .is_some_and(|bridge| bridge.close_to_tray())
                })
                .unwrap_or(false);
            if !close_to_tray {
                return true;
            }
            // The gpui platform layer has no window-hide primitive: minimizing
            // keeps the window (and the process) alive, and the tray "Open"
            // item raises it again. macOS additionally hides the whole app,
            // which is the platform's own close-to-tray convention.
            window.minimize_window();
            #[cfg(target_os = "macos")]
            cx.hide();
            false
        });
    }

    /// Move the OS events produced since the last poll into the pending queue.
    ///
    /// Returns whether anything was queued. The wake-up tick calls this without a
    /// window; the events are applied by [`Shell::poll_platform`] on the next
    /// render, where `activate_window` is available.
    fn queue_platform_events(&mut self) -> bool {
        let Some(bridge) = &self.platform else {
            return false;
        };
        let events = bridge.poll();
        if events.is_empty() {
            return false;
        }
        self.pending_platform.extend(events);
        true
    }

    /// Drain and apply the OS events produced since the last frame.
    pub(super) fn poll_platform(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.platform.is_none() {
            return;
        }
        self.queue_platform_events();
        if self.pending_platform.is_empty() {
            return;
        }
        let events = std::mem::take(&mut self.pending_platform);
        for event in events {
            self.apply_platform_event(event, window, cx);
        }
        cx.notify();
    }

    /// Apply one OS event.
    fn apply_platform_event(
        &mut self,
        event: PlatformEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            PlatformEvent::ShowWindow => window.activate_window(),
            PlatformEvent::QuickCapture { voice } => {
                window.activate_window();
                self.request_quick_capture(voice);
            }
            PlatformEvent::SyncNow => match self.provider.sync_now() {
                Ok(_) => {
                    self.toaster.toast = Some(Toast::message(t_global("settings.sync_completed")));
                }
                Err(error) => {
                    self.show_error(format!("{} {error}", t_global("settings.sync_failed")));
                }
            },
            PlatformEvent::Quit => cx.quit(),
            PlatformEvent::ReminderOpen {
                page_path,
                block_id,
            } => {
                window.activate_window();
                self.open_page_with_highlight(page_path, block_id);
            }
            PlatformEvent::DeepLink(link) => {
                window.activate_window();
                self.apply_deep_link(&link);
            }
        }
    }

    /// Route a `stratum://` link through the shared router.
    ///
    /// Board links carry the element fragment, so a `Board#Element` link opens
    /// the whiteboard on that element; page links open the editor; journal links
    /// select the day.
    pub fn apply_deep_link(&mut self, link: &DeepLink) {
        match link {
            DeepLink::Page { path } => self.open_page(path.clone()),
            DeepLink::Journal { date } => {
                self.set_journal_start(date.clone(), false);
                self.navigate(Panel::Journal);
            }
            DeepLink::Board { .. } | DeepLink::Panel { .. } => {
                let target = match parse_target(&route(link)) {
                    Some(target) => target,
                    None => {
                        log::warn!("deep link: no route for {link:?}");
                        return;
                    }
                };
                self.navigate(target.panel);
                if target.panel == Panel::Whiteboards {
                    self.set_canvas_start(CanvasStart::from_target(&target));
                }
            }
        }
    }

    /// Re-apply the tray/shortcut/notification preferences when the settings
    /// changed.
    ///
    /// The Settings panel exposes a monotonic `saved_generation` counter, so a
    /// save is applied on the next frame. A 30 s re-read catches changes made
    /// while the panel is not open (a vault switch loads a different config).
    pub(super) fn sync_platform_settings(&mut self, cx: &mut Context<Self>) {
        if self.platform.is_none() {
            return;
        }
        let generation = self
            .settings
            .as_ref()
            .map(|view| view.read(cx).saved_generation());
        if let Some(generation) = generation {
            if generation != self.platform_generation {
                self.platform_generation = generation;
                self.apply_platform_preferences();
                return;
            }
        }

        let now = Instant::now();
        let due = self
            .platform_settings_check
            .is_none_or(|last| now.duration_since(last) >= SETTINGS_REFRESH);
        if !due {
            return;
        }
        self.platform_settings_check = Some(now);
        if let Ok(settings) = self.provider.settings() {
            let fingerprint = settings_fingerprint(&settings);
            if self.platform_fingerprint.as_deref() != Some(fingerprint.as_str()) {
                self.platform_fingerprint = Some(fingerprint);
                self.apply_platform_preferences();
            }
        }
    }

    /// Push the current settings into the platform bridge.
    fn apply_platform_preferences(&mut self) {
        let Ok(settings) = self.provider.settings() else {
            return;
        };
        self.platform_fingerprint = Some(settings_fingerprint(&settings));
        if let Some(bridge) = self.platform.as_mut() {
            bridge.apply_settings(&settings);
        }
    }

    /// Open a note dropped from the OS file manager.
    ///
    /// gpui delivers native drops; the editor owns the drops that land on a
    /// block (it imports them as attachments). This is the shell-level rule the
    /// shipped webview never had: a Markdown note inside the vault opens in the
    /// editor. Assets that miss the editor are ignored, and a note outside the
    /// vault is ignored too.
    ///
    /// Returns whether a page was opened.
    pub fn handle_drop(&mut self, paths: &[std::path::PathBuf], cx: &mut Context<Self>) -> bool {
        let vault = match self.provider.settings() {
            Ok(settings) => std::path::PathBuf::from(settings.vault_path),
            Err(_) => return false,
        };
        for item in stratum_platform::dragdrop::classify(&vault, paths) {
            if let stratum_platform::dragdrop::DroppedItem::Page(path) = item {
                self.open_page(path);
                cx.notify();
                return true;
            }
        }
        false
    }

    /// The shortcut state, for the Settings Tray tab and the error bar.
    pub fn platform_shortcut_error(&self) -> Option<String> {
        self.platform
            .as_ref()
            .and_then(|bridge| bridge.shortcut_error().map(str::to_string))
    }

    /// Whether the tray icon exists in this session.
    pub fn tray_present(&self) -> bool {
        self.platform
            .as_ref()
            .is_some_and(|bridge| bridge.tray_present())
    }
}

/// A compact fingerprint of the platform-relevant settings.
pub fn settings_fingerprint(settings: &SettingsDto) -> String {
    format!(
        "{}|{}|{}|{}|{}",
        settings.tray.enabled,
        settings.tray.close_to_tray,
        settings.tray.quick_capture_shortcut,
        settings.reminders.enabled,
        settings.reminders.sound
    )
}
