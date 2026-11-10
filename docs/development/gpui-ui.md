# Native GPUI Shell

Stratum ships two front ends:

- The frozen React + Tauri application in `src/` and `src-tauri/`, which remains
  the reference implementation and is not modified by the native shell work.
- The native GPUI shell in `crates/stratum-ui`, which reproduces the React
  shell's chrome (sidebar, navigation, recents, footer, error bar and
  toaster) on top of the headless service layer.

`crates/stratum-i18n` provides the locale catalogs the native shell renders.
Both crates are workspace members and are covered by the GPUI CI workflow.

---

## Crate Layout

```
crates/
├── stratum-i18n/
│   ├── locales/            # byte-identical copies of src/i18n/locales/*.json
│   ├── src/lib.rs          # catalogs, interpolation, parity helpers
│   └── tests/              # catalog + parity gates
└── stratum-ui/
    ├── assets/             # embedded SVG icons, Inter faces, OFL notice
    ├── src/
    │   ├── lib.rs          # crate root, actions, init
    │   ├── main.rs         # binary entry point (1280×720 window), start-state flags
    │   ├── assets.rs       # embedded asset source + font registration
    │   ├── provider/
    │   │   ├── mod.rs      # ShellData seam and provider selection
    │   │   ├── assets.rs   # attachment/thumbnail seam
    │   │   ├── journal.rs  # journal date discovery, creation and DTO mapping
    │   │   ├── voice.rs    # voice-capture seam (start/stop/transcribe/insert)
    │   │   ├── fixture/    # deterministic fixture provider (pages, blocks, journal)
    │   │   └── tests.rs    # provider contract tests
    │   ├── router.rs       # routes, navigation, shortcuts, path codec
    │   ├── theme.rs        # shipped palette derivation mapped to gpui-kit
    │   ├── shell/          # sidebar, footer, error/toaster chrome, journal wiring
    │   └── panels/         # panel bodies (journal, voice) and icon mapping
    │       ├── journal/    # JournalView: state, navigation, reload, dates, stack, header, calendar, rows, error
    │       └── voice/      # VoiceCapture: stage machine and gpui entity
    └── tests/              # route, theme, layout, asset, journal and editor contracts
```

The React application is the visual ground truth. Route order, navigation
order, shortcut modifiers, palette formulas and locale keys are pinned by
tests against the shipped TypeScript implementation.

---

## Modules

### `stratum-i18n`

`stratum_i18n::Locale` enumerates the eight supported catalogs (`en`, `de`,
`el`, `es`, `fr`, `it`, `ja`, `zh`). The catalogs are embedded with
`include_str!`, flattened into dotted lookup keys and cached process-wide.

| API | Purpose |
|-----|---------|
| `t(locale, key)` | Resolve a key, returning the key itself when missing |
| `tr(locale, key, vars)` | Resolve and i18next-compatible `{{name}}` interpolate |
| `set_locale` / `current_locale` | Process-wide active locale |
| `t_global` / `tr_global` | Resolve in the active locale |
| `key_set` | Flattened user-facing keys (`_meta` excluded) |
| `missing_keys` / `extra_keys` | Parity drift helpers against English |

A missing key deliberately resolves to the raw key so an unfinished catalog is
visible in the UI rather than silently falling back to English.

### `stratum-ui`

`stratum_ui::Shell` is the root view. It owns the active route, read-mode and
sidebar state, and renders the shell chrome. The shell does not own business
logic: data comes from an `AppProvider` implementation.

| Module | Responsibility |
|--------|----------------|
| `router` | `Panel`, `ROUTES`, `NAV_ITEMS`, `SHORTCUTS`, `parse_path`, `path_for`, percent codec |
| `theme` | `ThemeConfig`, `derive_palette`, `apply_to_kit`, golden palette formulas |
| `assets` | `StratumAssets` (icons + fonts), `register_fonts` |
| `provider` | `ShellData`, `ShellSnapshot`, `FixtureProvider`, `provider_from_env`, asset seam (`provider/assets.rs`), journal seam (`provider/journal.rs`) and voice seam (`provider/voice.rs`) |
| `shell` | header, sidebar, footer, error bar, toaster, journal view wiring and start-state setters |
| `panels` | panel body rendering and icon mapping; `panels/journal` (entity, state, navigation, reload, calendar/stack/read-mode surface) and `panels/voice` (capture stage machine and entity) |

The provider seam is the only place the shell touches the service layer. The
real provider drives `stratum_app::StratumService`; `FixtureProvider` returns a
deterministic 87-page snapshot for tests and the capture harness.

`ShellData` also carries the attachment seam used by the native editor
(`read_asset_thumbnail`, `page_attachments`, `import_asset_bytes`,
`import_asset_path`, `delete_asset`). The trait defaults keep providers without
asset support compiling — no thumbnail, an empty list, a pass-through row and a
no-op delete. `AppProvider` maps the seam onto `stratum_app::asset`
(`asset_thumbnail`, `import_asset_bytes_checked`, `import_asset_path_checked`,
`get_page_asset_refs`, `delete_asset`); `FixtureProvider` keeps an in-memory
asset map with deterministic `assets/<stem>-<12 hex>.<ext>` names (v5 UUID over
the bytes) and lists the rows whose references appear in the saved page content,
so the drop/paste flow is testable without a vault.

The journal seam mirrors the shipped `JournalPanel` data flow. `journal_dates`
returns the dates that already have journal pages (newest first),
`ensure_today_journal` creates today's page if it is missing, and
`create_journal_day` creates a page for an arbitrary date. `AppProvider` maps
these onto `StratumService` page/block APIs and parses the `journals/<date>.md`
paths into sorted dates. `FixtureProvider` ships a fixed today (`2026-11-10`),
three past days (`2026-11-09`, `2026-11-08`, `2026-11-07`) and a recording at
`assets/voice/fixture.flac`; transcription inserts two deterministic blocks so
the capture harness can exercise the full voice flow without a microphone.

The voice seam (`stt_configured`, `voice_start`, `voice_stop`,
`voice_transcribe`, `voice_insert_clip`) has neutral trait defaults so providers
without audio support still compile. The trait defaults report STT as
unconfigured, fail `voice_start`, return an empty take and skip insertion.

---

## Building and Running

On Linux the shell links against xkbcommon and X11, so the development host
needs `libxkbcommon-dev`, `libxkbcommon-x11-dev`, `libx11-dev` and `libxcb1-dev`
(the CI `install-linux-deps` action installs these).

```bash
# Build the shell
cargo build -p stratum-ui

# Run against the fixture (no vault required)
STRATUM_FIXTURE=1 cargo run -p stratum-ui

# Run against a real vault
STRATUM_VAULT=/path/to/vault cargo run -p stratum-ui

# Select a UI catalog
STRATUM_LOCALE=de STRATUM_FIXTURE=1 cargo run -p stratum-ui
```

The binary opens a 1280×720 window. Shortcuts are installed from the shared
`SHORTCUTS` table: `Ctrl/Cmd+Shift+R` toggles read mode, `Ctrl/Cmd+B` toggles
the sidebar and `Escape` dismisses the topmost overlay.

### Environment variables

| Variable | Effect |
|----------|--------|
| `STRATUM_FIXTURE=1` | Use the deterministic fixture provider |
| `STRATUM_VAULT=/path` | Open the real service provider for that vault |
| `STRATUM_LOCALE=<code>` | Select one of the eight catalogs |
| `STRATUM_COLLAPSED=1` | Start with the 52 px icon rail (capture harness) |
| `STRATUM_ROUTE=/journal` | Open a route through the shared router at start-up |
| `STRATUM_PAGE=pages/Welcome.md` | Open a page in the editor at start-up |
| `STRATUM_JOURNAL_DATE=2026-11-09` | Start the journal on a specific day |
| `STRATUM_CALENDAR=1` | Start with the journal calendar expanded |
| `STRATUM_READ_MODE=1` | Start in read mode |

The start-state flags are applied by `apply_start_state` in `main.rs` and exist
for the capture harness; they do not change the interactive behaviour of the
shell.

---

## Testing

The GPUI crates are tested without opening a window:

```bash
cargo test -p stratum-i18n -p stratum-ui
cargo clippy -p stratum-i18n -p stratum-ui --all-targets -- -D warnings
```

The suites cover:

- Catalog parity, interpolation, missing-key behavior and locale round-trips.
- Route/navigation/shortcut tables against the shipped React order.
- Theme derivation goldens generated from the shipped TypeScript formulas.
- Shell geometry, navigation state, export success/failure, toaster and error
  bar behavior.
- Embedded icon/font availability and the `SHELL_I18N_KEYS` contract in all
  eight locales.
- The journal surface (`tests/journal_contract.rs`): localized date labels,
  calendar grid generation, the seven-slot week layout, the lazy stack plan and
  window growth, the voice stage machine, the fixture journal/voice data and the
  `JOURNAL_I18N_KEYS` release gate in all eight catalogs.

`stratum_ui` unit tests use the built-in `#[test]` attribute. `gpui_kit::*`
re-exports GPUI's own `test` attribute macro, so test modules that glob-import
the kit import the built-in attribute explicitly to avoid creating a GPUI app
context.

---

## Screenshot Capture

`scripts/capture-gpui-shell.sh` starts the shell in fixture mode on a private
Xvfb screen, waits for the window, and captures it with ImageMagick. It captures
both sidebar states:

```bash
./scripts/capture-gpui-shell.sh
# → dist/gpui-shell.png            (expanded, 224 px sidebar)
# → dist/gpui-shell-collapsed.png  (52 px icon rail)
```

The expanded capture is the parity reference for the shipped shell; the
collapsed capture pins the second geometry from §3. Measure them before
accepting a change: the sidebar/main boundary must sit at x=224 expanded and
x=52 collapsed (the divider pixel included), the expanded sidebar must show 12
navigation rows in the shipped order, and the collapsed rail must show the same
12 icons with no labels and no recents list.

The output path can be overridden with the first argument or `OUTPUT` (the
collapsed file derives from it). `STRATUM_COLLAPSED=1` is used for the rail
because Xvfb runs without a window manager, so synthetic key events are not
reliably delivered; the shortcut itself is covered by the `SHORTCUTS` contract
test. The script requires `Xvfb`, `xdotool` and ImageMagick's `import`. It is
used by the GPUI CI workflow to publish a screenshot artifact, and is safe to
run locally without a desktop session.

`scripts/capture-gpui-panels.sh` captures the journal surface in three states
using the start-state flags:

```bash
./scripts/capture-gpui-panels.sh
# → dist/gpui-journal.png           (edit mode)
# → dist/gpui-journal-calendar.png  (STRATUM_CALENDAR=1)
# → dist/gpui-journal-read.png      (past day, STRATUM_READ_MODE=1)
```

The edit-mode capture is the parity reference for the shipped `JournalPanel`:
the header shows the localized day label with previous/next-day and calendar
controls, the today section renders the editor, and the stacked past days follow
the same section order as the React panel. The calendar capture pins the popup
geometry and weekday headers; the read capture pins the read-mode treatment of
a past day. The first argument or `OUTPUT` overrides the edit-mode path and the
other two derive from it. Requirements are the same as the shell capture.

---

## CI

`.github/workflows/gpui-ci.yml` runs the GPUI checks on changes to the two
crates, the capture scripts or the CI wiring:

1. `gpui-shell` — formatting, build, tests and clippy on **Linux, Windows and
   macOS** (fail-fast disabled, one job per OS).
2. `gpui-capture` — Linux only: shell, journal and editor-perf captures, plus
   artifact upload.

Both jobs install the Tauri Linux dependencies through
`.github/actions/install-linux-deps`, which now also carries the native shell
headers (Wayland, xkbcommon, fontconfig, freetype, Vulkan/EGL/X11). They run
`npm ci` first because `stratum-app` embeds the Mermaid bundle from
`node_modules`; without it the GPUI build fails on a clean checkout.

The main `ci.yml` keeps the GPUI crates in `cargo test --workspace` (the
workspace gate must cover every member) and publishes the shell capture through
its `gpui-capture` job.

---

## Current Scope and Limitations

The native shell reproduces the application chrome and implements the editor
and journal surfaces. The journal view covers the shipped `JournalPanel`
behaviour: localized day labels, previous/next-day navigation, the month
calendar with journal/today markers, a lazily loaded stack of past days, read
mode and the voice-capture flow. Stacked day editors use the editor's
auto-height mode, so each section contributes its full content height to the
journal scroll container; journal editors omit the page-level attachment panel,
matching `JournalPanel`. Other panel bodies (graph, search, etc.) are the next
layer; the route/navigation/i18n contracts are already in place for them.

The toaster dismisses manually or through `Escape`; there is no timer-based
auto-dismiss yet. This is intentional for the E2 shell and is documented in
`crates/stratum-ui/src/shell/mod.rs`.

The shell is desktop-only. Mobile remains the React/Tauri application.

---

## Adding a Panel or Locale

1. Add the panel to `Panel::ALL`, `ROUTES` and (if it appears in the sidebar)
   `NAV_ITEMS` in `crates/stratum-ui/src/router.rs`.
2. Add its title key to `Panel::title_key` and its icon to the icon table in
   `crates/stratum-ui/src/panels/mod.rs`.
3. Add any new user-facing strings to `SHELL_I18N_KEYS` in
   `crates/stratum-ui/src/shell/mod.rs`; the locale-parity test will fail until
   every catalog has the key.
4. For a new locale, copy the shipped catalog to
   `crates/stratum-i18n/locales/`, extend `Locale`, `LOCALES` and
   `Locale::ALL`, and update the parity tests.
