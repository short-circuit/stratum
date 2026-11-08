# Frontend Freeze Policy

Stratum's web frontend (React + Tauri under `src/` and `src-tauri/`) is
**frozen at v1.0.0**. That build is the behavioural and visual reference the
gpui rewrite is measured against: every parity check in
`.hermes/plans/2026-11-08_101000-gpui-frontend-implementation.md` compares the
new frontend to what v1.0.0 ships today.

Freeze starts at the v1.0.0 release commit and ends at cutover — the release
that deletes `src/`, `src-tauri/`, npm and the Playwright harness.

## What is allowed

- **Bug fixes.** A defect that ships in v1.0.0 may be fixed on `master`.
- **Security fixes** and dependency updates that clear an advisory.
- **Documentation and tests** that do not change behaviour.
- **CI maintenance** needed to keep the release pipeline working.

A fix must stay surgical: repair the defect, touch nothing else. A "fix" that
adds a panel, a settings tab, a config key, or changes a database schema is a
feature and gets parked.

## What is not allowed

- New features, new panels, new settings tabs, new commands.
- Refactors of code the fix does not touch.
- Schema changes to the vault, `blocks.db`, or `.pkm/config.toml`.
- Dependency upgrades beyond security advisories.
- Changes to the IPC contract: command names, argument shapes and payloads are
  part of the parity reference.

## The one structural exception

Phase P0 (card `t_d4ebc13d`, "Extract headless stratum-app service crate") moves
command implementations out of `src-tauri` into a new crate. That is a
structural change, not a feature, and it is permitted **only** under these
conditions:

- the frozen web app behaves identically before and after;
- every Tauri command keeps its name, arguments and response shape;
- the existing e2e harness and full test suite stay green;
- no user-visible behaviour, layout or wording changes.

Any P0 PR that fails those conditions is not a refactor — it is a bug, and the
freeze applies.

## Hotfixes

Fixes that must ship to users of the frozen build branch from the release tag,
not from `master`:

```bash
git checkout -b hotfix/<slug> v1.0.0
# ...fix, test, PR against master...
```

Land the fix on `master` as well so the reference build and the trunk do not
diverge, then tag a patch release if users need the binary.

## Enforcement

- Every PR against `master` during the freeze is reviewed against this policy.
  Anything that is not a fix, a security/dependency update, docs or tests needs
  the `freeze-exception` label and a one-line justification in the PR body.
- `repo-hygiene` and the full CI matrix are unchanged: a fix must still pass
  everything a feature did.
- The gpui work lives on the `gpui-frontend` branch (and per-card worktrees) and
  is not subject to the freeze. It merges to `master` only at cutover.
