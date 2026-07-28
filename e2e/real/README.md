# Stratum PKM — Real E2E Testing with tauri-driver

## Prerequisites

```bash
# Install tauri-driver
cargo install tauri-driver

# Build the app in debug mode
cargo build -p stratum-tauri
```

## Running

```bash
# Terminal 1: Start the Tauri app with WebDriver support
cargo run -p stratum-tauri &

# Terminal 2: Start tauri-driver
tauri-driver

# Terminal 3: Run the tests
npx playwright test --config e2e/playwright.real.config.ts
```

Or with a single command:
```bash
npm run test:e2e:real
```

## What These Tests Cover

These tests interact with a **real compiled Tauri app** — real SQLite, real Tantivy, real IPC.
Unlike the mock-based E2E tests in `e2e/specs/`, these verify:
- IPC serialization/deserialization works correctly
- Tauri command handlers execute without errors
- The full Rust → TypeScript → Rust round-trip is correct
- Real database operations persist as expected

## CI Integration

The `e2e-real` CI job builds the app, starts tauri-driver with xvfb-run (headless), and runs the tests.
This requires `libwebkit2gtk-4.1-dev` and `xvfb` to be installed.
