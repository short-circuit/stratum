#!/usr/bin/env bash
# Capture screenshots of the native Stratum journal, page, search, query, Ask
# Notes, Templates, Flashcards, Reminders, Plugins and Graph surfaces.
#
# The shell is started with STRATUM_FIXTURE=1 so the capture is deterministic
# and needs no vault. When no DISPLAY is set the script starts a private Xvfb
# screen, waits for the window to map, and captures the root window with
# ImageMagick. Forty-three states are captured:
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
#  15. the Ask Notes route with a question answered
#      (STRATUM_ROUTE=/ask STRATUM_ASK="what did I decide about the project?")
#  16. the Templates route with the fixture templates
#      (STRATUM_ROUTE=/templates)
#  17. the Flashcards route on the first question
#      (STRATUM_ROUTE=/flashcards)
#  18. the Flashcards route with the first answer revealed
#      (STRATUM_ROUTE=/flashcards STRATUM_FLASHCARD_REVEAL=1)
#  19. the Reminders route with the grouped fixture rows
#      (STRATUM_ROUTE=/reminders)
#  20. the Reminders route with the due dialog open for the first overdue row
#      (STRATUM_ROUTE=/reminders STRATUM_REMINDER_DIALOG=<block id>)
#  21. the journal route with a due-reminder toast and its open action
#      (STRATUM_ROUTE=/journal STRATUM_REMINDER_TOAST=1)
#  22. the Plugins route on the installed tab (STRATUM_ROUTE=/plugins)
#  23. the Plugins route on the community directory tab
#      (STRATUM_ROUTE=/plugins STRATUM_PLUGINS_TAB=directory)
#  24. the Plugins route on the theme hub tab
#      (STRATUM_ROUTE=/plugins STRATUM_PLUGINS_TAB=themes)
#  25. the Graph route with the full fixture graph (STRATUM_ROUTE=/graph)
#  26. the Graph route filtered to a connected component
#      (STRATUM_ROUTE=/graph STRATUM_GRAPH_MODE=component)
#  27. the Graph route with the settings panel expanded
#      (STRATUM_ROUTE=/graph STRATUM_GRAPH_SETTINGS=1)
#  28. the Whiteboards gallery with the fixture boards
#      (STRATUM_ROUTE=/whiteboards)
#  29. the whiteboard editor on the framed fixture board
#      (STRATUM_ROUTE=/whiteboards STRATUM_WHITEBOARD="Architecture sketch")
#  30. the whiteboard editor with the library drawer open
#      (… STRATUM_WHITEBOARD_LIBRARY=1)
#  31. the whiteboard editor with the style panel open
#      (… STRATUM_WHITEBOARD_STYLE=1)
#  32. the whiteboard editor with an inline text edit overlay open
#      (… STRATUM_WHITEBOARD_TEXT=1)
#  33. the Settings route on the vault tab (STRATUM_ROUTE=/settings)
#  34. the Settings route on the theme tab
#      (STRATUM_ROUTE=/settings STRATUM_SETTINGS_TAB=theme)
#  35. the Settings route on the AI tab
#      (STRATUM_ROUTE=/settings STRATUM_SETTINGS_TAB=ai)
#  36. the Settings route on the research tab
#      (STRATUM_ROUTE=/settings STRATUM_SETTINGS_TAB=research)
#  37. the Settings route on the developer tab with the commit log expanded
#      (STRATUM_ROUTE=/settings STRATUM_SETTINGS_TAB=developer STRATUM_SETTINGS_COMMITS=1)
#  38. the Settings route on the sync tab
#      (STRATUM_ROUTE=/settings STRATUM_SETTINGS_TAB=sync)
#  39. the Settings route on the sync tab with seeded merge conflicts
#      (STRATUM_ROUTE=/settings STRATUM_SETTINGS_TAB=sync STRATUM_SETTINGS_CONFLICTS=1)
#  40. the Settings route on the tray tab
#      (STRATUM_ROUTE=/settings STRATUM_SETTINGS_TAB=tray)
#  41. the Settings route on the notifications tab
#      (STRATUM_ROUTE=/settings STRATUM_SETTINGS_TAB=notifications)
#  42. the Quick Capture dialog in text mode over the journal
#      (STRATUM_ROUTE=/journal STRATUM_QUICK_CAPTURE=1)
#  43. the Quick Capture dialog in voice mode
#      (STRATUM_ROUTE=/journal STRATUM_QUICK_CAPTURE_VOICE=1)
#
# The journal PNGs are written to dist/gpui-journal.png,
# dist/gpui-journal-calendar.png and dist/gpui-journal-read.png; the page PNGs to
# dist/gpui-page.png, dist/gpui-page-note-menu.png, dist/gpui-page-read.png,
# dist/gpui-page-cards.png, dist/gpui-page-highlight.png and
# dist/gpui-page-snippet.png; the search PNGs to dist/gpui-search.png and
# dist/gpui-search-tag.png; the query PNGs to dist/gpui-query.png,
# dist/gpui-query-results.png and dist/gpui-query-datalog.png; the Ask Notes PNG
# to dist/gpui-ask.png; the Templates PNG to dist/gpui-templates.png; the
# Flashcards PNGs to dist/gpui-flashcards.png and dist/gpui-flashcards-answer.png;
# the Reminders PNGs to dist/gpui-reminders.png, dist/gpui-reminders-dialog.png
# and dist/gpui-reminder-toast.png; the Plugins PNGs to dist/gpui-plugins.png,
# dist/gpui-plugins-directory.png and dist/gpui-plugins-themes.png; the Graph PNGs
# to dist/gpui-graph.png, dist/gpui-graph-component.png and
# dist/gpui-graph-settings.png; the Whiteboards PNGs to dist/gpui-whiteboards.png,
# dist/gpui-whiteboard-editor.png, dist/gpui-whiteboard-library.png,
# dist/gpui-whiteboard-style.png and dist/gpui-whiteboard-text.png; the Settings
# PNGs to dist/gpui-settings-vault.png, dist/gpui-settings-theme.png,
# dist/gpui-settings-ai.png, dist/gpui-settings-research.png,
# dist/gpui-settings-developer.png, dist/gpui-settings-sync.png,
# dist/gpui-settings-sync-conflicts.png, dist/gpui-settings-tray.png and
# dist/gpui-settings-notifications.png.
# Pass a path as the first argument or set OUTPUT to override the journal
# edit-mode capture (the other captures derive from its directory).
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
ASK_OUT="$OUT_DIR/gpui-ask.png"
TEMPLATES_OUT="$OUT_DIR/gpui-templates.png"
FLASHCARDS_OUT="$OUT_DIR/gpui-flashcards.png"
FLASHCARDS_ANSWER_OUT="$OUT_DIR/gpui-flashcards-answer.png"
REMINDERS_OUT="$OUT_DIR/gpui-reminders.png"
REMINDERS_DIALOG_OUT="$OUT_DIR/gpui-reminders-dialog.png"
REMINDER_TOAST_OUT="$OUT_DIR/gpui-reminder-toast.png"
PLUGINS_OUT="$OUT_DIR/gpui-plugins.png"
PLUGINS_DIRECTORY_OUT="$OUT_DIR/gpui-plugins-directory.png"
PLUGINS_THEMES_OUT="$OUT_DIR/gpui-plugins-themes.png"
GRAPH_OUT="$OUT_DIR/gpui-graph.png"
GRAPH_COMPONENT_OUT="$OUT_DIR/gpui-graph-component.png"
GRAPH_SETTINGS_OUT="$OUT_DIR/gpui-graph-settings.png"
WHITEBOARDS_OUT="$OUT_DIR/gpui-whiteboards.png"
WHITEBOARD_EDITOR_OUT="$OUT_DIR/gpui-whiteboard-editor.png"
WHITEBOARD_LIBRARY_OUT="$OUT_DIR/gpui-whiteboard-library.png"
WHITEBOARD_STYLE_OUT="$OUT_DIR/gpui-whiteboard-style.png"
WHITEBOARD_TEXT_OUT="$OUT_DIR/gpui-whiteboard-text.png"
SETTINGS_VAULT_OUT="$OUT_DIR/gpui-settings-vault.png"
SETTINGS_THEME_OUT="$OUT_DIR/gpui-settings-theme.png"
SETTINGS_AI_OUT="$OUT_DIR/gpui-settings-ai.png"
SETTINGS_RESEARCH_OUT="$OUT_DIR/gpui-settings-research.png"
SETTINGS_DEVELOPER_OUT="$OUT_DIR/gpui-settings-developer.png"
SETTINGS_SYNC_OUT="$OUT_DIR/gpui-settings-sync.png"
SETTINGS_SYNC_CONFLICTS_OUT="$OUT_DIR/gpui-settings-sync-conflicts.png"
SETTINGS_TRAY_OUT="$OUT_DIR/gpui-settings-tray.png"
SETTINGS_NOTIFICATIONS_OUT="$OUT_DIR/gpui-settings-notifications.png"
QUICK_CAPTURE_OUT="$OUT_DIR/gpui-quick-capture.png"
QUICK_CAPTURE_VOICE_OUT="$OUT_DIR/gpui-quick-capture-voice.png"
# Deterministic block on pages/fixture.md (provider::FIXTURE_HIGHLIGHT_BLOCK).
FIXTURE_HIGHLIGHT="413cf12c-969f-54ef-bf75-40ffd89e8618"
# The fixture's first overdue reminder (`overdue-1`): the provider builds block
# ids as `00000000-0000-5000-8000-{slug:0>12}`, so this id targets its dialog.
FIXTURE_REMINDER_BLOCK="00000000-0000-5000-8000-000overdue-1"
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

capture_state "$OUT" STRATUM_ROUTE=/journal
capture_state "$CALENDAR_OUT" STRATUM_ROUTE=/journal STRATUM_CALENDAR=1
capture_state \
    "$READ_OUT" \
    STRATUM_ROUTE=/journal STRATUM_JOURNAL_DATE=2026-11-07 STRATUM_READ_MODE=1

capture_state \
    "$PAGE_OUT" \
    STRATUM_ROUTE=page STRATUM_PAGE=pages/fixture.md
capture_state \
    "$PAGE_MENU_OUT" \
    STRATUM_ROUTE=page STRATUM_PAGE=pages/fixture.md STRATUM_MENU=note
capture_state \
    "$PAGE_READ_OUT" \
    STRATUM_ROUTE=page STRATUM_PAGE=pages/fixture.md STRATUM_READ_MODE=1
capture_state \
    "$PAGE_CARDS_OUT" \
    STRATUM_ROUTE=page STRATUM_PAGE=pages/fixture.md STRATUM_CARDS=open
capture_state \
    "$PAGE_HIGHLIGHT_OUT" \
    STRATUM_ROUTE=page STRATUM_PAGE=pages/fixture.md "STRATUM_HIGHLIGHT=$FIXTURE_HIGHLIGHT"
capture_state \
    "$PAGE_SNIPPET_OUT" \
    STRATUM_ROUTE=page STRATUM_PAGE=pages/fixture.md STRATUM_CARDS=open STRATUM_SNIPPET=1

capture_state \
    "$SEARCH_OUT" \
    STRATUM_ROUTE=/search STRATUM_SEARCH=todo
capture_state \
    "$SEARCH_TAG_OUT" \
    STRATUM_ROUTE=/search STRATUM_SEARCH=#project

capture_state "$QUERY_OUT" STRATUM_ROUTE=/query
capture_state \
    "$QUERY_RESULTS_OUT" \
    STRATUM_ROUTE=/query STRATUM_QUERY_RUN=1
capture_state \
    "$QUERY_DATALOG_OUT" \
    STRATUM_ROUTE=/query STRATUM_QUERY_TAB=datalog

capture_state \
    "$ASK_OUT" \
    STRATUM_ROUTE=/ask STRATUM_ASK="what did I decide about the project?"

capture_state "$TEMPLATES_OUT" STRATUM_ROUTE=/templates

capture_state "$FLASHCARDS_OUT" STRATUM_ROUTE=/flashcards
capture_state \
    "$FLASHCARDS_ANSWER_OUT" \
    STRATUM_ROUTE=/flashcards STRATUM_FLASHCARD_REVEAL=1

capture_state "$REMINDERS_OUT" STRATUM_ROUTE=/reminders
capture_state \
    "$REMINDERS_DIALOG_OUT" \
    STRATUM_ROUTE=/reminders "STRATUM_REMINDER_DIALOG=$FIXTURE_REMINDER_BLOCK"
capture_state \
    "$REMINDER_TOAST_OUT" \
    STRATUM_ROUTE=/journal STRATUM_REMINDER_TOAST=1

capture_state "$PLUGINS_OUT" STRATUM_ROUTE=/plugins
capture_state \
    "$PLUGINS_DIRECTORY_OUT" \
    STRATUM_ROUTE=/plugins STRATUM_PLUGINS_TAB=directory
capture_state \
    "$PLUGINS_THEMES_OUT" \
    STRATUM_ROUTE=/plugins STRATUM_PLUGINS_TAB=themes

capture_state "$GRAPH_OUT" STRATUM_ROUTE=/graph
capture_state \
    "$GRAPH_COMPONENT_OUT" \
    STRATUM_ROUTE=/graph STRATUM_GRAPH_MODE=component STRATUM_GRAPH_COMPONENT=0
capture_state \
    "$GRAPH_SETTINGS_OUT" \
    STRATUM_ROUTE=/graph STRATUM_GRAPH_SETTINGS=1

capture_state "$WHITEBOARDS_OUT" STRATUM_ROUTE=/whiteboards
capture_state \
    "$WHITEBOARD_EDITOR_OUT" \
    STRATUM_ROUTE=/whiteboards "STRATUM_WHITEBOARD=Architecture sketch"
capture_state \
    "$WHITEBOARD_LIBRARY_OUT" \
    STRATUM_ROUTE=/whiteboards "STRATUM_WHITEBOARD=Architecture sketch" \
    STRATUM_WHITEBOARD_LIBRARY=1
capture_state \
    "$WHITEBOARD_STYLE_OUT" \
    STRATUM_ROUTE=/whiteboards "STRATUM_WHITEBOARD=Architecture sketch" \
    STRATUM_WHITEBOARD_STYLE=1
capture_state \
    "$WHITEBOARD_TEXT_OUT" \
    STRATUM_ROUTE=/whiteboards "STRATUM_WHITEBOARD=Architecture sketch" \
    STRATUM_WHITEBOARD_TEXT=1

capture_state "$SETTINGS_VAULT_OUT" STRATUM_ROUTE=/settings
capture_state \
    "$SETTINGS_THEME_OUT" \
    STRATUM_ROUTE=/settings STRATUM_SETTINGS_TAB=theme
capture_state \
    "$SETTINGS_AI_OUT" \
    STRATUM_ROUTE=/settings STRATUM_SETTINGS_TAB=ai
capture_state \
    "$SETTINGS_RESEARCH_OUT" \
    STRATUM_ROUTE=/settings STRATUM_SETTINGS_TAB=research
capture_state \
    "$SETTINGS_DEVELOPER_OUT" \
    STRATUM_ROUTE=/settings STRATUM_SETTINGS_TAB=developer STRATUM_SETTINGS_COMMITS=1
capture_state \
    "$SETTINGS_SYNC_OUT" \
    STRATUM_ROUTE=/settings STRATUM_SETTINGS_TAB=sync
capture_state \
    "$SETTINGS_SYNC_CONFLICTS_OUT" \
    STRATUM_ROUTE=/settings STRATUM_SETTINGS_TAB=sync STRATUM_SETTINGS_CONFLICTS=1
capture_state \
    "$SETTINGS_TRAY_OUT" \
    STRATUM_ROUTE=/settings STRATUM_SETTINGS_TAB=tray
capture_state \
    "$SETTINGS_NOTIFICATIONS_OUT" \
    STRATUM_ROUTE=/settings STRATUM_SETTINGS_TAB=notifications

capture_state "$QUICK_CAPTURE_OUT" STRATUM_ROUTE=/journal STRATUM_QUICK_CAPTURE=1
capture_state \
    "$QUICK_CAPTURE_VOICE_OUT" \
    STRATUM_ROUTE=/journal STRATUM_QUICK_CAPTURE_VOICE=1
