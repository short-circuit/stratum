#!/usr/bin/env bash
# Run a CI gate command and surface its failure detail as GitHub Actions
# annotations.
#
# Why: GitHub only shows job logs to users with admin access to the repository,
# so a red run on a public repo is otherwise opaque — the check-run annotations
# are all an outside contributor (or an agent without a token) sees. This
# wrapper runs the gate, forwards its output to the job log unchanged, and on a
# non-zero exit re-emits the actionable lines and the log tail as `::error::`
# workflow commands, which GitHub turns into annotations.
#
# It never changes the gate's verdict: the exit status is passed through.
# Annotations are capped at seven (GitHub drops everything past ten per step),
# so the failing lines come first and the tail is collapsed into one message.
#
# Usage: scripts/ci-annotate.sh <command> [args...]
# Example: bash scripts/ci-annotate.sh cargo test --workspace
# Requirements: bash (git-bash on the Windows runners). Run from any directory.
set -uo pipefail

if [[ $# -eq 0 ]]; then
    echo "usage: ci-annotate.sh <command> [args...]" >&2
    exit 2
fi

log="$(mktemp "${TMPDIR:-/tmp}/ci-annotate.XXXXXX")"
trap 'rm -f "$log"' EXIT

"$@" >"$log" 2>&1
status=$?
cat "$log"

if [[ "$status" -ne 0 ]]; then
    echo "::group::gate failure detail"
    # Rust diagnostics, cargo test failure sections, perf/golden/a11y summaries,
    # panics and the perf harness's own report lines carry the actionable
    # detail. `\r` is stripped because the Windows runners emit CRLF and GitHub
    # rejects the trailing carriage return in annotations.
    grep -nE '^(error|failures:|    [A-Za-z_][A-Za-z0-9_:]*|perf: |goldens: |a11y: |fixture |thread .* panicked)' "$log" |
        tail -n 6 |
        while IFS= read -r line; do
            printf '::error::%s\n' "${line//$'\r'/}"
        done
    # Always include the tail: budget numbers and summary lines do not match
    # the patterns above.
    tail_summary="$(tail -n 10 "$log" | tr '\n' '|' | tr -d '\r')"
    printf '::error::gate tail: %s\n' "$tail_summary"
    echo "::endgroup::"
fi

exit "$status"
