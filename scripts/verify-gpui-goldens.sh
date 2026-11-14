#!/usr/bin/env bash
# Golden-screenshot parity gate for the native shell.
#
# Captures every shipped surface at the plan's reference sizes — desktop
# 1440x900 (shell expanded/collapsed plus the panel matrix) and mobile
# 1080x2400 — and diffs each capture against the committed golden in
# `goldens/gpui/`. A capture fails when its normalized RMSE exceeds
# `GOLDEN_TOLERANCE` (default 0.02, i.e. 2%), which is the font/anti-aliasing
# drift budget from the parity plan; structural changes (a moved column, a
# missing label) blow past it.
#
# `--update` regenerates the golden set from the current build (review the
# resulting image diff before committing it).
#
# Report: `.hermes/parity/goldens-report.json`.
# Requirements: Xvfb, xdotool, ImageMagick. Run from any directory.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GOLDENS="$ROOT/goldens/gpui"
WORK="${GOLDEN_WORK:-$ROOT/.hermes/parity/captures}"
REPORT="${GOLDEN_REPORT:-$ROOT/.hermes/parity/goldens-report.json}"
TOLERANCE="${GOLDEN_TOLERANCE:-0.02}"
UPDATE=0
for arg in "$@"; do
    if [[ "$arg" == "--update" ]]; then UPDATE=1; fi
done

mkdir -p "$WORK" "$GOLDENS" "$(dirname "$REPORT")"

# Every capture must land on a private Xvfb screen at the plan's reference size.
# An ambient DISPLAY (a developer's desktop session, which may also advertise
# Wayland) would otherwise be captured at whatever size that session has, and
# gpui would pick the Wayland backend where xdotool cannot see the window.
unset DISPLAY WAYLAND_DISPLAY

echo "== capturing desktop surfaces at 1440x900 =="
CAPTURE_WIDTH=1440 CAPTURE_HEIGHT=900 CAPTURE_DISPLAY="${CAPTURE_DISPLAY:-95}" \
    "$ROOT/scripts/capture-gpui-shell.sh" "$WORK/gpui-shell.png"
CAPTURE_WIDTH=1440 CAPTURE_HEIGHT=900 CAPTURE_DISPLAY="${CAPTURE_DISPLAY:-95}" \
    "$ROOT/scripts/capture-gpui-panels.sh" "$WORK/gpui-journal.png"

echo "== capturing mobile surfaces at 1080x2400 =="
CAPTURE_WIDTH=1080 CAPTURE_HEIGHT=2400 CAPTURE_DISPLAY="${CAPTURE_DISPLAY:-94}" \
    "$ROOT/scripts/capture-gpui-mobile.sh" "$WORK/gpui-mobile-journal.png"

python3 - "$WORK" "$GOLDENS" "$REPORT" "$TOLERANCE" "$UPDATE" <<'PY'
import json, os, subprocess, sys

work, goldens, report_path, tolerance, update = sys.argv[1:6]
tolerance = float(tolerance)
update = update == "1"

captures = sorted(
    name for name in os.listdir(work)
    if name.endswith(".png") and os.path.getsize(os.path.join(work, name)) > 0
)
if not captures:
    print("goldens: no captures produced", file=sys.stderr)
    sys.exit(1)

rows = []
failed = 0
for name in captures:
    capture = os.path.join(work, name)
    golden = os.path.join(goldens, name)
    if update:
        subprocess.run(["cp", capture, golden], check=True)
        rows.append({"surface": name, "status": "updated", "rmse": 0.0})
        continue
    if not os.path.exists(golden):
        rows.append({"surface": name, "status": "missing_golden", "rmse": None})
        failed += 1
        continue
    # Normalized RMSE: 0 = identical, 1 = maximally different. ImageMagick
    # prints "absolute (normalized)" — the absolute value is in the quantum
    # range (65535 on Q16 builds), so the tolerance must be applied to the
    # normalized figure in parentheses.
    result = subprocess.run(
        ["compare", "-metric", "RMSE", capture, golden, "null:"],
        capture_output=True, text=True,
    )
    stderr = (result.stderr or result.stdout).strip()
    try:
        if "(" in stderr and ")" in stderr:
            value = float(stderr.split("(", 1)[1].split(")", 1)[0])
        else:
            # Some builds omit the normalized figure; scale the absolute value.
            value = float(stderr.split()[0]) / 65535.0
    except (IndexError, ValueError):
        rows.append({"surface": name, "status": "error", "rmse": None, "detail": stderr})
        failed += 1
        continue
    ok = value <= tolerance
    if not ok:
        failed += 1
    rows.append({
        "surface": name,
        "status": "match" if ok else "drift",
        "rmse": round(value, 6),
        "tolerance": tolerance,
    })

report = {
    "surfaces": len(rows),
    "failed": failed,
    "tolerance": tolerance,
    "mode": "update" if update else "verify",
    "rows": rows,
}
with open(report_path, "w") as fh:
    json.dump(report, fh, indent=2)
    fh.write("\n")

for row in rows:
    rmse = "-" if row["rmse"] is None else f"{row['rmse']:.4f}"
    print(f"{row['status']:14s} {row['surface']:40s} rmse={rmse}")
print(f"\ngoldens: {len(rows)} surfaces, {failed} failed (tolerance {tolerance}), report {report_path}")
sys.exit(1 if failed else 0)
PY
