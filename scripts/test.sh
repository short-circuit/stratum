#!/usr/bin/env bash
set -euo pipefail

# ─── Test Script for Stratum ─────────────────────────────────────────────────
# Runs all tests: Rust unit/integration/doc, Flutter, linting.
#
# Usage:
#   ./scripts/test.sh                         # Full test suite
#   ./scripts/test.sh --rust-only             # Rust tests only
#   ./scripts/test.sh --flutter-only          # Flutter tests only
#   ./scripts/test.sh --coverage              # Rust tests with code coverage
#   ./scripts/test.sh --quick                 # Skip lints and slow tests
# ──────────────────────────────────────────────────────────────────────────────

RUST_ONLY=false
FLUTTER_ONLY=false
COVERAGE=false
QUICK=false

while [[ $# -gt 0 ]]; do
  case "$1" in
    --rust-only) RUST_ONLY=true; shift ;;
    --flutter-only) FLUTTER_ONLY=true; shift ;;
    --coverage) COVERAGE=true; shift ;;
    --quick) QUICK=true; shift ;;
    --help|-h)
      sed -n '/^# ───/,/^# ──/{/^# ──/d;p}' "$0" | head -n -1
      exit 0
      ;;
    *) echo "Unknown option: $1"; exit 1 ;;
  esac
done

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

FAILURES=0

echo "╔════════════════════════════════╗"
echo "║  Stratum Test Suite            ║"
echo "╚════════════════════════════════╝"
echo ""

# ── Rust tests ───────────────────────────────────────────────────────────────
if ! $FLUTTER_ONLY; then
  echo "── Rust unit + integration tests ──"
  if cargo test --workspace; then
    echo "  ✓ Rust tests passed"
  else
    echo "  ✗ Rust tests failed"
    ((FAILURES++))
  fi
  echo ""

  if ! $QUICK; then
    echo "── Rust doc tests ──"
    if cargo test --workspace --doc; then
      echo "  ✓ Rust doc tests passed"
    else
      echo "  ✗ Rust doc tests failed"
      ((FAILURES++))
    fi
    echo ""

    echo "── Clippy ──"
    if cargo clippy --workspace -- -D warnings 2>/dev/null; then
      echo "  ✓ Clippy passed"
    else
      echo "  ⚠ Clippy found issues"
    fi
    echo ""

    echo "── Rust formatting ──"
    if cargo fmt --all --check 2>/dev/null; then
      echo "  ✓ Formatting OK"
    else
      echo "  ✗ Formatting issues found (run 'cargo fmt --all')"
      ((FAILURES++))
    fi
    echo ""
  fi

  # ── Coverage (requires cargo-tarpaulin or llvm-tools) ────────────────────
  if $COVERAGE; then
    if command -v cargo-tarpaulin &>/dev/null; then
      echo "── Code coverage ──"
      cargo tarpaulin --workspace --out html --out lcov --output-dir coverage/
      echo "  ✓ Coverage report: coverage/html/index.html"
      echo ""
    elif rustup component list --installed 2>/dev/null | grep -q llvm-tools; then
      echo "── Code coverage (llvm-cov) ──"
      cargo llvm-cov --workspace --html --output-dir coverage/
      echo "  ✓ Coverage report: coverage/html/index.html"
      echo ""
    else
      echo "── Coverage skipped ──"
      echo "  Install cargo-tarpaulin or cargo-llvm-cov to enable coverage."
      echo ""
    fi
  fi
fi

# ── Flutter tests ────────────────────────────────────────────────────────────
if ! $RUST_ONLY; then
  if [ ! -d "frontend" ] || [ ! -f "frontend/pubspec.yaml" ]; then
    echo "── Flutter frontend not found — skipping ──"
    echo ""
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
      echo "── Flutter not found on PATH — skipping Flutter tests ──"
      echo ""
      cd "$ROOT" 2>/dev/null || true
      return
    fi

    cd frontend

    echo "── Flutter tests ──"
    if $FLUTTER_CMD test; then
      echo "  ✓ Flutter tests passed"
    else
      echo "  ✗ Flutter tests failed"
      ((FAILURES++))
    fi
    echo ""

    if ! $QUICK; then
      echo "── Flutter analyze ──"
      if $FLUTTER_CMD analyze; then
        echo "  ✓ Flutter analyze passed"
      else
        echo "  ⚠ Flutter analyze found issues"
      fi
      echo ""
    fi

    cd "$ROOT"
  fi
fi

# ── Summary ──────────────────────────────────────────────────────────────────
if [ "$FAILURES" -eq 0 ]; then
  echo "All checks passed."
else
  echo "$FAILURES test suite(s) failed."
  exit 1
fi
