#!/usr/bin/env bash
# Capture a screenshot of the native Stratum gpui shell.
#
# The shell is started with STRATUM_FIXTURE=1 so the capture is deterministic
# and needs no vault. When no DISPLAY is set the script starts a private Xvfb
# screen, waits for the window to map, and captures the root window with
# ImageMagick. The resulting PNG is written to dist/gpui-shell.png by default;
# pass a path as the first argument or set OUTPUT to override it.
#
# Requirements: Xvfb, xdotool and ImageMagick's `import` (the CI job installs
# them). Run from any directory; the script locates the workspace itself.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="${1:-${OUTPUT:-$ROOT/dist/gpui-shell.png}}"
LOG="${CAPTURE_LOG:-$ROOT/dist/gpui-shell.log}"
WIDTH="${CAPTURE_WIDTH:-1280}"
HEIGHT="${CAPTURE_HEIGHT:-720}"
DISPLAY_NUM="${CAPTURE_DISPLAY:-99}"

mkdir -p "$(dirname "$OUT")" "$(dirname "$LOG")"

export STRATUM_FIXTURE=1
export STRATUM_LOCALE="${STRATUM_LOCALE:-en}"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
export LIBGL_ALWAYS_SOFTWARE=1
export WINIT_UNIX_BACKEND=x11
export RUST_BACKTRACE=1

for tool in Xvfb xdotool import; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        echo "capture-gpui-shell: missing required tool: $tool" >&2
        exit 1
    fi
done

XVFB_PID=""
APP_PID=""
cleanup() {
    if [[ -n "$APP_PID" ]]; then
        kill "$APP_PID" 2>/dev/null || true
        wait "$APP_PID" 2>/dev/null || true
    fi
    if [[ -n "$XVFB_PID" ]]; then
        kill "$XVFB_PID" 2>/dev/null || true
        wait "$XVFB_PID" 2>/dev/null || true
    fi
}
trap cleanup EXIT

# Build first so the timed capture window is not spent compiling.
cargo build -p stratum-ui --bin stratum-ui --manifest-path "$ROOT/Cargo.toml"

if [[ -z "${DISPLAY:-}" ]]; then
    Xvfb ":$DISPLAY_NUM" -screen "0" "${WIDTH}x${HEIGHT}x24" -nolisten tcp \
        >/dev/null 2>&1 &
    XVFB_PID=$!
    export DISPLAY=":$DISPLAY_NUM"
    # Give Xvfb a moment to create its socket before the client connects.
    for _ in $(seq 1 40); do
        if xdotool getdisplaygeometry >/dev/null 2>&1; then
            break
        fi
        sleep 0.25
    done
fi

"$CARGO_TARGET_DIR/debug/stratum-ui" >"$LOG" 2>&1 &
APP_PID=$!

# Wait for the shell window. gpui maps it asynchronously; poll instead of a
# fixed sleep so fast machines do not capture an empty root window.
WINDOW_ID=""
for _ in $(seq 1 120); do
    if ! kill -0 "$APP_PID" 2>/dev/null; then
        echo "capture-gpui-shell: shell exited early; see $LOG" >&2
        cat "$LOG" >&2 || true
        exit 1
    fi
    WINDOW_ID="$(xdotool search --onlyvisible --name 'Stratum' 2>/dev/null | head -n1 || true)"
    if [[ -n "$WINDOW_ID" ]]; then
        break
    fi
    sleep 0.25
done

if [[ -z "$WINDOW_ID" ]]; then
    echo "capture-gpui-shell: timed out waiting for the Stratum window" >&2
    cat "$LOG" >&2 || true
    exit 1
fi

# Pin the window to the top-left so the capture is exactly the shell bounds
# (no window-manager decorations under Xvfb, but this keeps it deterministic
# if the app is started on a real display).
xdotool windowmove "$WINDOW_ID" 0 0 || true
xdotool windowsize "$WINDOW_ID" "$WIDTH" "$HEIGHT" || true
xdotool windowactivate "$WINDOW_ID" 2>/dev/null || true
# Let fonts, icons and the first paint settle.
sleep 1.5

import -display "$DISPLAY" -window root "$OUT"

if [[ ! -s "$OUT" ]]; then
    echo "capture-gpui-shell: screenshot was not written to $OUT" >&2
    exit 1
fi

echo "captured $OUT (${WIDTH}x${HEIGHT})"
