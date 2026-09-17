# Stratum Automated E2E Harness (tauri-driver)

Drives the **compiled Tauri app** through the WebDriver protocol using
`tauri-driver` + `WebKitWebDriver`. Real IPC, real SQLite, real filesystem, real
UI rendering — no mocks.

```
e2e/harness/
├── bin/run.mjs        # one-command runner: starts Xvfb + tauri-driver, runs the suite, tears down
├── lib/driver.js      # WebDriver session management (create/attach/wait-for-render/teardown)
├── tests/             # (add more test modules here; runTests() in bin/run.mjs is the entry point)
└── README.md          # this file
```

## Why a new harness?

The legacy `e2e/real/` path was written against the deprecated `webdriver.remote()`
API (webdriver v8) and no longer works with the installed `webdriver` v9 package
(`remote is not a function`). It also never provided a working app binary path:
plain `cargo build` produces a **dev-mode** binary that loads `devUrl`
(`http://localhost:5173`) instead of embedded assets, so the old tests were
silently asserting against a "connection refused" error page.

## Prerequisites

- `tauri-driver` on `$PATH` (`cargo install tauri-driver`)
- `WebKitWebDriver` — installed as part of `webkit2gtk-4.1` on Arch (under
  `/usr/lib/webkit2gtk-4.1/`); on Nix it lives in the webkitgtk store path. The
  harness auto-detects known locations and also honors `HARNESS_DRIVER`. **Note:
  if yours is missing, set `HARNESS_DRIVER=/path/to/WebKitWebDriver`.**
- `Xvfb` (for headless display)
- The app binary built in **production mode**:
  ```
  cargo build -p stratum-tauri    # requires the `custom-protocol` feature in
  ```                             # src-tauri/Cargo.toml — see below

## The custom-protocol requirement

Tauri v2 decides dev vs production from the `custom-protocol` Cargo feature:

```rust
// src-tauri/Cargo.toml — this is REQUIRED for embedded-asset E2E
tauri = { version = "2", features = ["custom-protocol"] }
```

Without it a plain `cargo build` sets `dev = !custom_protocol` → the app loads
`devUrl` (`http://localhost:5173`) instead of embedded `dist/` assets. Real E2E
against a compiled binary needs the embedded-asset build, so the harness
requires this feature. (The dev workflow is unaffected: `npm run tauri:dev`
still uses the vite server.)

## Running

```bash
# From the repo root (app must already be built in production mode)
node e2e/harness/bin/run.mjs

# Force a fresh production build first
cargo build -p stratum-tauri
node e2e/harness/bin/run.mjs
```

### Environment variables

| Variable            | Purpose                                                | Default                               |
|---------------------|--------------------------------------------------------|---------------------------------------|
| `HARNESS_APP`       | path to the compiled app binary                       | `<repo>/target/debug/stratum-tauri`   |
| `HARNESS_DRIVER`    | path to `WebKitWebDriver`                             | auto-detect                           |
| `HARNESS_DISPLAY`   | X display number                                      | `99`                                  |
| `HARNESS_XVFB`      | set `0` to reuse an already-running X server          | `1`                                   |
| `HARNESS_NO_CLEAN`  | set `1` to keep processes alive after the run        | unset                                 |
| `HARNESS_TDRIVER`   | alternate `tauri-driver` binary / path                | `tauri-driver`                        |

## What it tests

1. **App boots and renders** — confirms the window is on `tauri://localhost`
   and the title is `Stratum`.
2. **Real UI content** — finds rendered anchor (wiki-link) elements and reads
   their text.
3. **IPC round-trip / app state** — `executeScript` probes the live DOM for
   `#root`, rendered text, and the `tauri://` scheme (proves the real frontend
   mounted against the real Rust backend, not an error page).
4. **Screenshot capture** — writes `/tmp/stratum-harness-last.png`.

Exit code is non-zero on any failure so it can gate CI.

## Adding tests

Extend `runTests(driver)` in `bin/run.mjs`, or import `createSession` /
`waitForApp` from `lib/driver.js` and write focused scripts. Standard WebDriver
commands (find/click/type/getText/screenshot) are all available on the driver
object from `webdriver` v9.

## CI wiring

See the `e2e` CI job (t_20d4e32c) for wiring this into GitHub Actions.
