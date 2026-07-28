#!/bin/bash
# Real E2E test runner for Stratum PKM
# Builds the app, starts tauri-driver, runs WebDriver tests
# Works both locally and in CI (uses GITHUB_WORKSPACE or script location)
set -euo pipefail

export PATH="$HOME/.cargo/bin:$PATH"

# Determine workspace root (works in CI and locally)
if [ -n "${GITHUB_WORKSPACE:-}" ]; then
  STRATUM_DIR="$GITHUB_WORKSPACE"
else
  SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
  STRATUM_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
fi

APP_BINARY="$STRATUM_DIR/target/debug/stratum-tauri"
TEST_SCRIPT="$STRATUM_DIR/e2e/real/test.mjs"

# Find WebKitWebDriver (Ubuntu: /usr/lib/*/webkit2gtk-*/WebKitWebDriver, Arch: /usr/lib/webkit2gtk-4.1/WebKitWebDriver)
WEBKIT_DRIVER=$(find /usr/lib -name "WebKitWebDriver" -type f 2>/dev/null | head -1 || true)
if [ -z "$WEBKIT_DRIVER" ]; then
  echo "❌ WebKitWebDriver not found. Install webkit2gtk-4.1-dev (Ubuntu) or webkit2gtk (Arch)."
  echo "   Ubuntu: sudo apt install libwebkit2gtk-4.1-dev"
  echo "   Arch:   sudo pacman -S webkit2gtk-4.1"
  exit 1
fi
echo "✅ WebKitWebDriver: $WEBKIT_DRIVER"

cleanup() {
  echo "=== Cleanup ==="
  kill $TAURI_DRIVER_PID 2>/dev/null || true
  kill $APP_PID 2>/dev/null || true
  wait 2>/dev/null || true
}
trap cleanup EXIT INT TERM

echo "=== Stratum Real E2E Test Runner ==="
echo "Workspace: $STRATUM_DIR"
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

# Start tauri-driver with WebKitWebDriver path
echo "Starting tauri-driver on port 4444..."
tauri-driver --port 4444 --native-driver "$WEBKIT_DRIVER" &
TAURI_DRIVER_PID=$!
sleep 2

# Run tests
echo "Running tests..."
cd "$STRATUM_DIR"
node "$TEST_SCRIPT"
RESULT=$?

echo "=== Exit code: $RESULT ==="
exit $RESULT
