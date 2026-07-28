#!/bin/bash
# Real E2E test runner for Stratum PKM
# Builds the app, starts tauri-driver, runs Playwright tests
set -euo pipefail

export PATH="$HOME/.cargo/bin:$PATH"

STRATUM_DIR="$HOME/stratum"
APP_BINARY="$STRATUM_DIR/src-tauri/target/debug/stratum-tauri"
E2E_SPEC_DIR="$STRATUM_DIR/e2e/specs/real"

# Use absolute paths
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

cleanup() {
  echo "=== Cleanup ==="
  kill $TAURI_DRIVER_PID 2>/dev/null || true
  kill $APP_PID 2>/dev/null || true
  wait 2>/dev/null || true
}
trap cleanup EXIT INT TERM

echo "=== Stratum Real E2E Test Runner ==="
echo "App binary: $APP_BINARY"

# 1. Build the app if not already built
if [ ! -f "$APP_BINARY" ]; then
  echo "Building app..."
  cd "$STRATUM_DIR" && cargo build -p stratum-tauri
fi

# 2. Start the Tauri app in background
echo "Starting Tauri app..."
xvfb-run -a "$APP_BINARY" &
APP_PID=$!
echo "App PID: $APP_PID"
sleep 3

# 3. Start tauri-driver
echo "Starting tauri-driver..."
tauri-driver --port 4444 &
TAURI_DRIVER_PID=$!
echo "tauri-driver PID: $TAURI_DRIVER_PID"
sleep 2

# 4. Run the Playwright tests directly against the spec files
echo "Running Playwright tests..."
cd "$STRATUM_DIR"
npx playwright test "$E2E_SPEC_DIR" --config "$STRATUM_DIR/e2e/playwright.real.config.ts" "$@"
RESULT=$?

echo "=== Tests completed with exit code: $RESULT ==="
exit $RESULT
