#!/usr/bin/env bash
# Capture screenshots of the native Stratum journal, page, search and query
# surfaces.
#
# The shell is started with STRATUM_FIXTURE=1 so the capture is deterministic
# and needs no vault. When no DISPLAY is set the script starts a private Xvfb
# screen, waits for the window to map, and captures the root window with
# ImageMagick. Fourteen states are captured:
#
#   1. the journal route in edit mode
#   2. the journal route with the calendar popup expanded (STRATUM_CALENDAR=1)
#   3. the journal route on a deeper past day in read mode
#      (STRATUM_JOURNAL_DATE=2026-11-07 STRATUM_READ_MODE=1), which scrolls that
#      day into view
#   4. the fixture page in edit mode (STRATUM_ROUTE=page STRATUM_PAGE=pages/fixture.md)
#   5. the fixture page with the Note menu open (STRATUM_MENU=note)
#   6. the fixture page in read mode (STRATUM_READ_MODE=1)
#   7. the fixture page with backlinks/suggestions expanded (STRATUM_CARDS=open)
#   8. the fixture page with a block revealed (STRATUM_HIGHLIGHT=<fixture id>)
#   9. the fixture page with the backlink snippet card preloaded
#      (STRATUM_CARDS=open STRATUM_SNIPPET=1)
#  10. the search route with a full-text query submitted
#      (STRATUM_ROUTE=/search STRATUM_SEARCH=todo)
#  11. the search route in tag mode (STRATUM_ROUTE=/search STRATUM_SEARCH=#project)
#  12. the query route on the builder tab (STRATUM_ROUTE=/query)
#  13. the query route after running the default query
#      (STRATUM_ROUTE=/query STRATUM_QUERY_RUN=1)
#  14. the query route on the raw Datalog tab
#      (STRATUM_ROUTE=/query STRATUM_QUERY_TAB=datalog)
#
# The journal PNGs are written to dist/gpui-journal.png,
# dist/gpui-journal-calendar.png and dist/gpui-journal-read.png; the page PNGs to
# dist/gpui-page.png, dist/gpui-page-note-menu.png, dist/gpui-page-read.png,
# dist/gpui-page-cards.png, dist/gpui-page-highlight.png and
# dist/gpui-page-snippet.png; the search PNGs to dist/gpui-search.png and
# dist/gpui-search-tag.png; the query PNGs to dist/gpui-query.png,
# dist/gpui-query-results.png and dist/gpui-query-datalog.png. Pass a path as the
# first argument or set OUTPUT to override the journal edit-mode capture (the
# other captures derive from its directory).
#
# Requirements: Xvfb, xdotool and ImageMagick's `import` (the CI job installs
# them). Run from any directory; the script locates the workspace itself.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="${1:-${OUTPUT:-$ROOT/dist/gpui-journal.png}}"
CALENDAR_OUT="${OUT%.png}-calendar.png"
READ_OUT="${OUT%.png}-read.png"
OUT_DIR="$(dirname "$OUT")"
PAGE_OUT="$OUT_DIR/gpui-page.png"
PAGE_MENU_OUT="$OUT_DIR/gpui-page-note-menu.png"
PAGE_READ_OUT="$OUT_DIR/gpui-page-read.png"
PAGE_CARDS_OUT="$OUT_DIR/gpui-page-cards.png"
PAGE_HIGHLIGHT_OUT="$OUT_DIR/gpui-page-highlight.png"
PAGE_SNIPPET_OUT="$OUT_DIR/gpui-page-snippet.png"
SEARCH_OUT="$OUT_DIR/gpui-search.png"
SEARCH_TAG_OUT="$OUT_DIR/gpui-search-tag.png"
QUERY_OUT="$OUT_DIR/gpui-query.png"
QUERY_RESULTS_OUT="$OUT_DIR/gpui-query-results.png"
QUERY_DATALOG_OUT="$OUT_DIR/gpui-query-datalog.png"
# Deterministic block on pages/fixture.md (provider::FIXTURE_HIGHLIGHT_BLOCK).
FIXTURE_HIGHLIGHT="413cf12c-969f-54ef-bf75-40ffd89e8618"
LOG="${CAPTURE_LOG:-$OUT_DIR/gpui-journal.log}"
WIDTH="${CAPTURE_WIDTH:-1280}"
HEIGHT="${CAPTURE_HEIGHT:-720}"
DISPLAY_NUM="${CAPTURE_DISPLAY:-99}"

mkdir -p "$(dirname "$OUT")" "$(dirname "$LOG")"

export STRATUM_FIXTURE=1
export STRATUM_LOCALE="${STRATUM_LOCALE:-en}"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
export LIBGL_ALWAYS_SOFTWARE=1
export WINIT_UNIX_BACKEND=x11
# gpui prefers Wayland when it is advertised, but the capture drives the window
# through xdotool, so force the X11 backend even on a Wayland session.
unset WAYLAND_DISPLAY
export RUST_BACKTRACE=1

for tool in Xvfb xdotool import; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        echo "capture-gpui-panels: missing required tool: $tool" >&2
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

# Start the shell, wait for its window, normalise its geometry and capture it.
# $1 = extra environment assignments (space separated, empty for the default).
capture_state() {
    local extra_env="$1"
    local out="$2"
    local window_id=""

    env $extra_env "$CARGO_TARGET_DIR/debug/stratum" >"$LOG" 2>&1 &
    APP_PID=$!

    # gpui maps the window asynchronously; poll instead of a fixed sleep so fast
    # machines do not capture an empty root window.
    for _ in $(seq 1 120); do
        if ! kill -0 "$APP_PID" 2>/dev/null; then
            echo "capture-gpui-panels: shell exited early; see $LOG" >&2
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
        echo "capture-gpui-panels: timed out waiting for the Stratum window" >&2
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
        echo "capture-gpui-panels: screenshot was not written to $out" >&2
        exit 1
    fi

    kill "$APP_PID" 2>/dev/null || true
    wait "$APP_PID" 2>/dev/null || true
    APP_PID=""
    echo "captured $out (${WIDTH}x${HEIGHT})"
}

capture_state "STRATUM_ROUTE=/journal" "$OUT"
capture_state "STRATUM_ROUTE=/journal STRATUM_CALENDAR=1" "$CALENDAR_OUT"
capture_state \
    "STRATUM_ROUTE=/journal STRATUM_JOURNAL_DATE=2026-11-07 STRATUM_READ_MODE=1" \
    "$READ_OUT"

capture_state \
    "STRATUM_ROUTE=page STRATUM_PAGE=pages/fixture.md" \
    "$PAGE_OUT"
capture_state \
    "STRATUM_ROUTE=page STRATUM_PAGE=pages/fixture.md STRATUM_MENU=note" \
    "$PAGE_MENU_OUT"
capture_state \
    "STRATUM_ROUTE=page STRATUM_PAGE=pages/fixture.md STRATUM_READ_MODE=1" \
    "$PAGE_READ_OUT"
capture_state \
    "STRATUM_ROUTE=page STRATUM_PAGE=pages/fixture.md STRATUM_CARDS=open" \
    "$PAGE_CARDS_OUT"
capture_state \
    "STRATUM_ROUTE=page STRATUM_PAGE=pages/fixture.md STRATUM_HIGHLIGHT=$FIXTURE_HIGHLIGHT" \
    "$PAGE_HIGHLIGHT_OUT"
capture_state \
    "STRATUM_ROUTE=page STRATUM_PAGE=pages/fixture.md STRATUM_CARDS=open STRATUM_SNIPPET=1" \
    "$PAGE_SNIPPET_OUT"

capture_state \
    "STRATUM_ROUTE=/search STRATUM_SEARCH=todo" \
    "$SEARCH_OUT"
capture_state \
    "STRATUM_ROUTE=/search STRATUM_SEARCH=#project" \
    "$SEARCH_TAG_OUT"

capture_state "STRATUM_ROUTE=/query" "$QUERY_OUT"
capture_state \
    "STRATUM_ROUTE=/query STRATUM_QUERY_RUN=1" \
    "$QUERY_RESULTS_OUT"
capture_state \
    "STRATUM_ROUTE=/query STRATUM_QUERY_TAB=datalog" \
    "$QUERY_DATALOG_OUT"
