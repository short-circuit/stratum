# Stratum Real E2E Tests (legacy path)

> **Status: superseded.** The maintained real-tauri E2E runner is
> [`e2e/harness/`](../harness/README.md) — a single command
> (`npm run test:e2e:harness`) that builds, drives the compiled app through
> `tauri-driver` + `WebKitWebDriver`, writes machine-readable results, and is
> wired into CI (run on every push/PR, on a **weekly schedule**, and on manual
> `workflow_dispatch`). Use that; this directory is kept for reference.
>
> Known issue: `npm run test:e2e:real` (`e2e/real/test.mjs`) is **broken** with
> the current `webdriver` v9 dependency — it imports `remote()` from
> `webdriver`, an API removed in v9 (`remote is not a function`). The harness
> replaced it with the v9 `WebDriver.newSession()` API. Fix or remove this
> script if you ever need the legacy path again.

Tests the **compiled Tauri app** via `tauri-driver` and the WebDriver protocol.
Real IPC, real SQLite, real filesystem — no mocks.

## Prerequisites

```bash
# 1. tauri-driver
cargo install tauri-driver

# 2. WebKitWebDriver (from webkit2gtk)
# Arch:     sudo pacman -S webkit2gtk        # ships in webkit2gtk-4.1
#           (binary: /usr/lib/webkit2gtk-4.1/WebKitWebDriver, or via webkit2gtk-4.1)
# Ubuntu:   sudo apt install webkit2gtk-4.1-dev webkit2gtk-driver
# Fedora:   sudo dnf install webkit2gtk4.1-devel

# 3. xvfb (for headless)
# Arch:     sudo pacman -S xorg-server-xvfb
# Ubuntu:   sudo apt install xvfb
```

## Running

```bash
# One command — builds app, starts xvfb, tauri-driver, runs tests:
bash e2e/real/run.sh

# Or step-by-step:
cd ~/stratum
cargo build -p stratum-tauri
xvfb-run -a ./target/debug/stratum-tauri &
tauri-driver --port 4444 &
node e2e/real/test.mjs
```

> Note: `e2e/real/run.sh` + `test.mjs` are the legacy runner. Prefer the
> maintained harness:  `npm run test:e2e:harness` (see
> [`e2e/harness/README.md`](../harness/README.md)).

## CI

The old `e2e-real` job was replaced by the `e2e-harness` job in
`.github/workflows/ci.yml`. The maintained real E2E runs:

- on every push to `master` and every PR to `master` (required check),
- on a **weekly schedule** (drift check against the latest `master`; see the
  `schedule` trigger in `ci.yml`),
- manually via `workflow_dispatch`.

It requires a display server (xvfb) and adds several minutes to CI.

## Test Specs

| File | Tests | What it covers |
|------|-------|---------------|
| `test.mjs` | 2 | App boots, root element renders with content |

## Limitations

- Requires WebKitWebDriver binary (part of webkit2gtk system package)
- Only works on Linux with X11/Wayland
- Each platform (Linux/macOS/Windows) needs its own tauri-driver + WebDriver setup

For the maintained harness's limitations and setup, see
[`e2e/harness/README.md`](../harness/README.md).
