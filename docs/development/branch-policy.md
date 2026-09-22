# Branch Policy and Hygiene

This document defines how branches are named, protected, and kept clean in the
`stratum` repository. It also records completed branch-cleanup passes.

## Protected Branches

The following branches are protected and **must not** be deleted. Review them
before any cleanup pass, and treat any pattern listed here as a protected
category:

- `master` — the integration trunk. All feature work merges here; CI defines it
  as the deployment branch.
- `gh-pages` — the published documentation site (built by the `docs`
  workflow and deployed via `peaceiris/actions-gh-pages`).
- `release/*` — release-preparation branches (for example
  `release/v0.7.1-version-t_5eb0059b`). CDN/CI and the release process depend on
  them; keep the current and any in-flight release branches.
- Live integration branches — user-facing feature branches that are still
  actively developed and ahead of `master` (for example the MCP server
  integration branch). Anything with open, unmerged work toward a planned
  feature is protected. A branch is only clean-up-eligible once its work has
  been merged to `master` (for example via PR) or is confirmed abandoned.
- `backup/*` — emergency safety copies of unmerged work that exists nowhere
  else (for example `backup/e8-sizing-gate-splits`, the only copy of an
  unfinished E8 refactor). Treat these as protected until the work is landed or
  permanently superseded.

## Worktrees and Workspace Branches

Contributors often work through Hermes-style task worktrees under
`.worktrees/<task-id>` on branches named `wt/<task-id>` (or
`stratum/<task-id>-<slug>` for project-linked task worktrees). These branches
are transient by design:

- They are **not** protected. Their commits are expected to land on `master`
  via PR; the branch itself is deleted afterwards.
- Before deleting a branch that is (or was) checked out in a worktree, remove
  the worktree first, otherwise local deletion will be refused:
  `git worktree remove --force .worktrees/<task-id>`.
- After a cleanup pass, run `git worktree prune` and `git fetch --prune origin`
  to drop stale worktree bookkeeping and remote-tracking refs.

## Identifying Merged or Stale Branches

Before proposing any branch for deletion, verify it is safe with Git itself —
never delete on naming convention alone:

1. **Fully merged into `master`** (safe to delete):

   ```bash
   git branch -r --merged master
   git branch -d <branch>             # refuses if not fully merged
   ```

2. **Ancestor of a live/protected branch** (safe to delete — its work is
   reachable through the protected branch):

   ```bash
   git merge-base --is-ancestor <branch> <protected-branch> && echo "ancestor — safe"
   ```

3. **Stale/unused** — no commits ahead of `master` and no open PR or worktree
   referencing it. Check with `git log origin/<branch> ^master --oneline` and
   review whether any open PR or issue still depends on it.

## Cleanup Command Sequence

The standard cleanup procedure for merged/stale branches:

```bash
# 1. Ensure the work you want to keep is reachable from a protected branch.
#    If the branch carries unique commits not on master, merge them first
#    (fe.g. by opening a PR and landing it), otherwise the work is lost.
git checkout master && git pull

# 2. If the branch is checked out in a worktree, remove that worktree first.
git worktree remove --force .worktrees/<task-id>

# 3. Delete on the remote (origin) and locally, in that order.
git push origin --delete <branch>
git branch -d <branch>               # use -D only if the branch is not merged

# 4. Prune stale tracking refs and worktree bookkeeping.
git fetch --prune origin
git worktree prune
```

Deleting the remote ref first guarantees the local delete can never re-push a
dead branch by accident. Verify afterwards that the protected branch set is
intact and local `master` is in sync with `origin/master`.

## Cleanup Log

### 2026-09-22 — merged / stale branch cleanup

Completed as part of the "remove unused/merged remote branches" housekeeping
request. All deletions were verified safe before execution and the protected
branches above were left untouched.

**Branches deleted** (from `origin`, and locally where they were tracked):

1. `e7-land-verified` — merged into `master`
2. `fix/backend-acceptance-defects-t_d65ce325` — merged into `master`
3. `fix/mobile-ondevice-defects-t_3a936d34` — merged into `master`
4. `fix/mobile-settings-vault-stale-t_126309a2` — merged into `master`
5. `fix/release-land-e7-missing-acceptance-t_5eb0059b` — merged into `master`
6. `stratum/t_2253206d-mobile-smoke` — its worktree
   `.worktrees/t_2253206d` was removed first; its unique E7.F9 fix was
   preserved via PR #192 (merged to `master` as `6f5072a`) before deletion
7. `wt/t_2335c56d` — remote-only; ancestor of the live MCP integration branch
8. `wt/t_4acd5437` — remote-only; ancestor of the live MCP integration branch
9. `wt/t_547e25f1` — remote-only; ancestor of the live MCP integration branch
10. `wt/t_af0ce286` — remote-only; ancestor of the live MCP integration branch
11. `scratch/merge-e7f9` — PR scratch branch, deleted after PR #192 merged

**Verification performed:** all ten pre-existing branches were confirmed to be
ancestors of `origin/master` or of the live MCP integration branch; local
`master` matched `origin/master` at `6f5072a`; `git fetch --prune origin`
completed cleanly and the remote afterwards contained exactly the five
protected branches (`master`, `gh-pages`,
`release/v0.7.1-version-t_5eb0059b`, `stratum/t_7a548710-mcp-server`,
`backup/e8-sizing-gate-splits`).

Full deletion record and audit: this document plus the board task
`t_8098a7d2` (deletion record) and its audit artifact
`stratum-remote-branch-audit.md`.
