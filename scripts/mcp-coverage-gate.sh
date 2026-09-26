#!/usr/bin/env bash
# CI coverage gate for the MCP server crate.
#
# Builds and runs the pkm-mcp (+ pkm-mcp-security) test suites under
# `-Cinstrument-coverage` and asserts that the MCP crate's line coverage is
# at or above the configured threshold. Uses the rustup llvm-tools (no extra
# crates to install) so the gate is dependency-free.
#
# Usage: bash scripts/mcp-coverage-gate.sh [threshold_percent] [target_dir]
set -euo pipefail

THRESHOLD="${1:-80}"
COV_TARGET="${2:-target/cov}"
cd "$(dirname "$0")/.."

export CARGO_TARGET_DIR="$COV_TARGET"
export RUSTFLAGS="-Cinstrument-coverage"
export LLVM_PROFILE_FILE="$PWD/$COV_TARGET/mcp-%p-%m.profraw"

LLVM_BIN="$(rustc --print sysroot)/lib/rustlib/$(rustc -vV | sed -n 's/^host: //p')/bin"
COV="$LLVM_BIN/llvm-cov"
PROFDATA="$LLVM_BIN/llvm-profdata"

# Build + run the MCP crate test suites under instrumentation.
cargo test -p pkm-mcp --all-targets >/dev/null 2>&1

# Merge profiles.
"$PROFDATA" merge -sparse "$COV_TARGET"/mcp-*.profraw -o "$COV_TARGET/mcp.profdata"

# Map instrumented binaries (the lib/test/example binaries for pkm-mcp and
# pkm-mcp-security).
BINS=$(find "$COV_TARGET/debug/deps" -maxdepth 1 -type f -executable \
  \( -name 'pkm_mcp-*' -o -name 'pkm_mcp_security-*' \) \
  | tr '\n' ' ')
if [ -z "$BINS" ]; then
  echo "ERROR: no pkm-mcp instrumented binaries found" >&2
  exit 2
fi

ARGS=""
for b in $BINS; do ARGS="$ARGS --object $b"; done

echo "== MCP coverage report (line %) =="
# Restrict the report to the MCP crates only (both pkm-mcp and
# pkm-mcp-security); ignore the huge dependency footprint.
REPORT=$("$COV" report --instr-profile "$COV_TARGET/mcp.profdata" $ARGS \
  --ignore-filename-regex='tests/|examples/|/registry/|/rustc/|/.cargo/|/pkm-core/|/pkm-block/|/pkm-index/|/pkm-markdown/|/pkm-query/')
echo "$REPORT" | grep -E "pkm-mcp/src|pkm-mcp-security/src|TOTAL" | head -40

# Parse the TOTAL line: it ends with region/line summary metrics in the
# printed order
#   Filename Regions Missed.Cover Functions Missed.Funcs Executed Lines Missed.Lines Cover
# so line coverage (percent) is the 10th whitespace-separated field.
TOTAL_LINE=$(echo "$REPORT" | tail -1)
LINE_PCT=$(echo "$TOTAL_LINE" | awk '{print $10}' | tr -d '%')
echo ""
echo "MCP line coverage: ${LINE_PCT:-0}% (threshold: ${THRESHOLD}%)"

# awk returns 0 for the comparison; guard against an empty/unparsed value.
if [ -z "$LINE_PCT" ] || ! echo "$LINE_PCT" | grep -qE '^[0-9]+(\.[0-9]+)?$'; then
  echo "FAIL: could not parse a numeric line-coverage percentage from:" >&2
  echo "$TOTAL_LINE" >&2
  exit 1
fi
if awk "BEGIN { exit !($LINE_PCT >= $THRESHOLD) }"; then
  echo "PASS: MCP line coverage >= ${THRESHOLD}%"
  exit 0
else
  echo "FAIL: MCP line coverage $LINE_PCT% is below the ${THRESHOLD}% gate" >&2
  exit 1
fi
