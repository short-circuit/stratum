# Reminders

You can set a **due date/time on any block**. When the time arrives, Stratum
shows an OS notification and an in-app toast; clicking it takes you straight to
that block and highlights it. You can then snooze or clear the reminder.

Reminders are **fully offline** and **no background daemon** — the scheduler
runs inside the app process while Stratum is open.

## Why reminders?

Use them to remember anything attached to a note: "follow up with Ana",
"ship the release notes", "renew the domain". Because a reminder is just a
property on a block, it travels with the note through Markdown, search, backlinks,
and Git sync — there's no separate database to lose.

## The `.due` property

A reminder is stored as a plain block property, exactly like a marker or
priority:

```markdown
- Follow up with Ana
  .due: 2026-10-15T07:00:00+00:00
```

> [!note]
> The `.due` value is always stored as an **RFC3339 UTC** timestamp. The UI
> shows and collects times in your local timezone and converts before saving.

The parser also accepts hand-written naive local timestamps (e.g.
`.due: 2026-10-15T09:00` or `.due: 2026-10-15T09:00:00`), interpreting them as
your local time. All storage and comparison happen in UTC.

## Setting, changing, snoozing, completing

- **Set a reminder**: right-click a block → **Set reminder…**, pick a date/time,
  or use a quick option (In 1 hour, Tomorrow 09:00, This evening 18:00).
- **Change**: right-click → **Change reminder…**.
- **Clear**: right-click → **Clear reminder** (removes the `.due` property).
- **Snooze**: from the reminder panel, **Snooze 10 min** or **Snooze 1 h**.
- **Complete**: from the reminder panel, **Complete** clears the reminder.

Mobile: tap a block's reminder chip to open the same dialog.

## How notifications work

- The scheduler **scans every 30 seconds** for blocks whose `.due` time has
  passed.
- Only blocks **not** marked `DONE`/`CANCELLED` remind you.
- Each reminder fires **once** per due time (per app session). If the app is
  closed at the due time, it fires on next launch (overdue reminders re-fire,
  which is intended).
- Clicking the OS notification opens Stratum, focuses the main window, and
  navigates to the block, briefly flashing it.
- A **sound** is played if "Notification sound" is enabled in settings.

## The Reminders panel

The **Reminders** panel (in the sidebar / mobile "More" menu) lists all blocks
that carry a `.due`. It splits them into **Overdue** (red), **Today** (amber),
and **Upcoming** (gray), each with the page title and when it's due, plus
actions to **Snooze**, **Complete**, or **Open** the block.

## Settings

- **Notifications** tab: **Enable reminders** (turn the scheduler + notifications
  on/off) and **Notification sound**.

> [!info]
> Turning off "Enable reminders" stops the scheduler. Reminders you already set
> are kept — they just won't notify until you re-enable.

## Limitations

- Reminders only fire **while the app is running** (no background daemon).
- On mobile, notifications follow the platform's rules; the in-app toast works
  while the app is in the foreground.

## Related

- [Tasks](tasks.md) — combining reminders with task markers
- [Block Editor](block-editor.md) — editing blocks and properties
- [Configuration](../getting-started/configuration.md) — reminder settings
