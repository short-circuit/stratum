# Quick Capture

Capture a note from anywhere without opening the main Stratum window. Quick
Capture is a desktop-only feature backed by the system tray and a global
keyboard shortcut. It writes a **block** into today's journal (or a configurable
inbox page) through the normal block path, and the `.md` file on disk is updated
so that Git sync sees it.

<!-- SCREENSHOT: [quick-capture] The Quick Capture dialog with a text input and a voice button -->

## System Tray

On desktop, Stratum runs a system tray icon. Click the tray icon and a menu
appears with:

| Menu Item | Action |
|-----------|--------|
| Open Stratum | Shows and focuses the main window |
| Quick Capture | Opens the Quick Capture dialog in text mode |
| Voice Capture | Opens the Quick Capture dialog in voice mode |
| Sync Now | Runs the same sync flow as the Settings → Sync tab |
| Quit | Saves your library and exits the app |

## Close to Tray

By default, clicking the window close button **hides** the window to the tray
instead of quitting Stratum. The app keeps running in the background so you can
re-open it from the tray icon or a shortcut. Disable this in **Settings → Tray**
if you prefer the close button to quit the app.

## Global Shortcut

The default global shortcut that opens Quick Capture is
`CmdOrCtrl+Shift+Space` (⌘⇧Space on macOS, Ctrl+Shift+Space on Windows/Linux). You
can change it in **Settings → Tray** under **Quick capture shortcut**. The
shortcut works even when the main window is hidden or another application has
focus.

## Capture Dialog

- Press **Enter** (without Shift) to save the capture. The note is written to the
  configured target page and the dialog closes.
- Press **Escape** to discard the capture and close the dialog.
- Click the **voice** button to record a voice memo instead of typing. The voice
  memo is transcribed into today's journal.
- The footer shows the **capture target** (today's journal or your inbox page)
  so you always know where a capture will land.

## Capture Targets

Quick Capture writes to one of two places, configurable in **Settings → Tray**:

| Target | Location |
|--------|----------|
| Today's journal | `journals/YYYY-MM-DD.md` (the current date's daily note) |
| Inbox page | A vault-relative page path you configure (default `Inbox.md`) |

When the target is the **Inbox page**, you can set the page path in Settings.
The page is created automatically if it does not exist.

## Settings

Open **Settings → Tray** to configure:

| Setting | Description |
|---------|-------------|
| Enable tray icon | Show or hide the system tray icon |
| Close to tray | Hide the window to tray on close instead of quitting |
| Quick capture shortcut | Global keyboard shortcut that opens Quick Capture |
| Quick capture target | Where captures are written: today's journal or an inbox page |
| Inbox page path | The vault-relative path of the inbox page (shown only when the target is the inbox) |

## Platform Notes

Quick Capture and the tray are **desktop-only**. On mobile there is no tray icon
and closing the app quits it.

## Troubleshooting

- **The tray icon does not appear on Linux.** Some Linux desktop environments
  (notably without a StatusNotifier host) do not display tray icons. Install a
  StatusNotifier host such as an AppIndicator extension, or use the global
  shortcut to open Quick Capture.
- **The global shortcut does nothing.** The shortcut may conflict with another
  application or a previous Stratum shortcut registration. Choose a different
  shortcut in **Settings → Tray** and save.
- **A capture went to the wrong page.** Check **Quick capture target** and
  **Inbox page path** in **Settings → Tray**.
