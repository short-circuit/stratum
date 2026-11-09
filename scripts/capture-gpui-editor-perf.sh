#!/usr/bin/env bash
# Capture the 10 000-block editor performance report.
#
# Builds and runs `stratum-editor-perf`, which opens the native editor on the
# deterministic 10 000-block fixture, sweeps the virtual list for
# `STRATUM_PERF_FRAMES` frames (default 600), writes the frame and row-build
# timings as JSON and quits. When no DISPLAY is set the script starts a private
# Xvfb screen; the binary needs a window because gpui only paints on a real
# surface.
#
# The JSON is written to STRATUM_PERF_OUT (default
# `.hermes/perf/editor-10k-frames.json`). The script prints the measured p50/p95
# frame times, the derived fps and the editor row-materialization p95. It exits
# non-zero only when `render_p95_ms` exceeds the 55 fps budget (18.18 ms); frame
# pacing under software rendering is reported but not gated.
#
# The gate runs the release binary: a debug build spends an order of magnitude
# more time building row elements, so a debug `render_p95_ms` says nothing about
# the 55 fps target. Set STRATUM_PERF_PROFILE=debug to reproduce a debug run.
#
# Requirements: Xvfb when no DISPLAY is set, python3 for JSON parsing.
# Run from any directory; the script locates the workspace itself.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="${STRATUM_PERF_OUT:-$ROOT/.hermes/perf/editor-10k-frames.json}"
FRAMES="${STRATUM_PERF_FRAMES:-600}"
WIDTH="${CAPTURE_WIDTH:-1280}"
HEIGHT="${CAPTURE_HEIGHT:-720}"
DISPLAY_NUM="${CAPTURE_DISPLAY:-98}"
LOG="${PERF_LOG:-$ROOT/.hermes/perf/editor-10k.log}"
PROFILE="${STRATUM_PERF_PROFILE:-release}"
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
BUDGET_MS="18.18"

if [[ "$PROFILE" == "release" ]]; then
    BUILD_ARGS=(--release)
    BIN="$TARGET_DIR/release/stratum-editor-perf"
else
    BUILD_ARGS=()
    BIN="$TARGET_DIR/debug/stratum-editor-perf"
fi

mkdir -p "$(dirname "$OUT")" "$(dirname "$LOG")"

export STRATUM_PERF_OUT="$OUT"
export STRATUM_PERF_FRAMES="$FRAMES"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
export LIBGL_ALWAYS_SOFTWARE=1
export WINIT_UNIX_BACKEND=x11
export RUST_BACKTRACE=1

if ! command -v python3 >/dev/null 2>&1; then
    echo "capture-gpui-editor-perf: missing required tool: python3" >&2
    exit 1
fi

XVFB_PID=""
cleanup() {
    if [[ -n "$XVFB_PID" ]]; then
        kill "$XVFB_PID" 2>/dev/null || true
        wait "$XVFB_PID" 2>/dev/null || true
    fi
}
trap cleanup EXIT

# Build first so the timed run is not spent compiling.
cargo build -p stratum-ui --bin stratum-editor-perf --manifest-path "$ROOT/Cargo.toml" "${BUILD_ARGS[@]}"

if [[ -z "${DISPLAY:-}" ]]; then
    if ! command -v Xvfb >/dev/null 2>&1; then
        echo "capture-gpui-editor-perf: no DISPLAY and Xvfb is not installed" >&2
        exit 1
    fi
    Xvfb ":$DISPLAY_NUM" -screen "0" "${WIDTH}x${HEIGHT}x24" -nolisten tcp \
        >/dev/null 2>&1 &
    XVFB_PID=$!
    export DISPLAY=":$DISPLAY_NUM"
    # Give Xvfb a moment to create its socket before the client connects.
    for _ in $(seq 1 40); do
        if [[ -S "/tmp/.X11-unix/X$DISPLAY_NUM" ]]; then
            break
        fi
        sleep 0.25
    done
fi

# A stale report from a previous run must not satisfy this run's gate.
rm -f "$OUT"

if command -v timeout >/dev/null 2>&1; then
    # Software rendering under Xvfb paces frames far below 55 fps, so allow the
    # full default 600-frame sweep to finish.
    timeout 2400 "$BIN" >"$LOG" 2>&1 || {
        status=$?
        echo "capture-gpui-editor-perf: editor perf run failed (exit $status); see $LOG" >&2
        cat "$LOG" >&2 || true
        exit 1
    }
else
    "$BIN" >"$LOG" 2>&1 || {
        status=$?
        echo "capture-gpui-editor-perf: editor perf run failed (exit $status); see $LOG" >&2
        cat "$LOG" >&2 || true
        exit 1
    }
fi

if [[ ! -s "$OUT" ]]; then
    echo "capture-gpui-editor-perf: report was not written to $OUT; see $LOG" >&2
    cat "$LOG" >&2 || true
    exit 1
fi

python3 - "$OUT" "$BUDGET_MS" <<'PY'
import json
import sys

path, budget = sys.argv[1], float(sys.argv[2])
with open(path, encoding="utf-8") as handle:
    report = json.load(handle)

frames = int(report.get("frames", 0))
blocks = int(report.get("blocks", 0))
rows = int(report.get("rows_rendered", 0))
render_p95 = float(report.get("render_p95_ms", 0.0))
fps_p50 = float(report.get("fps_p50", 0.0))
fps_p95 = float(report.get("fps_p95", 0.0))

print(
    f"editor-perf: {frames} frames, blocks={blocks}, rows_rendered={rows}, "
    f"render_p95={render_p95:.2f}ms (budget {budget:.2f}ms / 55 fps)"
)
print(
    f"editor-perf: frame p50={float(report.get('p50_ms', 0.0)):.2f}ms "
    f"({fps_p50:.1f} fps), p95={float(report.get('p95_ms', 0.0)):.2f}ms "
    f"({fps_p95:.1f} fps), p99={float(report.get('p99_ms', 0.0)):.2f}ms, "
    f"max={float(report.get('max_ms', 0.0)):.2f}ms"
)
print(f"editor-perf: report {path}")

if frames == 0:
    print("editor-perf: no frames were recorded", file=sys.stderr)
    sys.exit(1)
if blocks != 10_000:
    print(
        f"editor-perf: expected 10000 blocks, captured {blocks}",
        file=sys.stderr,
    )
    sys.exit(1)
if render_p95 > budget:
    print(
        f"editor-perf: render p95 {render_p95:.2f}ms exceeds the {budget:.2f}ms budget",
        file=sys.stderr,
    )
    sys.exit(1)
PY
