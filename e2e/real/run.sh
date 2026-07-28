#!/bin/bash
# Real E2E test runner for Stratum PKM
# Builds the app, starts tauri-driver, runs WebDriver tests
set -euo pipefail

export PATH="$HOME/.cargo/bin:$PATH"

STRATUM_DIR="$HOME/stratum"
APP_BINARY="$STRATUM_DIR/src-tauri/target/debug/stratum-tauri"
TEST_SCRIPT="$STRATUM_DIR/e2e/real/test.mjs"

cleanup() {
  echo "=== Cleanup ==="
  kill $TAURI_DRIVER_PID 2>/dev/null || true
  kill $APP_PID 2>/dev/null || true
  wait 2>/dev/null || true
}
trap cleanup EXIT INT TERM

echo "=== Stratum Real E2E Test Runner ==="
echo "App: $APP_BINARY"
echo "Test: $TEST_SCRIPT"

# Build if needed
if [ ! -f "$APP_BINARY" ]; then
  echo "Building app..."
  cd "$STRATUM_DIR" && cargo build -p stratum-tauri
fi

# Start app with virtual display
echo "Starting Tauri app (xvfb)..."
xvfb-run -a "$APP_BINARY" &
APP_PID=$!
sleep 3

# Start tauri-driver
echo "Starting tauri-driver..."
tauri-driver --port 4444 &
TAURI_DRIVER_PID=$!
sleep 2

# Run tests
echo "Running tests..."
cd "$STRATUM_DIR"
node "$TEST_SCRIPT"
RESULT=$?

echo "=== Exit code: $RESULT ==="
exit $RESULT
