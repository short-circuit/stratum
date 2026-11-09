# Native GPUI Shell

Stratum ships two front ends:

- The frozen React + Tauri application in `src/` and `src-tauri/`, which remains
  the reference implementation and is not modified by the native shell work.
- The native GPUI shell in `crates/stratum-ui`, which reproduces the React
  shell's chrome (sidebar, navigation, recents, footer, status bar, error bar
  and toaster) on top of the headless service layer.

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
    │   ├── main.rs         # binary entry point (1280×720 window)
    │   ├── assets.rs       # embedded asset source + font registration
    │   ├── provider.rs     # ShellData seam, fixture and service providers
    │   ├── router.rs       # routes, navigation, shortcuts, path codec
    │   ├── theme.rs        # shipped palette derivation mapped to gpui-kit
    │   ├── shell/          # sidebar, footer, status bar, error/toaster chrome
    │   └── panels/         # panel stubs and icon mapping
    └── tests/              # route, theme, layout and asset contracts
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
| `provider` | `ShellData`, `ShellSnapshot`, `FixtureProvider`, `provider_from_env` |
| `shell` | header, sidebar, footer, status bar, error bar, toaster |
| `panels` | panel stub rendering and icon mapping |

The provider seam is the only place the shell touches the service layer. The
real provider drives `stratum_app::StratumService`; `FixtureProvider` returns a
deterministic 87-page snapshot for tests and the capture harness.

---

## Building and Running

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

`stratum_ui` unit tests use the built-in `#[test]` attribute. `gpui_kit::*`
re-exports GPUI's own `test` attribute macro, so test modules that glob-import
the kit import the built-in attribute explicitly to avoid creating a GPUI app
context.

---

## Screenshot Capture

`scripts/capture-gpui-shell.sh` starts the shell in fixture mode on a private
Xvfb screen, waits for the window, and captures it with ImageMagick:

```bash
./scripts/capture-gpui-shell.sh
# → dist/gpui-shell.png
```

The output path can be overridden with the first argument or `OUTPUT`. The
script requires `Xvfb`, `xdotool` and ImageMagick's `import`. It is used by the
GPUI CI workflow to publish a screenshot artifact, and is safe to run locally
without a desktop session.

---

## CI

`.github/workflows/gpui-ci.yml` runs the GPUI checks on changes to the two
crates, the capture script or the CI wiring:

1. Formatting (`cargo fmt --all --check`).
2. Build (`cargo build -p stratum-i18n -p stratum-ui`).
3. Tests (`cargo test -p stratum-i18n -p stratum-ui`).
4. Clippy with `-D warnings`.
5. Screenshot capture and artifact upload.

The workflow installs the same Linux build dependencies as the Tauri jobs plus
the GPUI capture tools (`xvfb`, `xdotool`, `imagemagick`, fontconfig and
freetype headers). It runs `npm ci` first because `stratum-app` embeds the
Mermaid bundle from `node_modules`.

---

## Current Scope and Limitations

The native shell currently reproduces the application chrome and renders panel
stubs. Panel bodies (editor, graph, search, etc.) are the next layer; the
route/navigation/i18n contracts are already in place for them.

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
