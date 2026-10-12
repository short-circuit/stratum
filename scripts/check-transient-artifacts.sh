#!/usr/bin/env bash
# check-transient-artifacts.sh
#
# Stratum repo hygiene gate: fails if any transient agent artifact is
# tracked. Run in CI (ci.yml) and locally. A tracked transients file
# (plan/QA report/finding/audit/repro/evidence/screenshot/worktree) means
# an agent wrote it outside .hermes/ and it got committed — reject.
#
# Exit 0 = clean. Exit 1 = offenders found (list them).
set -u

cd "$(dirname "$0")/.." || exit 2

# Anything tracked that matches a transient pattern. git ls-files yields
# repo-root-relative paths WITHOUT a leading slash, so patterns that target
# the repo root use ^ (not ^/). Anchored to avoid matching src files that
# merely mention these names.
offenders=$(git ls-files | grep -E \
  '(^|/)\.evidence/|^\.screenshots/|^\.worktrees/|^qa-[^/]+-plan\.md$|^QA-[^/]+-REPORT-[^/]+\.md$|^ERROR_STATE_[^/]+\.md$|^docs/fc-[^/]+\.md$|^docs/audit-[^/]+\.md$|^docs/qa/|^verification/|^docs/acceptance-(defects|verdicts)\.md$' \
  | grep -v -E '^verification/cdp_probe\.mjs$' || true)

if [ -n "$offenders" ]; then
  echo "::error::Transient agent artifacts are tracked — move them to .hermes/ or git rm them:"
  echo "$offenders"
  exit 1
fi

echo "OK: no transient artifacts tracked."
exit 0
