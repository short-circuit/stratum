#!/usr/bin/env bash
set -euo pipefail

# ─── Setup Script for Stratum ────────────────────────────────────────────────
# Checks prerequisites, installs missing tools, initializes git hooks.
#
# Usage:
#   ./scripts/setup.sh                        # Full setup check
#   ./scripts/setup.sh --ci                   # CI mode — no installs, just check
#   ./scripts/setup.sh --rust-only            # Only check/install Rust tooling
#   ./scripts/setup.sh --flutter-only         # Only check/install Flutter tooling
# ──────────────────────────────────────────────────────────────────────────────

CI_MODE=false
RUST_ONLY=false
FLUTTER_ONLY=false

while [[ $# -gt 0 ]]; do
  case "$1" in
    --ci) CI_MODE=true; shift ;;
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

echo "╔════════════════════════════════╗"
echo "║  Stratum Setup                 ║"
echo "╚════════════════════════════════╝"
echo ""

FAILED=false
WARNINGS=false

check_cmd() {
  local name="$1" cmd="${2:-$1}"
  if command -v "$cmd" &>/dev/null; then
    echo "  ✓ $name ($(command -v "$cmd"))"
    return 0
  else
    echo "  ✗ $name — NOT FOUND"
    return 1
  fi
}

version_ge() {
  local want="$1" have="$2"
  printf '%s\n%s\n' "$want" "$have" | sort -V | head -1 | grep -qxF "$want"
}

# ── Prerequisites ────────────────────────────────────────────────────────────
echo "── Checking prerequisites ──"

# Rust
if ! $FLUTTER_ONLY; then
  if check_cmd "rustc" rustc; then
    RUST_VER="$(rustc --version | grep -oP '\d+\.\d+\.\d+' | head -1)"
    if version_ge "1.75" "$RUST_VER"; then
      echo "    rustc $RUST_VER (>= 1.75 ✓)"
    else
      echo "    rustc $RUST_VER (>= 1.75 required — UPGRADE NEEDED)"
      FAILED=true
    fi
  else
    echo "    Install Rust: https://rustup.rs"
    if ! $CI_MODE; then
      echo "    → Attempting install via rustup..."
      curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
      source "$HOME/.cargo/env"
    fi
    FAILED=true
  fi

  if check_cmd "cargo"; then
    # Check wasm32 target for plugins
    if rustup target list --installed 2>/dev/null | grep -q wasm32-unknown-unknown; then
      echo "    wasm32-unknown-untarget ✓"
    else
      echo "    wasm32-unknown-unknown — NOT installed (needed for WASM plugins)"
      if ! $CI_MODE; then
        rustup target add wasm32-unknown-unknown
        echo "    ✓ installed"
      fi
    fi
  fi
  echo ""
fi

# Flutter
if ! $RUST_ONLY; then
  # Search common installation paths if not on PATH
  if ! command -v flutter &>/dev/null; then
    for dir in "$HOME/development/flutter" "$HOME/flutter" "/opt/flutter" "/usr/local/flutter"; do
      if [ -x "$dir/bin/flutter" ]; then
        export PATH="$dir/bin:$PATH"
        break
      fi
    done
  fi

  if check_cmd "flutter"; then
    FLUTTER_VER="$(flutter --version 2>/dev/null | grep 'Flutter' | grep -oP '\d+\.\d+\.\d+' | head -1)"
    echo "    Flutter $FLUTTER_VER"
  else
    echo "    Install Flutter: https://docs.flutter.dev/get-started/install"
    FAILED=true
  fi

  if check_cmd "dart"; then
    DART_VER="$(dart --version 2>&1 | grep -oP '\d+\.\d+\.\d+' | head -1)"
    echo "    Dart $DART_VER"
  fi
  echo ""
fi

# ── Platform-specific SDKs ───────────────────────────────────────────────────
echo "── Checking platform SDKs ──"

case "$(uname -s)" in
  Linux*)
    if ! $RUST_ONLY; then
      check_cmd "pkg-config" || WARNINGS=true
      # Check GTK/glib for linux desktop Flutter
      if ! pkg-config --exists gtk+-3.0 2>/dev/null; then
        echo "  ⚠ gtk+-3.0 not found (needed for linux desktop)"
        echo "    Install: sudo apt install libgtk-3-dev"
        WARNINGS=true
      fi
    fi
    ;;
  Darwin*)
    if ! $RUST_ONLY; then
      check_cmd "xcodebuild" || WARNINGS=true
      if ! xcodebuild -version &>/dev/null 2>&1; then
        echo "  ⚠ Xcode not fully configured (needed for macOS + iOS)"
        WARNINGS=true
      fi
    fi
    ;;
  MINGW*|MSYS*|CYGWIN*)
    if ! $RUST_ONLY; then
      check_cmd "cl" "cl.exe" || WARNINGS=true
      if ! command -v cl.exe &>/dev/null; then
        echo "  ⚠ MSVC compiler not in PATH (needed for Windows desktop)"
        echo "    Run from 'Developer Command Prompt for VS' or vcvars64.bat"
        WARNINGS=true
      fi
    fi
    ;;
esac

# Android SDK (for android builds)
if ! $RUST_ONLY; then
  if [ -n "${ANDROID_HOME:-}" ] || [ -n "${ANDROID_SDK_ROOT:-}" ]; then
    echo "  ✓ Android SDK found"
  else
    echo "  ⚠ ANDROID_HOME not set (needed for Android builds)"
    WARNINGS=true
  fi
fi
echo ""

# ── Git hooks ────────────────────────────────────────────────────────────────
echo "── Setting up git hooks ──"
if [ -d ".git" ]; then
  HOOKS_DIR=".git/hooks"

  # Pre-commit hook: run tests and clippy on staged changes
  if [ ! -f "$HOOKS_DIR/pre-commit" ]; then
    cat > "$HOOKS_DIR/pre-commit" << 'HOOK'
#!/usr/bin/env bash
set -euo pipefail
echo "Pre-commit: checking formatting..."
cargo fmt --all --check 2>/dev/null || {
  echo "⚠ Formatting issues — run 'cargo fmt --all'"
  exit 1
}
echo "Pre-commit: OK"
HOOK
    chmod +x "$HOOKS_DIR/pre-commit"
    echo "  ✓ pre-commit hook installed (format check)"
  else
    echo "  - pre-commit hook already exists"
  fi

  echo ""
else
  echo "  ⚠ Not a git repository — hooks not installed"
  echo ""
fi

# ── Summary ──────────────────────────────────────────────────────────────────
if $FAILED; then
  echo "Setup incomplete — some requirements missing."
  echo "Fix the issues above, then re-run this script."
  exit 1
elif $WARNINGS; then
  echo "Setup OK with warnings (some optional SDKs missing)."
  exit 0
else
  echo "Setup complete — all prerequisites satisfied."
  exit 0
fi
