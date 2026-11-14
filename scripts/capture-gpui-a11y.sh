#!/usr/bin/env bash
# Dump the native shell's AccessKit tree for the accessibility probe.
#
# Opens the deterministic fixture shell at 1440x900 on a private Xvfb screen and
# prints the tree gpui reports to the platform adapter
# (`Window::debug_a11y_tree_json`). When no assistive technology is connected the
# tree is not built, and the probe prints `{"a11y": "inactive"}`.
#
# The probe runs inside a private D-Bus session so accesskit_unix can register on
# the AT-SPI bus; `scripts/verify-gpui-a11y.sh` interprets the JSON. Install
# `at-spi2-core` (and a screen reader or `accerciser`) for an active tree.
#
# Output: `.hermes/a11y/tree.json` by default (override with $A11Y_OUT or $1).
# Requirements: Xvfb; D-Bus for an active tree. Run from any directory.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="${1:-${A11Y_OUT:-$ROOT/.hermes/a11y/tree.json}}"
WIDTH="${CAPTURE_WIDTH:-1440}"
HEIGHT="${CAPTURE_HEIGHT:-900}"
DISPLAY_NUM="${CAPTURE_DISPLAY:-97}"
LOG="${A11Y_LOG:-$ROOT/.hermes/a11y/probe.log}"
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"

mkdir -p "$(dirname "$OUT")" "$(dirname "$LOG")"

# Re-exec inside a private session bus when one is not already active, so the
# AT-SPI registry (when at-spi2-core is installed) can see the probe.
if [[ -z "${DBUS_SESSION_BUS_ADDRESS:-}" ]] && command -v dbus-run-session >/dev/null 2>&1; then
    export A11Y_OUT="$OUT"
    exec dbus-run-session -- "$0" "$@"
fi

export STRATUM_FIXTURE=1
export STRATUM_LOCALE="${STRATUM_LOCALE:-en}"
export CARGO_TARGET_DIR="$TARGET_DIR"
export LIBGL_ALWAYS_SOFTWARE=1
export WINIT_UNIX_BACKEND=x11
# gpui prefers Wayland when it is advertised, and the probe runs on an X11
# Xvfb screen, so pin the backend.
unset WAYLAND_DISPLAY
export RUST_BACKTRACE=1
export NO_AT_BRIDGE=0

cargo build -p stratum-ui --bin stratum-a11y-probe --manifest-path "$ROOT/Cargo.toml"

for tool in Xvfb xdotool; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        echo "capture-gpui-a11y: missing required tool: $tool" >&2
        exit 1
    fi
done

# shellcheck source=lib/capture-display.sh
source "$ROOT/scripts/lib/capture-display.sh"

XVFB_PID=""
cleanup() {
    if [[ -n "$XVFB_PID" ]]; then
        kill "$XVFB_PID" 2>/dev/null || true
        wait "$XVFB_PID" 2>/dev/null || true
    fi
}
trap cleanup EXIT

if [[ -z "${DISPLAY:-}" ]]; then
    acquire_capture_display "$WIDTH" "$HEIGHT" "$DISPLAY_NUM" || exit 1
fi

"$TARGET_DIR/debug/stratum-a11y-probe" >"$OUT" 2>"$LOG"
if [[ ! -s "$OUT" ]]; then
    echo "capture-gpui-a11y: probe wrote no tree (see $LOG)" >&2
    exit 1
fi
echo "captured $OUT"
