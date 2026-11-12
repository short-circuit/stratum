# Platform integration

Stratum's desktop features that reach outside the window — the system tray, the
global Quick Capture shortcut, reminder notifications, the single-instance lock,
`stratum://` deep links, the file manager and native drag-and-drop — are
implemented by the native shell itself. There is no plugin runtime and no
webview IPC in the path: the shell calls the operating system directly.

The capability set replaces the seven Tauri plugins the React app shipped with
(tray, global-shortcut, notification, dialog, opener, single-instance, log) plus
the webview-provided clipboard and drag-and-drop. The React app in `src/` and
`src-tauri/` remains the frozen reference; both front ends behave the same way.

> [!note]
> The desktop capabilities are documented here. The Android host wires the same
> capability set through its own platform layer (`vendor/gpui-mobile` +
> `crates/stratum-mobile`); see [Android app](android.md) and the
> [Android host](../development/gpui-ui.md#android-host) developer notes. The
> shared models (tray menu, shortcut validation, notification text, deep-link
> parsing, drop classification) are target-independent.

## Android capabilities

| Capability | Implementation |
|---|---|
| Microphone (dictation, voice capture) | runtime `RECORD_AUDIO` request via the platform layer, then `AudioRecord` capture |
| Reminder notifications | `NotificationManager` channel created on first use |
| File picker (attachments, import) | Storage Access Framework picker, no storage permission |
| `stratum://` deep links | intent filter on the host activity; the link routes through the shared parser |
| Clipboard | Android clipboard service |
| Safe areas, IME, lifecycle, back gesture | `WindowInsets` listener, `InputConnection` proxy, lifecycle callbacks (see the developer notes) |

## Capability matrix

| Capability | Native implementation | Linux | Windows | macOS |
|------------|-----------------------|-------|---------|-------|
| Tray icon + menu | `stratum-platform::tray` (KSNI on Linux, Win32 tray on Windows, `NSStatusItem` on macOS) | ✅ | ✅ | ✅ best-effort |
| Close to tray | window close hook in the shell (`settings.tray.close_to_tray`) | ✅ | ✅ | ✅ (hides the app) |
| Global Quick Capture shortcut | `stratum-platform::hotkey` (`global-hotkey`) | ✅ X11/XWayland | ✅ | ✅ |
| Reminder notifications | `stratum-platform::notify` (`notify-rust`; D-Bus / WinRT / `NSUserNotification`) | ✅ | ✅ | ✅ |
| Notification activation | focus window + reveal the block | ✅ | ✅ | ✅ |
| Single instance + argv forwarding | `stratum-platform::instance` (local socket) | ✅ | ✅ | ✅ |
| `stratum://` deep links | `stratum-platform::deep_link` (parsing + scheme registration) | ✅ | ✅ | scheme declared in the bundle (`Info.plist`) |
| File and folder pickers | native shell dialogs | ✅ | ✅ | ✅ |
| Reveal an exported file | `stratum-platform::opener` (`xdg-open` / `explorer /select,` / `open -R`) | ✅ | ✅ | ✅ |
| Clipboard | native shell clipboard | ✅ | ✅ | ✅ |
| Drag-and-drop of files | native drops (attachments) + drop-to-open for notes | ✅ | ✅ | ✅ |
| Logging | `log` + the shell's error bar | ✅ | ✅ | ✅ |

## System tray and close to tray

The tray menu is byte-identical to the shipped one:

| Menu item | Action |
|-----------|--------|
| Open Stratum | shows, unminimizes and focuses the window |
| Quick Capture | opens the Quick Capture dialog in text mode |
| Voice Capture | opens the Quick Capture dialog in voice mode |
| Sync Now | runs the same sync flow as **Settings → Sync** |
| Quit | stops the app |

The menu labels come from the locale catalogs (`tray.open`, `tray.sync_now`,
`tray.quit`, `quickcapture.title`, `quickcapture.voice`), so the tray follows the
selected UI language.

With **Settings → Tray → Enable tray icon** on, closing the window keeps the
process resident: the window is minimized (and the app hidden on macOS) instead
of quitting. Turn the toggle off to make the close button quit Stratum.

## Global Quick Capture shortcut

The shortcut registered at start-up is the one stored in **Settings → Tray →
Quick capture shortcut** (default `CmdOrCtrl+Shift+Space`). Pressing it opens
Quick Capture in text mode; the tray menu still offers the same action when the
binding cannot be registered.

- On Linux the shortcut is grabbed through X11, so it works in X11 and XWayland
  sessions. A pure Wayland session has no X server to grab from; registration
  then fails, the failure is logged and shown in the error bar, and the tray
  entry keeps working.
- A binding another application already owns fails the same way, with the OS
  error shown next to the **Quick capture shortcut** label.

See [Quick Capture](quick-capture.md) for the dialog itself and the capture
targets.

## Reminder notifications

When a block reminder becomes due, the shell posts a system notification in
addition to the in-app toast. Activating the notification focuses the window,
opens the page and highlights the block — the same path as the toast's **Open**
action. Notification text is localized (`reminders.toast.due_title`) and the body
follows the shipped format: the page title when known, then the block content,
capped at 200 characters.

Notifications honour **Settings → Notifications** (`reminders.enabled`) and the
sound toggle (`reminders.sound`).

## Single instance and launch arguments

A second launch never opens a second window. It hands its argv to the running
instance over a local socket and exits; the running instance applies any
`stratum://` link it received. This is what makes a browser or file-manager link
open the note or whiteboard in the app you already have open.

## Deep links

Stratum owns the `stratum://` scheme:

| Link | Opens |
|------|-------|
| `stratum://board/<board>` | that whiteboard |
| `stratum://board/<board>#<element>` | that whiteboard, focused on the element or frame |
| `stratum://page/<path>` | a note in the vault (path is percent-encoded) |
| `stratum://journal` | today's journal |
| `stratum://journal/<YYYY-MM-DD>` | that day's journal |

Whiteboard elements expose their link through the board editor's copy-link
action, so a frame or element can be shared as a `stratum://board/…#…` URL.

Scheme registration happens at start-up: a desktop entry with
`MimeType=x-scheme-handler/stratum` on Linux, the `HKCU\Software\Classes\stratum`
keys on Windows, and the bundle's `CFBundleURLTypes` on macOS (added by the
packaging phase). A registration failure is logged and never blocks start-up.

## Files: pickers, exports and drops

- **Pickers** — the import source folder, the attachment import, plugin install
  and every export row use the native file dialogs.
- **Reveal** — each export row in **Settings → Vault** has an **Open folder**
  action that reveals the written file in the OS file manager.
- **Drops** — dropping files on a block imports them as attachments. Dropping a
  Markdown note inside the vault anywhere else opens that note.

## Verifying the integration

The shell reads its start state from environment variables, so each capability
can be exercised without clicking:

```bash
STRATUM_FIXTURE=1 STRATUM_QUICK_CAPTURE=1 cargo run -p stratum-ui
STRATUM_FIXTURE=1 STRATUM_QUICK_CAPTURE_VOICE=1 cargo run -p stratum-ui
STRATUM_FIXTURE=1 STRATUM_DEEP_LINK='stratum://board/Architecture%20sketch#frame-1' \
  cargo run -p stratum-ui
```

A second `cargo run` while the first is open prints
`another Stratum instance is running; launch arguments forwarded` and exits —
that is the single-instance path.

## Troubleshooting

| Symptom | Cause | Fix |
|---------|-------|-----|
| No tray icon | the desktop has no StatusNotifier host (e.g. a bare window manager) | run a panel that implements the StatusNotifierItem protocol, or leave the tray off |
| Shortcut does nothing | a Wayland session, or another app owns the binding | check the error bar, pick another binding in Settings → Tray |
| No notification | notifications disabled in Settings, or the desktop has no notification daemon | enable them, or run a notification service |
| Deep link opens nothing | the scheme is not registered yet (first run registers it) | start the app once, then retry the link |
