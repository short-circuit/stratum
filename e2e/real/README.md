# Stratum Real E2E Tests

These tests run against a **compiled Tauri app** using `tauri-driver` + WebDriverIO.
Unlike the mock E2E tests (`npm run test:e2e`), these verify real IPC, real SQLite,
and real filesystem operations.

## Prerequisites

```bash
# Install tauri-driver
cargo install tauri-driver

# Install WebDriverIO CLI
npm install --save-dev @wdio/cli @wdio/local-runner @wdio/mocha-framework
npx wdio config
```

## Running

```bash
# From project root:
bash e2e/real/run.sh
```

Or step by step:

```bash
# Terminal 1: Start the app (requires display)
xvfb-run -a ./src-tauri/target/debug/stratum-tauri &

# Terminal 2: Start tauri-driver
tauri-driver --port 4444 &

# Terminal 3: Run tests
npx wdio run e2e/wdio.conf.ts
```

## CI

The `e2e-real` job in `.github/workflows/ci.yml` runs on `workflow_dispatch` only
(manual trigger via GitHub UI) because it requires a display server and adds
~5 minutes to CI time.

## How It Works

1. `tauri-driver` is a WebDriver-compatible server that bridges Playwright/WebDriverIO
   with the Tauri WebView
2. The Tauri app registers itself with tauri-driver on launch
3. Test commands (`wdio`) send WebDriver commands through tauri-driver
4. The WebView renders the React frontend, and IPC calls go to real Rust code

## Test Specs

| File | Tests | What it covers |
|------|-------|---------------|
| `app.spec.ts` | 2 | App boots, renders vault picker or main UI |

## Limitations

- Requires a display server (X11/Wayland). Use `xvfb-run` for headless CI.
- WebDriverIO is a separate test framework from Playwright (used for mock E2E).
- Each platform (Linux/macOS/Windows) needs its own tauri-driver build.
