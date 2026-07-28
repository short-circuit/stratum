# Stratum Real E2E Tests

Tests the **compiled Tauri app** via `tauri-driver` and the WebDriver protocol.
Real IPC, real SQLite, real filesystem — no mocks.

## Prerequisites

```bash
# 1. tauri-driver
cargo install tauri-driver

# 2. WebKitWebDriver (from webkit2gtk)
# Arch:     already included in webkit2gtk-4.1
# Ubuntu:   sudo apt install webkit2gtk-4.1-dev
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

## CI

The `e2e-real` CI job runs on `workflow_dispatch` only (manual trigger via GitHub UI).
It requires a display server (xvfb) and adds ~5 minutes to CI.

## Test Specs

| File | Tests | What it covers |
|------|-------|---------------|
| `test.mjs` | 2 | App boots, root element renders with content |

## Limitations

- Requires WebKitWebDriver binary (part of webkit2gtk system package)
- Only works on Linux with X11/Wayland
- Each platform (Linux/macOS/Windows) needs its own tauri-driver + WebDriver setup
