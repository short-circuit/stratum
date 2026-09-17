# Stratum Automated E2E Harness (tauri-driver)

Drives the **compiled Tauri app** through the WebDriver protocol using
`tauri-driver` + `WebKitWebDriver`. Real IPC, real SQLite, real filesystem, real
UI rendering — no mocks.

```
e2e/harness/
├── bin/run.mjs        # one-command runner: starts Xvfb + tauri-driver, runs the suite, tears down
├── lib/driver.js      # WebDriver session management (create/attach/wait-for-render/teardown)
├── lib/results.js     # machine-readable results reporting (JSON manifest + JUnit XML)
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
npm run test:e2e:harness            # same as: node e2e/harness/bin/run.mjs

# Force a fresh production build first
cargo build -p stratum-tauri
npm run test:e2e:harness
```

The runner is deliberately self-contained: it validates prerequisites, starts
Xvfb + tauri-driver, runs the suite, writes results, and tears everything
down — so the single command above is the full flow (no manual driver/Xvfb
setup).

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

## Results (machine-readable)

Every run writes machine-readable results into the repo's **gitignored**
`test-results/` directory:

| File                              | Format     | Purpose                               |
|-----------------------------------|------------|---------------------------------------|
| `test-results/e2e-harness-manifest.json` | JSON   | **Authoritative** report: per-test outcome (`pass`/`fail`), counts, environment, timing |
| `test-results/e2e-harness-junit.xml`     | JUnit XML | Convenience for CI tooling that only speaks JUnit (the JSON is definitive) |

```json
// test-results/e2e-harness-manifest.json (abridged)
{
  "schema": "stratum-e2e-harness/manifest/v1",
  "app": "stratum-tauri",
  "suite": "stratum automated e2e (tauri-driver)",
  "status": "pass",
  "counts": { "passed": 8, "failed": 0, "skipped": 0, "total": 8 },
  "tests": [
    { "suite": "harness", "name": "App window is on tauri://localhost", "status": "pass", "detail": "", "durationMs": 0 }
  ]
}
```

The runner's exit code already reflects pass/fail, so the manifest is
additional structured evidence — CI failure artifact upload follows the same
`test-results/` directory. The JSON `status` field is derived from the same
outcomes that drive the exit code; the JUnit XML is regenerated from the same
in-memory results and is never the source of truth.

## Adding tests

Extend `runTests(driver)` in `bin/run.mjs`, or import `createSession` /
`waitForApp` from `lib/driver.js` and write focused scripts. Standard WebDriver
commands (find/click/type/getText/screenshot) are all available on the driver
object from `webdriver` v9.

Tests MUST record their outcome through the `ok(...)` / `bad(...)` helpers in
`bin/run.mjs` (or call `recordTest()` directly) — the helpers both print to
stdout **and** feed `lib/results.js`, so every assertion appears in the
machine-readable manifest. A test that does not call `ok`/`bad` is invisible
to the report and to CI.

## CI wiring

The harness is packaged so an `e2e-real` CI job can invoke it with a single
command. The job itself is wired into `.github/workflows/ci.yml` by the
separate "Add E2E test job to CI" task (t_20d4e32c); the harness side makes
that job a thin wrapper:

```bash
# From the repo root in CI (Ubuntu runner):
npm ci                                  # install deps (webdriver, tauri toolchain)
npm run build                           # build frontend -> dist/
cargo build -p stratum-tauri            # prod-mode binary with embedded assets
cargo install tauri-driver              # WebDriver intermediary
sudo apt-get install -y xvfb            # headless display (install-linux-deps already installs webkit2gtk-4.1)
xvfb-run -a node e2e/harness/bin/run.mjs
```

What the runner does in CI, in order:

1. Validates prerequisites (`tauri-driver`, `WebKitWebDriver`, built app) and
   exits `2` with a clear message if any are missing — a misconfigured job
   fails fast instead of silently passing.
2. Starts `Xvfb` on an unused display, then `tauri-driver` (which spawns
   `WebKitWebDriver` on its native port).
3. Creates a WebDriver session against the **compiled app binary**
   (`target/debug/stratum-tauri`, production/embed mode) and runs the suite.
4. Writes machine-readable results to `test-results/e2e-harness-manifest.json`
   (JSON) and `test-results/e2e-harness-junit.xml` (JUnit XML) — both in the
   repo's gitignored `test-results/` directory.
5. Exits non-zero on any failure.

CI should upload `test-results/**` (and optionally
`/tmp/stratum-harness-last.png`) as an artifact **on failure** so a broken run
is actionable without a manual reproduction. `WebKitWebDriver` is provided by
the `webkit2gtk-4.1` package that install-linux-deps already installs; on
runners where it lands elsewhere, set `HARNESS_DRIVER` to its absolute path.
