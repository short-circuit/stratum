#!/usr/bin/env bash
# Run a CI gate command and surface its failure detail as GitHub Actions
# annotations.
#
# Why: GitHub only shows job logs to users with admin access to the repository,
# so a red run on a public repo is otherwise opaque — the check-run annotations
# are all an outside contributor (or an agent without a token) sees. This
# wrapper runs the gate, forwards its output to the job log unchanged, and on a
# non-zero exit re-emits the failure lines and the log tail as `::error::`
# workflow commands, which GitHub turns into annotations.
#
# It never changes the gate's verdict: the exit status is passed through.
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
    # Rust diagnostics, cargo test failure sections, gate summaries and panics
    # carry the actionable lines. `\r` is stripped because the Windows runners
    # emit CRLF and GitHub rejects the trailing carriage return in annotations.
    grep -nE '^(error|failures:|    [A-Za-z_][A-Za-z0-9_:]*|perf: |goldens: |a11y: |thread .* panicked)' "$log" |
        tail -n 40 |
        while IFS= read -r line; do
            printf '::error::%s\n' "${line//$'\r'/}"
        done
    # Always include the tail: budget numbers and summary lines do not match
    # the patterns above.
    tail -n 15 "$log" |
        while IFS= read -r line; do
            printf '::error::%s\n' "${line//$'\r'/}"
        done
    echo "::endgroup::"
fi

exit "$status"
