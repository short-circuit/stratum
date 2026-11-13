#!/usr/bin/env bash
# Capture the native Stratum mobile shell at the shipped Android size.
#
# The shell is started with STRATUM_FIXTURE=1 and STRATUM_MOBILE=1 so the
# capture is deterministic and needs no vault. A private Xvfb screen of
# 1080×2400 is created when DISPLAY is unset, the window is pinned to the
# top-left and the root window is captured with ImageMagick. Every capture uses
# STRATUM_INSETS=48,0,72,0,0 so the status-bar, home-indicator and gesture-area
# geometry matches the Android host without a real device.
#
# Fifteen states are captured:
#
#   1. the journal tab in edit mode (STRATUM_ROUTE=/journal)
#   2. the journal with the calendar sheet expanded (STRATUM_CALENDAR=1)
#   3. the page surface (STRATUM_ROUTE=page STRATUM_PAGE=pages/fixture.md)
#   4. the search tab with a full-text query (STRATUM_ROUTE=/search STRATUM_SEARCH=todo)
#   5. the graph tab in its mobile 2D layout (STRATUM_ROUTE=/graph)
#   6. the More sheet over the journal (STRATUM_MOBILE_MORE=1)
#   7. the Ask Notes surface with an answer
#      (STRATUM_ROUTE=/ask STRATUM_ASK="what did I decide about the project?")
#   8. the Query surface with the default query run
#      (STRATUM_ROUTE=/query STRATUM_QUERY_RUN=1)
#   9. the Templates surface (STRATUM_ROUTE=/templates)
#  10. the Flashcards surface (STRATUM_ROUTE=/flashcards)
#  11. the Kanban surface (STRATUM_ROUTE=/kanban)
#  12. the Reminders surface (STRATUM_ROUTE=/reminders)
#  13. the Whiteboards gallery (STRATUM_ROUTE=/whiteboards)
#  14. the Plugins installed tab (STRATUM_ROUTE=/plugins)
#  15. the Settings surface (STRATUM_ROUTE=/settings)
#
# The PNGs are written to dist/gpui-mobile-journal.png,
# dist/gpui-mobile-journal-calendar.png, dist/gpui-mobile-page.png,
# dist/gpui-mobile-search.png, dist/gpui-mobile-graph.png,
# dist/gpui-mobile-more.png, dist/gpui-mobile-ask.png, dist/gpui-mobile-query.png,
# dist/gpui-mobile-templates.png, dist/gpui-mobile-flashcards.png,
# dist/gpui-mobile-kanban.png, dist/gpui-mobile-reminders.png,
# dist/gpui-mobile-whiteboards.png, dist/gpui-mobile-plugins.png and
# dist/gpui-mobile-settings.png.
#
# The script fails unless all fifteen files exist and each is exactly
# 1080×2400. Pass a path as the first argument or set OUTPUT to override the
# journal capture (the other captures derive from its directory).
#
# Requirements: Xvfb, xdotool and ImageMagick (`import`, `identify`). Run from
# any directory; the script locates the workspace itself.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="${1:-${OUTPUT:-$ROOT/dist/gpui-mobile-journal.png}}"
OUT_DIR="$(dirname "$OUT")"
LOG="${CAPTURE_LOG:-$OUT_DIR/gpui-mobile.log}"
WIDTH="${CAPTURE_WIDTH:-1080}"
HEIGHT="${CAPTURE_HEIGHT:-2400}"
DISPLAY_NUM="${CAPTURE_DISPLAY:-98}"

mkdir -p "$OUT_DIR"

export STRATUM_FIXTURE=1
export STRATUM_MOBILE=1
export STRATUM_INSETS="${STRATUM_INSETS:-48,0,72,0,0}"
export STRATUM_LOCALE="${STRATUM_LOCALE:-en}"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
export LIBGL_ALWAYS_SOFTWARE=1
export WINIT_UNIX_BACKEND=x11
# gpui prefers Wayland when it is advertised, but the capture drives the window
# through xdotool, so force the X11 backend even on a Wayland session.
unset WAYLAND_DISPLAY
export RUST_BACKTRACE=1

for tool in Xvfb xdotool import identify; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        echo "capture-gpui-mobile: missing required tool: $tool" >&2
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

# Build first so the timed capture windows are not spent compiling.
cargo build -p stratum-ui --bin stratum --manifest-path "$ROOT/Cargo.toml"

if [[ -z "${DISPLAY:-}" || "${CAPTURE_USE_EXISTING_DISPLAY:-0}" != "1" ]]; then
    # The mobile capture must own an exact 1080×2400 screen; a developer's
    # desktop display would otherwise decide the PNG size. Pick the first free
    # X display so a stale lock never blocks the run.
    for candidate in $(seq 90 110); do
        if [[ ! -e "/tmp/.X${candidate}-lock" ]]; then
            DISPLAY_NUM="$candidate"
            break
        fi
    done
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
    geometry="$(xdotool getdisplaygeometry 2>/dev/null || true)"
    if [[ "$geometry" != "$WIDTH $HEIGHT" ]]; then
        echo "capture-gpui-mobile: Xvfb geometry is ${geometry:-unavailable}, expected $WIDTH $HEIGHT" >&2
        exit 1
    fi
fi

# Start the shell, wait for its window, normalise its geometry and capture it.
# $1 = output path; the remaining arguments are environment assignments passed
# verbatim to `env` (quoting keeps values with spaces intact).
capture_state() {
    local out="$1"
    shift
    local window_id=""

    env "$@" "$CARGO_TARGET_DIR/debug/stratum" >"$LOG" 2>&1 &
    APP_PID=$!

    # gpui maps the window asynchronously; poll instead of a fixed sleep so fast
    # machines do not capture an empty root window.
    for _ in $(seq 1 120); do
        if ! kill -0 "$APP_PID" 2>/dev/null; then
            echo "capture-gpui-mobile: shell exited early; see $LOG" >&2
            cat "$LOG" >&2 || true
            exit 1
        fi
        window_id="$(xdotool search --onlyvisible --name 'Stratum' 2>/dev/null | head -n1 || true)"
        if [[ -n "$window_id" ]]; then
            break
        fi
        sleep 0.25
    done

    if [[ -z "$window_id" ]]; then
        echo "capture-gpui-mobile: timed out waiting for the Stratum window" >&2
        cat "$LOG" >&2 || true
        exit 1
    fi

    # Pin the window to the top-left so the capture is exactly the shell bounds
    # (no window-manager decorations under Xvfb, but this keeps it deterministic
    # if the app is started on a real display).
    xdotool windowmove "$window_id" 0 0 || true
    xdotool windowsize "$window_id" "$WIDTH" "$HEIGHT" || true
    xdotool windowactivate "$window_id" 2>/dev/null || true
    # Let fonts, icons and the first paint settle.
    sleep 1.5

    import -display "$DISPLAY" -window root "$out"

    if [[ ! -s "$out" ]]; then
        echo "capture-gpui-mobile: screenshot was not written to $out" >&2
        exit 1
    fi

    kill "$APP_PID" 2>/dev/null || true
    wait "$APP_PID" 2>/dev/null || true
    APP_PID=""
    echo "captured $out (${WIDTH}x${HEIGHT})"
}

capture_state "$OUT" STRATUM_ROUTE=/journal
capture_state "$OUT_DIR/gpui-mobile-journal-calendar.png" \
    STRATUM_ROUTE=/journal STRATUM_CALENDAR=1
capture_state "$OUT_DIR/gpui-mobile-page.png" \
    STRATUM_ROUTE=page STRATUM_PAGE=pages/fixture.md
capture_state "$OUT_DIR/gpui-mobile-search.png" \
    STRATUM_ROUTE=/search STRATUM_SEARCH=todo
capture_state "$OUT_DIR/gpui-mobile-graph.png" STRATUM_ROUTE=/graph
capture_state "$OUT_DIR/gpui-mobile-more.png" \
    STRATUM_ROUTE=/journal STRATUM_MOBILE_MORE=1
capture_state "$OUT_DIR/gpui-mobile-ask.png" \
    STRATUM_ROUTE=/ask STRATUM_ASK="what did I decide about the project?"
capture_state "$OUT_DIR/gpui-mobile-query.png" \
    STRATUM_ROUTE=/query STRATUM_QUERY_RUN=1
capture_state "$OUT_DIR/gpui-mobile-templates.png" STRATUM_ROUTE=/templates
capture_state "$OUT_DIR/gpui-mobile-flashcards.png" STRATUM_ROUTE=/flashcards
capture_state "$OUT_DIR/gpui-mobile-kanban.png" STRATUM_ROUTE=/kanban
capture_state "$OUT_DIR/gpui-mobile-reminders.png" STRATUM_ROUTE=/reminders
capture_state "$OUT_DIR/gpui-mobile-whiteboards.png" STRATUM_ROUTE=/whiteboards
capture_state "$OUT_DIR/gpui-mobile-plugins.png" STRATUM_ROUTE=/plugins
capture_state "$OUT_DIR/gpui-mobile-settings.png" STRATUM_ROUTE=/settings

# Verify the deliverable: fifteen PNGs, each exactly 1080×2400.
EXPECTED=(
    gpui-mobile-journal.png
    gpui-mobile-journal-calendar.png
    gpui-mobile-page.png
    gpui-mobile-search.png
    gpui-mobile-graph.png
    gpui-mobile-more.png
    gpui-mobile-ask.png
    gpui-mobile-query.png
    gpui-mobile-templates.png
    gpui-mobile-flashcards.png
    gpui-mobile-kanban.png
    gpui-mobile-reminders.png
    gpui-mobile-whiteboards.png
    gpui-mobile-plugins.png
    gpui-mobile-settings.png
)

count=0
for name in "${EXPECTED[@]}"; do
    path="$OUT_DIR/$name"
    if [[ ! -s "$path" ]]; then
        echo "capture-gpui-mobile: missing capture: $path" >&2
        exit 1
    fi
    dimensions="$(identify -format '%wx%h' "$path" 2>/dev/null || true)"
    if [[ "$dimensions" != "${WIDTH}x${HEIGHT}" ]]; then
        echo "capture-gpui-mobile: $path is ${dimensions:-unreadable}, expected ${WIDTH}x${HEIGHT}" >&2
        exit 1
    fi
    count=$((count + 1))
done

echo "captured $count mobile screenshots at ${WIDTH}x${HEIGHT} in $OUT_DIR"
