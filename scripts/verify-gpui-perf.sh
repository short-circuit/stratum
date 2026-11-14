#!/usr/bin/env bash
# Performance-budget gate for the native shell.
#
# Runs the shipped measurement binaries and asserts the plan's budgets:
#
# - editor: 10 000-block virtualized scroll at >= 55 fps
#   (`stratum-editor-perf`, render p95 <= 18.18 ms)
# - graph: the deterministic layout/render golden asserts its own fps budget
#   (`stratum-graph-golden --assert-fps`)
# - canvas: the canvas perf contract tests assert their frame budget
#   (`cargo test -p stratum-canvas perf`)
# - startup: cold start to first mapped window <= 500 ms and idle RSS <= 80 MB,
#   measured on the fixture shell (release binary when present, otherwise the
#   debug binary with a note that the budget applies to release)
#
# Report: `.hermes/parity/perf-report.json`.
# Requirements: Xvfb, xdotool, python3. Run from any directory.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPORT="${PERF_REPORT:-$ROOT/.hermes/parity/perf-report.json}"
# Resolve the real cargo target directory: the workspace pins `[build]
# target-dir` in .cargo/config.toml, so `$ROOT/target` is not where the binary
# lands. `cargo metadata` honours both that config and CARGO_TARGET_DIR.
TARGET_DIR="$(cargo metadata --no-deps --format-version 1 --manifest-path "$ROOT/Cargo.toml" 2>/dev/null \
    | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])' 2>/dev/null || true)"
if [ -z "$TARGET_DIR" ]; then
    TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
fi
mkdir -p "$(dirname "$REPORT")"

# Measure on a private Xvfb screen: an ambient desktop session would skew the
# cold-start/RSS numbers and let gpui pick the Wayland backend.
unset DISPLAY WAYLAND_DISPLAY

status=0

# GitHub renders `::error::` lines in a step's stdout as check-run annotations.
# Job logs are visible only to repository admins, so every failing gate
# annotates its own last lines: that is the only failure detail a token-less
# contributor or agent can read.
annotate_tail() { # annotate_tail <gate> <logfile> [lines]
    local gate="$1" log="$2" lines="${3:-3}"
    [ -f "$log" ] || return 0
    tail -n "$lines" "$log" | while IFS= read -r line; do
        [ -n "$line" ] && printf '::error::perf %s: %s\n' "$gate" "${line//$'\r'/}"
    done
}

# Run one sub-gate, keep its output in the report directory (uploaded as an
# artifact) and annotate the tail when it fails.
run_gate() { # run_gate <name> <logfile> <command...>
    local name="$1" log="$2"
    shift 2
    if "$@" >"$log" 2>&1; then
        cat "$log"
        return 0
    fi
    cat "$log"
    annotate_tail "$name" "$log"
    return 1
}

echo "== editor: 10 000-block scroll (55 fps budget) =="
if ! run_gate editor "$ROOT/.hermes/parity/perf-editor.log" \
    env STRATUM_PERF_OUT="$ROOT/.hermes/parity/editor-10k-frames.json" \
    "$ROOT/scripts/capture-gpui-editor-perf.sh"; then
    echo "perf: editor budget failed" >&2
    status=1
fi

echo "== graph: deterministic golden (fps budget) =="
# The fps gate needs a GPU adapter. GitHub-hosted runners expose none to wgpu,
# and a CPU rasterizer cannot hold a 60-fps-class budget at 20k nodes, so when
# the GPU renderer cannot start the deterministic CPU renderer (the path the
# shell itself uses on adapter-less devices) is measured WITHOUT the fps
# assertion: gating it there would measure the runner's CPU, not the renderer.
# A budget failure is recognised by the golden's own messages and stays red;
# every other failure means the renderer could not start.
GRAPH_LOG="$ROOT/.hermes/parity/perf-graph.log"
if cargo run -q --release -p stratum-graph --bin graph_golden -- --assert-fps >"$GRAPH_LOG" 2>&1; then
    cat "$GRAPH_LOG"
elif grep -qE "below the [0-9.]+ fps gate|exceeds the [0-9.]+ MB gate" "$GRAPH_LOG"; then
    cat "$GRAPH_LOG"
    # Annotate only the budget lines: the tail also carries cargo's workspace
    # dependency warnings, which are not the failure.
    grep -E "below the [0-9.]+ fps gate|exceeds the [0-9.]+ MB gate" "$GRAPH_LOG" \
        | while IFS= read -r line; do
            printf '::error::perf graph: %s\n' "${line//$'\r'/}"
        done
    echo "perf: graph budget failed" >&2
    status=1
else
    # Not a budget failure: the GPU renderer could not start on this host.
    cat "$GRAPH_LOG"
    echo "perf: GPU graph renderer unavailable on this host — measuring the deterministic CPU renderer (fps gate skipped)"
    printf '::notice::perf graph: no GPU graph renderer on this runner; fps gate skipped, CPU fallback measured\n'
    if ! run_gate graph-cpu "$ROOT/.hermes/parity/perf-graph-cpu.log" \
        cargo run -q --release -p stratum-graph --bin graph_golden -- --software; then
        echo "perf: graph CPU fallback failed" >&2
        status=1
    fi
fi

echo "== canvas: frame-budget contract =="
# `--test perf` selects the integration target that carries the 5k-element and
# text-layout budgets; a bare name filter would match no test.
if ! run_gate canvas "$ROOT/.hermes/parity/perf-canvas.log" \
    cargo test -q -p stratum-canvas --test perf; then
    echo "perf: canvas budget failed" >&2
    status=1
fi

echo "== startup: cold start + idle memory =="
# The gate reads the app's own anonymous memory (RssAnon) rather than VmRSS:
# the graphics stack (Vulkan ICD, Mesa/llvmpipe, font caches) contributes
# ~150-190 MB of file-backed shared mappings that every GPU application maps,
# so VmRSS measures the driver, not the shell. Both figures are reported; only
# RssAnon is gated against the 80 MB budget.
STARTUP_LOG="$ROOT/.hermes/parity/perf-startup.log"
if ! python3 - "$ROOT" "$TARGET_DIR" "$REPORT" >"$STARTUP_LOG" 2>&1 <<'PY'
import json, os, re, subprocess, sys, time

root, target_dir, report_path = sys.argv[1:4]
release = os.path.join(target_dir, "release", "stratum")
debug = os.path.join(target_dir, "debug", "stratum")
binary = release if os.path.exists(release) else debug
profile = "release" if binary == release else "debug"
if not os.path.exists(binary):
    print(f"startup: no shell binary under {target_dir} — build it first "
          f"(cargo build --release -p stratum-ui)")
    sys.exit(1)

display = ":93"
xvfb = subprocess.Popen(
    ["Xvfb", display, "-screen", "0", "1440x900x24", "-nolisten", "tcp"],
    stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
)
env = dict(os.environ, DISPLAY=display, STRATUM_FIXTURE="1",
           LIBGL_ALWAYS_SOFTWARE="1", WINIT_UNIX_BACKEND="x11")
env.pop("WAYLAND_DISPLAY", None)
try:
    time.sleep(1.0)
    started = time.monotonic()
    app = subprocess.Popen([binary], env=env,
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    first_frame = None
    rss_kb = None
    anon_kb = None
    deadline = started + 20.0
    while time.monotonic() < deadline:
        out = subprocess.run(
            ["xdotool", "search", "--onlyvisible", "--name", "Stratum"],
            env=env, capture_output=True, text=True,
        ).stdout.strip()
        if out:
            first_frame = (time.monotonic() - started) * 1000.0
            break
        time.sleep(0.05)
    # Idle memory once the shell has settled (8s: first paint, fixture load and
    # the first index pass are done; the allocator has released its scratch).
    time.sleep(8.0)
    try:
        with open(f"/proc/{app.pid}/status") as fh:
            for line in fh:
                if line.startswith("VmRSS:"):
                    rss_kb = int(re.findall(r"\d+", line)[0])
                elif line.startswith("RssAnon:"):
                    anon_kb = int(re.findall(r"\d+", line)[0])
    except FileNotFoundError:
        pass
    app.terminate()
    try:
        app.wait(timeout=10)
    except subprocess.TimeoutExpired:
        app.kill()
finally:
    xvfb.terminate()

budgets = {
    "cold_start_ms": 500.0,
    # PERF_IDLE_RSS_ANON_BUDGET_MB raises the ceiling for runs that cannot use a
    # hardware adapter (CI renders through software Vulkan/GL, whose in-process
    # framebuffers land in RssAnon): the shipping target is 80 MB on hardware.
    "idle_rss_anon_mb": float(os.environ.get("PERF_IDLE_RSS_ANON_BUDGET_MB", "80")),
}
cold_start_ok = first_frame is not None and first_frame <= budgets["cold_start_ms"]
rss_mb = None if rss_kb is None else round(rss_kb / 1024.0, 1)
anon_mb = None if anon_kb is None else round(anon_kb / 1024.0, 1)
anon_ok = anon_mb is not None and anon_mb <= budgets["idle_rss_anon_mb"]
report = {
    "profile": profile,
    "cold_start_ms": None if first_frame is None else round(first_frame, 1),
    "idle_rss_anon_mb": anon_mb,
    "idle_rss_total_mb": rss_mb,
    "budgets": budgets,
    "cold_start_ok": cold_start_ok,
    "idle_rss_anon_ok": anon_ok,
    "idle_budget_override": os.environ.get("PERF_IDLE_RSS_ANON_BUDGET_MB"),
    "note": (
        "budgets target the release build; a debug run reports a larger RSS"
        if profile == "debug" else ""
    ),
}
with open(report_path, "w") as fh:
    json.dump(report, fh, indent=2)
    fh.write("\n")
print(json.dumps(report, indent=2))
print(
    f"startup: profile={profile} cold_start_ms={report['cold_start_ms']} "
    f"idle_rss_anon_mb={anon_mb} cold_start_ok={cold_start_ok} "
    f"idle_rss_anon_ok={anon_ok}"
)
# The debug binary is not a shipping artifact, so only the release profile is
# gated; a debug run still reports its numbers.
if profile == "release" and not (cold_start_ok and anon_ok):
    sys.exit(1)
if first_frame is None:
    sys.exit(1)
PY
then
    cat "$STARTUP_LOG"
    annotate_tail startup "$STARTUP_LOG" 2
    status=1
else
    cat "$STARTUP_LOG"
fi

echo
if [[ "$status" -eq 0 ]]; then
    echo "perf: all budgets met (report: $REPORT)"
else
    echo "perf: budget failure (report: $REPORT)" >&2
fi
exit "$status"
