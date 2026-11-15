#!/usr/bin/env bash
# Verify the native shell's accessibility tree.
#
# Runs the AccessKit probe (`scripts/capture-gpui-a11y.sh`) and audits the tree
# it reports:
#
# - When assistive technology is connected, every node that carries an action
#   role (button, link, checkbox, textbox, tab) must expose a non-empty name, and
#   no name may be a raw catalog key (a leaked `foo.bar` string).
# - When no AT is connected gpui does not build the tree, so the probe prints
#   `{"a11y": "inactive"}`; the script reports that and exits 0. CI installs
#   `at-spi2-core` and runs the probe inside a D-Bus session, which is the closest
#   a headless runner gets to an active screen reader.
#
# Usage: scripts/verify-gpui-a11y.sh [tree.json]
# Requirements: Xvfb, python3; at-spi2-core for an active tree.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TREE="${1:-${A11Y_OUT:-$ROOT/.hermes/a11y/tree.json}}"

"$ROOT/scripts/capture-gpui-a11y.sh" "$TREE"

python3 - "$TREE" <<'PY'
import json, sys

path = sys.argv[1]
with open(path) as fh:
    raw = fh.read().strip()
try:
    data = json.loads(raw)
except json.JSONDecodeError:
    print(f"a11y: unreadable tree JSON in {path}", file=sys.stderr)
    sys.exit(1)

if data.get("a11y") == "inactive":
    print("a11y: inactive (no assistive technology connected; tree not built)")
    sys.exit(0)

nodes = data.get("tree", {}).get("nodes", [])
interactive = {"button", "link", "checkbox", "textbox", "tab", "combobox", "menuitem"}
failures = []
named = 0
for node_id, node in nodes:
    role = (node.get("role") or "").lower()
    name = (node.get("name") or "").strip()
    if role in interactive:
        if not name:
            failures.append(f"{node_id}: {role} has no accessible name")
        elif "." in name and " " not in name:
            # A label that looks like a catalog key means the raw key leaked.
            failures.append(f"{node_id}: {role} name looks like a raw key: {name!r}")
        else:
            named += 1
print(f"a11y: active, {len(nodes)} nodes, {named} named interactive controls")
if failures:
    for failure in failures:
        print(f"a11y: FAIL {failure}", file=sys.stderr)
    sys.exit(1)
PY
