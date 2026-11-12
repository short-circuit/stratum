//! OS glue for the native Stratum shell.
//!
//! The shipped desktop app leaned on seven Tauri plugins — tray,
//! global-shortcut, notification, dialog, opener, single-instance and
//! deep-link — plus webview-provided clipboard and drag-and-drop. The gpui
//! shell has no Tauri runtime, so this crate is the replacement: it owns the
//! OS-side capability glue and exposes a single event stream that the shell
//! consumes through [`runtime::PlatformRuntime`].
//!
//! Capability map (documented in `docs/guide/platform-integration.md`):
//!
//! | Capability | Provider |
//! |------------|----------|
//! | Tray icon, menu, close-to-tray | [`tray`] (`tray-icon`, KSNI backend on Linux) |
//! | Global quick-capture shortcut | [`hotkey`] (`global-hotkey`) |
//! | Reminder notifications + tap | [`notify`] (`notify-rust`) |
//! | Single instance + launch args | [`instance`] (`interprocess` local socket) |
//! | `stratum://` deep links | [`deep_link`] |
//! | Reveal / open in the OS | [`opener`] |
//! | Clipboard, file pickers, drag-drop | gpui platform layer (owned by `stratum-ui`) |
//!
//! Everything that touches an OS API sits behind `#[cfg(desktop)]`; the pure
//! models (menu layout, shortcut validation, notification text, deep-link
//! parsing, drop classification) build and test on every target so the Android
//! host (E10) reuses them without the desktop crates.
//!
//! # Events
//!
//! [`PlatformEvent`] is the only surface the UI consumes. The shell polls
//! [`runtime::PlatformRuntime::poll`] and applies each event to its own state;
//! no UI code talks to a platform API directly.

pub mod deep_link;
pub mod dragdrop;
pub mod hotkey;
pub mod instance;
pub mod notify;
pub mod opener;
pub mod runtime;
pub mod tray;

/// Everything the OS layer can hand to the shell.
///
/// The variants mirror the shipped Tauri event names (`quick-capture:open`,
/// `reminder:open`, `app:quit`) plus the tray menu ids, so the shell's
/// behaviour can be checked against the shipped app one-to-one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlatformEvent {
    /// Show, unminimize and focus the main window (tray activation).
    ShowWindow,
    /// Open the Quick Capture dialog. `voice` mirrors the shipped
    /// `quick-capture:open` payload.
    QuickCapture {
        /// Whether the dialog should start in voice-capture mode.
        voice: bool,
    },
    /// Run the same sync flow as the Settings Sync tab.
    SyncNow,
    /// Quit the app (tray menu item).
    Quit,
    /// A reminder notification was activated: focus the window and reveal the
    /// block.
    ReminderOpen {
        /// Vault-relative page path of the reminder.
        page_path: String,
        /// Block id of the reminder.
        block_id: String,
    },
    /// A `stratum://` deep link arrived (start-up argv or a second instance).
    DeepLink(deep_link::DeepLink),
}

/// How often [`runtime::PlatformRuntime::poll`] should be drained.
///
/// The OS channels are filled from foreign threads, so the shell wakes up on a
/// fixed cadence instead of subscribing to each backend's own thread.
pub const EVENT_POLL_INTERVAL_MS: u64 = 250;
