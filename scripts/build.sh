#!/usr/bin/env bash
set -euo pipefail

# ─── Build Script for Stratum ───────────────────────────────────────────────
# Builds Rust crates and Flutter frontend for development.
#
# Usage:
#   ./scripts/build.sh                        # Debug build for host platform
#   ./scripts/build.sh --release              # Release build
#   ./scripts/build.sh --platform linux       # Build Flutter for specific target
#   ./scripts/build.sh --rust-only            # Build only Rust crates
#   ./scripts/build.sh --flutter-only         # Build only Flutter frontend
#   ./scripts/build.sh --release --platform macos
#
# Supported Flutter platforms:
#   linux, macos, windows, android, ios, web
# ──────────────────────────────────────────────────────────────────────────────

RELEASE=""
PLATFORM=""
RUST_ONLY=false
FLUTTER_ONLY=false

while [[ $# -gt 0 ]]; do
  case "$1" in
    --release) RELEASE="--release"; shift ;;
    --platform) PLATFORM="$2"; shift 2 ;;
    --rust-only) RUST_ONLY=true; shift ;;
    --flutter-only) FLUTTER_ONLY=true; shift ;;
    --help|-h)
      sed -n '/^# ───/,/^# ──/{/^# ──/d;p}' "$0" | head -n -1
      exit 0
      ;;
    *) echo "Unknown option: $1"; exit 1 ;;
  esac
done

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

# ── Detect host platform ─────────────────────────────────────────────────────
detect_platform() {
  case "$(uname -s)" in
    Linux*)  echo "linux" ;;
    Darwin*) echo "macos" ;;
    MINGW*|MSYS*|CYGWIN*) echo "windows" ;;
    *)       echo "unknown" ;;
  esac
}

HOST_PLATFORM="$(detect_platform)"
TARGET="${PLATFORM:-$HOST_PLATFORM}"

# Normalize known platform names
case "$TARGET" in
  linux|Linux)     TARGET="linux" ;;
  macos|macOS|darwin|Darwin) TARGET="macos" ;;
  windows|Windows|win) TARGET="windows" ;;
  android|Android) TARGET="android" ;;
  ios|iOS)         TARGET="ios" ;;
  web|Web)         TARGET="web" ;;
  *)               echo "Unsupported target platform: $TARGET"; exit 1 ;;
esac

echo "╔══════════════════════════════════════════════════╗"
echo "║  Stratum Build                                   ║"
echo "║  Mode:     ${RELEASE:-debug}                      ║"
echo "║  Host:     $HOST_PLATFORM                         ║"
echo "║  Flutter:  $TARGET                                ║"
echo "╚══════════════════════════════════════════════════╝"
echo ""

# ── Rust build ───────────────────────────────────────────────────────────────
if ! $FLUTTER_ONLY; then
  echo "── Building Rust workspace ──"
  cargo build --workspace $RELEASE
  echo ""
fi

# ── Flutter build ────────────────────────────────────────────────────────────
if ! $RUST_ONLY; then
  if [ ! -d "frontend" ] || [ ! -f "frontend/pubspec.yaml" ]; then
    echo "── Flutter frontend not found at frontend/ ──"
    echo "   Skipping Flutter build."
    echo ""
    exit 0
  fi

  # Search common installation paths if not on PATH
  FLUTTER_CMD="flutter"
  if ! command -v "$FLUTTER_CMD" &>/dev/null; then
    for dir in "$HOME/development/flutter" "$HOME/flutter" "/opt/flutter" "/usr/local/flutter"; do
      if [ -x "$dir/bin/flutter" ]; then
        FLUTTER_CMD="$dir/bin/flutter"
        break
      fi
    done
  fi

  if ! command -v "$FLUTTER_CMD" &>/dev/null; then
    echo "── WARNING: 'flutter' not found on PATH. Skipping Flutter build. ──"
    echo "   Install Flutter SDK: https://docs.flutter.dev/get-started/install"
    echo ""
    cd "$ROOT"
    exit 0
  fi

  cd frontend
  $FLUTTER_CMD pub get 2>/dev/null || echo "  (flutter pub get skipped — continuing)"

  case "$TARGET" in
    linux)
      $FLUTTER_CMD config --enable-linux-desktop 2>/dev/null || true
      $FLUTTER_CMD build linux $RELEASE || echo "  (linux desktop build failed — GTK dev libs may be missing)"
      ;;
    macos)
      $FLUTTER_CMD config --enable-macos-desktop 2>/dev/null || true
      $FLUTTER_CMD build macos $RELEASE || echo "  (macOS build failed — requires macOS host)"
      ;;
    windows)
      $FLUTTER_CMD config --enable-windows-desktop 2>/dev/null || true
      $FLUTTER_CMD build windows $RELEASE || echo "  (Windows build failed — requires Windows host)"
      ;;
    android)
      $FLUTTER_CMD build apk $RELEASE || echo "  (Android build failed — Android SDK may be missing)"
      ;;
    ios)
      $FLUTTER_CMD build ios $RELEASE --no-codesign || echo "  (iOS build failed — requires macOS + Xcode)"
      ;;
    web)
      $FLUTTER_CMD build web $RELEASE || echo "  (Web build failed)"
      ;;
  esac

  cd "$ROOT"
  echo ""
fi

echo "── Build complete ──"
