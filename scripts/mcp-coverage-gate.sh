#!/usr/bin/env bash
# CI coverage gate for the MCP server crate.
#
# Builds and runs the pkm-mcp test suites under `-Cinstrument-coverage` and
# asserts that the MCP crate's line coverage is at or above the configured
# threshold. Uses the rustup llvm-tools (no extra crates to install) so the gate
# is dependency-free; python3 is only used to read llvm-cov's JSON export.
#
# Every failure mode is reported as a `::error::` workflow annotation, because
# Actions job logs are not downloadable without a token while check-run
# annotations are: a red run has to explain itself through the annotations
# alone. The instrumented build output is kept (not swallowed) for the same
# reason — a silent no-op build used to look exactly like a coverage shortfall.
#
# Usage: bash scripts/mcp-coverage-gate.sh [threshold_percent] [target_dir]
# Env:   MCP_COV_REUSE=1  skip the build and reuse an existing instrumented
#                        build in <target_dir> (local debugging only). The
#                        tree must match that build: profiles left over from a
#                        different tree report ~0%, not the real coverage.
set -euo pipefail

THRESHOLD="${1:-80}"
COV_TARGET="${2:-target/cov}"
REUSE="${MCP_COV_REUSE:-0}"
cd "$(dirname "$0")/.."

# --- failure reporting ------------------------------------------------------
# GitHub workflow commands must stay on one line, so multi-line detail is
# joined with '|' and truncated.
err() { echo "::error::$*"; }
oneline() { tr '\n' '|' | cut -c1-1500; }
# Compact description of what the target dir actually holds: the directory tree
# is what identifies a layout drift, and the profile count identifies a build
# that never wrote coverage. Listing every file would bury both in the
# annotation.
layout_summary() {
  local profiles=0
  shopt -s nullglob
  local files=("$COV_TARGET"/mcp-*.profraw)
  shopt -u nullglob
  profiles=${#files[@]}
  echo "dirs=$(find "$COV_TARGET" -maxdepth 3 -type d -not -path '*/.fingerprint*' 2>/dev/null | oneline) profiles=$profiles"
}
# `set -e` exits are silent in the annotation feed otherwise.
trap 'err "MCP coverage gate failed at line $LINENO"' ERR

# --- environment ------------------------------------------------------------
export CARGO_TARGET_DIR="$COV_TARGET"
export RUSTFLAGS="-Cinstrument-coverage"
export LLVM_PROFILE_FILE="$PWD/$COV_TARGET/mcp-%p-%m.profraw"

# Printed unconditionally: the next red run has to show which toolchain and
# which target layout it used, without needing the job log.
CARGO_ENV=$(env | grep -E '^CARGO_(TARGET_DIR|BUILD_TARGET|PROFILE|INCREMENTAL)' | sort | oneline || true)
echo "== MCP coverage gate =="
echo "threshold:           ${THRESHOLD}%"
echo "coverage target dir: $COV_TARGET"
echo "working directory:   $PWD"
echo "MCP_COV_REUSE:       $REUSE"
echo "rustc:               $(rustc -vV | oneline)"
echo "cargo:               $(cargo -V | oneline)"
echo "CARGO_* environment: ${CARGO_ENV:-<none>}"

if ! [[ "$THRESHOLD" =~ ^[0-9]+([.][0-9]+)?$ ]]; then
  err "threshold is not a number: '$THRESHOLD'"
  exit 2
fi
if ! command -v python3 >/dev/null 2>&1; then
  err "python3 is required to read the llvm-cov JSON export but was not found"
  exit 2
fi

LLVM_BIN="$(rustc --print sysroot)/lib/rustlib/$(rustc -vV | sed -n 's/^host: //p')/bin"
COV="$LLVM_BIN/llvm-cov"
PROFDATA="$LLVM_BIN/llvm-profdata"
for tool in "$COV" "$PROFDATA"; do
  if [ ! -x "$tool" ]; then
    err "missing $tool — install the llvm-tools-preview rustup component"
    exit 2
  fi
done

# --- build + run the instrumented suites ------------------------------------
BUILD_LOG="$COV_TARGET/mcp-cargo-test.log"
mkdir -p "$COV_TARGET"

if [ "$REUSE" = "1" ]; then
  echo "== reusing the existing instrumented build (MCP_COV_REUSE=1) =="
else
  # Stale profiles from an earlier run in the same target dir would be merged
  # into this run's report and silently inflate the result.
  rm -f "$COV_TARGET"/mcp-*.profraw "$COV_TARGET/mcp.profdata" "$COV_TARGET/mcp-coverage.json"
  echo "== cargo test -p pkm-mcp --all-targets (instrumented) =="
  status=0
  cargo test -p pkm-mcp --all-targets 2>&1 | tee "$BUILD_LOG" || status=$?
  if [ "$status" -ne 0 ]; then
    err "cargo test -p pkm-mcp --all-targets failed (exit $status); last lines: $(tail -5 "$BUILD_LOG" | oneline)"
    exit 1
  fi
fi

# --- merge the profiles -----------------------------------------------------
shopt -s nullglob
profiles=("$COV_TARGET"/mcp-*.profraw)
shopt -u nullglob
if [ "${#profiles[@]}" -eq 0 ]; then
  err "no profile files ($COV_TARGET/mcp-*.profraw): cargo test ran but no instrumented binary wrote coverage; $(layout_summary)"
  exit 1
fi
echo "== merging ${#profiles[@]} profile file(s) =="
status=0
"$PROFDATA" merge -sparse "${profiles[@]}" -o "$COV_TARGET/mcp.profdata" || status=$?
if [ "$status" -ne 0 ]; then
  err "llvm-profdata merge failed (exit $status) for ${#profiles[@]} profile file(s)"
  exit 1
fi

# --- map the instrumented binaries -----------------------------------------
# The binaries used to be searched for in "$COV_TARGET/debug/deps" only, which
# breaks as soon as the artifacts land anywhere else (a `--target`/cross build
# puts them in "$COV_TARGET/<triple>/debug/deps", and the runner's target layout
# is not ours to assume). Ask cargo which executables this build produced
# instead: that is layout-independent and only returns fresh binaries, so no
# stale object can skew the report.
MSGS="$COV_TARGET/mcp-cargo-messages.jsonl"
status=0
cargo test -p pkm-mcp --all-targets --no-run --message-format=json > "$MSGS" 2>"$COV_TARGET/mcp-cargo-no-run.log" || status=$?
if [ "$status" -ne 0 ]; then
  err "cargo test --no-run --message-format=json failed (exit $status); last lines: $(tail -5 "$COV_TARGET/mcp-cargo-no-run.log" | oneline)"
  exit 1
fi

BIN_LIST=$(python3 - "$MSGS" <<'PY'
import json
import os
import re
import sys

# Same binary selection as before: the pkm-mcp and pkm-mcp-security executables.
wanted = re.compile(r"^(pkm_mcp|pkm_mcp_security)-[0-9a-f]+$")
seen = set()
for line in open(sys.argv[1], encoding="utf-8", errors="replace"):
    try:
        message = json.loads(line)
    except json.JSONDecodeError:
        continue
    if message.get("reason") != "compiler-artifact":
        continue
    executable = message.get("executable")
    if not executable:
        continue
    if not wanted.match(os.path.basename(executable)):
        continue
    if executable not in seen:
        seen.add(executable)
        print(executable)
PY
)
if [ -z "$BIN_LIST" ]; then
  err "cargo built no pkm-mcp/pkm-mcp-security executable under $COV_TARGET; $(layout_summary)"
  exit 2
fi
echo "== instrumented binaries =="
echo "$BIN_LIST"

ARGS=()
while IFS= read -r binary; do
  ARGS+=(--object "$binary")
done <<< "$BIN_LIST"

# --- report -----------------------------------------------------------------
# Parsed from `llvm-cov export --summary-only` (JSON) rather than from the
# column positions of the printed report, whose field order follows the LLVM
# version rather than the tool contract.
COV_JSON="$COV_TARGET/mcp-coverage.json"
status=0
"$COV" export --summary-only --instr-profile "$COV_TARGET/mcp.profdata" "${ARGS[@]}" \
  --ignore-filename-regex='tests/|examples/|/registry/|/rustc/|/.cargo/|/pkm-core/|/pkm-block/|/pkm-index/|/pkm-markdown/|/pkm-query/' \
  > "$COV_JSON" || status=$?
if [ "$status" -ne 0 ]; then
  err "llvm-cov export failed (exit $status) for ${#ARGS[@]} arguments ($(echo "$BIN_LIST" | oneline))"
  exit 1
fi

REPORT=$(python3 - "$COV_JSON" <<'PY'
import json
import sys

data = json.load(open(sys.argv[1], encoding="utf-8"))
totals = data["data"][0]["totals"]["lines"]
print("== MCP coverage report (line %) ==")
for entry in sorted(data["data"][0].get("files", []), key=lambda f: f["summary"]["lines"]["percent"]):
    name = entry["filename"]
    if "/pkm-mcp/src" not in name and "/pkm-mcp-security/src" not in name:
        continue
    lines = entry["summary"]["lines"]
    print(f'{lines["percent"]:7.2f}%  {lines["covered"]:>5}/{lines["count"]:<5}  {name}')
print(f'{totals["percent"]:7.2f}%  {totals["covered"]:>5}/{totals["count"]:<5}  TOTAL')
print(f"COVERED_LINES={totals['covered']} TOTAL_LINES={totals['count']} LINE_PCT={totals['percent']:.2f}")
PY
)
echo "$REPORT" | grep -v '^COVERED_LINES=' || true
SUMMARY=$(echo "$REPORT" | grep '^COVERED_LINES=' || true)
LINE_PCT=$(echo "$SUMMARY" | sed -n 's/.*LINE_PCT=//p')

echo ""
echo "MCP line coverage: ${LINE_PCT:-0}% (threshold: ${THRESHOLD}%)"

if [ -z "$LINE_PCT" ] || ! echo "$LINE_PCT" | grep -qE '^[0-9]+(\.[0-9]+)?$'; then
  err "could not parse a numeric line-coverage percentage from $COV_JSON"
  exit 1
fi
if awk "BEGIN { exit !($LINE_PCT >= $THRESHOLD) }"; then
  echo "PASS: MCP line coverage >= ${THRESHOLD}%"
  exit 0
else
  err "MCP line coverage $LINE_PCT% is below the ${THRESHOLD}% gate"
  exit 1
fi
