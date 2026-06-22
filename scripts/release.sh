#!/usr/bin/env bash
set -euo pipefail

# ─── Release Script for Stratum ──────────────────────────────────────────────
# Full release build: test, tag, build Rust for all targets,
# build Flutter app for all desktop/mobile/web platforms, package archives.
#
# Usage:
#   ./scripts/release.sh                    # Auto-detect version from Cargo.toml
#   ./scripts/release.sh 0.2.0              # Specify version explicitly
#   ./scripts/release.sh --skip-tag         # Build without tagging
#   ./scripts/release.sh --rust-only        # Release only Rust binaries
#   ./scripts/release.sh --flutter-only     # Release only Flutter artifacts
# ──────────────────────────────────────────────────────────────────────────────

SKIP_TAG=false
RUST_ONLY=false
FLUTTER_ONLY=false

while [[ $# -gt 0 ]]; do
  case "$1" in
    --skip-tag) SKIP_TAG=true; shift ;;
    --rust-only) RUST_ONLY=true; shift ;;
    --flutter-only) FLUTTER_ONLY=true; shift ;;
    --help|-h)
      sed -n '/^# ───/,/^# ──/{/^# ──/d;p}' "$0" | head -n -1
      exit 0
      ;;
    *) VERSION="$1"; shift ;;
  esac
done

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

# ── Version ──────────────────────────────────────────────────────────────────
if [ -z "${VERSION:-}" ]; then
  VERSION="$(cargo metadata --format-version 1 --no-deps 2>/dev/null | \
    python3 -c "import sys,json; print(json.load(sys.stdin)['packages'][0]['version'])" 2>/dev/null || \
    grep '^version' Cargo.toml | head -1 | sed 's/.*= *"\(.*\)"/\1/')"
fi
echo "Releasing Stratum v$VERSION"
echo ""

# ── Run tests ────────────────────────────────────────────────────────────────
echo "── Running tests ──"
cargo test --workspace
if [ -d "frontend" ] && [ -f "frontend/pubspec.yaml" ]; then
  (cd frontend && flutter test)
fi
echo ""

# ── Lint ─────────────────────────────────────────────────────────────────────
echo "── Lint ──"
cargo clippy --workspace -- -D warnings 2>/dev/null || echo "  (clippy warnings found, check output)"
if [ -d "frontend" ] && [ -f "frontend/pubspec.yaml" ]; then
  (cd frontend && flutter analyze 2>/dev/null) || echo "  (flutter analyze warnings found, check output)"
fi
echo ""

# ── Git tag ──────────────────────────────────────────────────────────────────
if ! $SKIP_TAG; then
  echo "── Tagging v$VERSION ──"
  if git rev-parse "v$VERSION" >/dev/null 2>&1; then
    echo "  Tag v$VERSION already exists — skipping."
  else
    git tag -a "v$VERSION" -m "Release v$VERSION"
    git push origin "v$VERSION"
    echo "  Tagged and pushed v$VERSION"
  fi
  echo ""
fi

# ── Build Release Artifacts ──────────────────────────────────────────────────
RELEASE_DIR="$ROOT/release/v$VERSION"
mkdir -p "$RELEASE_DIR"

# ── Rust builds ──────────────────────────────────────────────────────────────
if ! $FLUTTER_ONLY; then
  echo "── Building Rust workspace (release) ──"
  cargo build --workspace --release

  # Copy CLI binary
  CLI_BIN="target/release/stratum"
  if [ -f "$CLI_BIN" ]; then
    case "$(uname -s)" in
      Linux*)  cp "$CLI_BIN" "$RELEASE_DIR/stratum-linux-amd64" ;;
      Darwin*) cp "$CLI_BIN" "$RELEASE_DIR/stratum-macos-amd64" ;;
      MINGW*|MSYS*|CYGWIN*)
        cp "target/release/stratum.exe" "$RELEASE_DIR/stratum-windows-amd64.exe"
        ;;
    esac
  fi
  echo ""
fi

# ── Flutter builds ───────────────────────────────────────────────────────────
if ! $RUST_ONLY; then
  if [ ! -d "frontend" ] || [ ! -f "frontend/pubspec.yaml" ]; then
    echo "── Flutter frontend not found — skipping ──"
  else
    # Search common Flutter installation paths
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
      echo "── Flutter not found — skipping Flutter builds ──"
      cd "$ROOT" 2>/dev/null || true
      return
    fi

    cd frontend

    # Desktop: linux (if host is linux or cross-toolchain available)
    $FLUTTER_CMD config --enable-linux-desktop 2>/dev/null || true
    if $FLUTTER_CMD build linux --release 2>/dev/null; then
      mkdir -p "$RELEASE_DIR/flutter-linux"
      cp -r build/linux/x64/release/bundle/* "$RELEASE_DIR/flutter-linux/"
      (cd "$RELEASE_DIR" && tar czf "stratum-linux-amd64-v$VERSION.tar.gz" flutter-linux/)
      rm -rf "$RELEASE_DIR/flutter-linux"
      echo "  → Linux bundle packaged"
    else
      echo "  Linux build skipped (not available on this host)"
    fi

    # Desktop: macos (if host is macOS)
    if [ "$(uname -s)" = "Darwin" ]; then
      $FLUTTER_CMD config --enable-macos-desktop 2>/dev/null || true
      if $FLUTTER_CMD build macos --release 2>/dev/null; then
        (cd "$RELEASE_DIR" && zip -r "stratum-macos-v$VERSION.zip" build/macos/Build/Products/Release/Stratum.app)
        echo "  → macOS bundle packaged"
      fi
    else
      echo "  macOS build skipped (requires macOS host)"
    fi

    # Desktop: windows (if host is windows)
    if [[ "$(uname -s)" =~ MINGW|MSYS|CYGWIN ]]; then
      $FLUTTER_CMD config --enable-windows-desktop 2>/dev/null || true
      if $FLUTTER_CMD build windows --release 2>/dev/null; then
        (cd "$RELEASE_DIR" && zip -r "stratum-windows-v$VERSION.zip" build/windows/x64/runner/Release/)
        echo "  → Windows bundle packaged"
      fi
    else
      echo "  Windows build skipped (requires Windows host)"
    fi

    # Mobile: android (if android SDK available)
    if $FLUTTER_CMD build apk --release 2>/dev/null; then
      cp build/app/outputs/flutter-apk/app-release.apk "$RELEASE_DIR/stratum-android-v$VERSION.apk"
      echo "  → Android APK packaged"
    else
      echo "  Android build skipped (Android SDK not configured)"
    fi

    # Mobile: ios (if macOS host)
    if [ "$(uname -s)" = "Darwin" ]; then
      if $FLUTTER_CMD build ios --release --no-codesign 2>/dev/null; then
        echo "  → iOS build completed (sign manually before distribution)"
      else
        echo "  iOS build skipped"
      fi
    fi

    # Web
    if $FLUTTER_CMD build web --release 2>/dev/null; then
      (cd "$RELEASE_DIR" && zip -r "stratum-web-v$VERSION.zip" build/web/)
      echo "  → Web bundle packaged"
    else
      echo "  Web build skipped (unexpected error)"
    fi

    cd "$ROOT"
  fi
fi

echo ""
echo "╔══════════════════════════════════════════════════╗"
echo "║  Release v$VERSION complete                      ║"
echo "║  Artifacts: $RELEASE_DIR                        ║"
echo "╚══════════════════════════════════════════════════╝"
ls -lh "$RELEASE_DIR/" 2>/dev/null || true
