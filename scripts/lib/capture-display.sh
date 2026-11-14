#!/usr/bin/env bash
# Shared helpers for the headless gpui capture scripts.
#
# The capture harnesses must own an exact-size X screen: an ambient desktop
# display, or a stale Xvfb left behind by an interrupted run, would otherwise
# decide the PNG geometry and silently produce a blank or wrong-sized capture
# that the golden gate reports as "drift". These helpers acquire a private
# screen that is verified to match the requested geometry, and refuse a capture
# that rendered nothing.
#
# Usage (from a capture script, after `set -euo pipefail`):
#
#     source "$ROOT/scripts/lib/capture-display.sh"
#     acquire_capture_display "$WIDTH" "$HEIGHT" "$DISPLAY_NUM" || exit 1
#     ...
#     assert_capture_not_blank "$out" || exit 1
#
# `acquire_capture_display` exports DISPLAY and sets XVFB_PID for the caller's
# cleanup trap; it never touches an X server it did not start.

# acquire_capture_display WIDTH HEIGHT PREFERRED_DISPLAY
#
# Starts a private Xvfb screen of WIDTHxHEIGHT on PREFERRED_DISPLAY when free,
# otherwise on the first free display in 90..110, and verifies the geometry
# before returning. Returns non-zero when no display could be acquired.
acquire_capture_display() {
    local width="$1" height="$2" preferred="$3"
    local candidate geometry pid
    local -a candidates=("$preferred")

    for candidate in $(seq 90 110); do
        if [[ "$candidate" != "$preferred" ]]; then
            candidates+=("$candidate");
        fi
    done

    for candidate in "${candidates[@]}"; do
        # Probe the server rather than /tmp/.X<n>-lock: an interrupted Xvfb
        # leaves a lock behind, and a display number can be taken by a server
        # this script must not disturb.
        if DISPLAY=":$candidate" xdotool getdisplaygeometry >/dev/null 2>&1; then
            continue
        fi
        Xvfb ":$candidate" -screen "0" "${width}x${height}x24" -nolisten tcp \
            >/dev/null 2>&1 &
        pid=$!
        export DISPLAY=":$candidate"
        for _ in $(seq 1 40); do
            if xdotool getdisplaygeometry >/dev/null 2>&1; then
                break
            fi
            sleep 0.25
        done
        geometry="$(xdotool getdisplaygeometry 2>/dev/null || true)"
        if [[ "$geometry" == "$width $height" ]]; then
            XVFB_PID="$pid"
            return 0
        fi
        # Xvfb lost the race (or started at the wrong size): drop it and try
        # the next candidate instead of capturing onto a foreign screen.
        kill "$pid" 2>/dev/null || true
        wait "$pid" 2>/dev/null || true
        unset DISPLAY
    done

    echo "capture: no free X display with ${width}x${height} geometry" >&2
    return 1
}

# assert_capture_not_blank FILE
#
# Fails when a capture is a flat image (fewer than 8 unique colors), which is
# what an unpainted root window looks like. Without this check a blank capture
# reaches the golden diff as a large RMSE instead of a clear harness error.
assert_capture_not_blank() {
    local out="$1" colors
    colors="$(identify -format '%k' "$out" 2>/dev/null || echo 0)"
    if [[ "${colors:-0}" -lt 8 ]]; then
        echo "capture: $out looks blank (${colors:-0} unique colors)" >&2
        return 1
    fi
    return 0
}
