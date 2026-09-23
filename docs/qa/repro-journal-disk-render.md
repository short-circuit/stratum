# Reproduction: journal entries existing as files are not rendered in journal view

Task: t_12ffd5ff (QA reproduction) — feeds backend fix t_a7863cb6.

## Status

REPRODUCED on the pre-fix branch (`wt/t_12ffd5ff`, HEAD `b73ac55`). The buggy
code is present there; the sibling fix branch `wt/t_c6e43085` (commit `6d341c3`)
only partially fixed it (see "Gap" below).

**FIXED** on branch `wt/t_a7863cb6` (this task): the read-path now surfaces
on-disk pages through `list_pages` (merging `journals/*.md` files on disk with
the SQLite page list, deduplicated) IN ADDITION to the `get_blocks_heal_from_disk`
block-row self-heal wired into `get_blocks`/`open_page`. All three regression
tests in `src-tauri/tests/journal_disk_render_regression.rs` pass; existing
`command_tests`, `feature_commands`, and `query_export_commands` remain green
(DB-backed entries unaffected, no duplicates).


## Root cause (verified by reading the source on this branch)

All three read commands the journal view depends on hit SQLite `blocks.db`
exclusively, with no disk fallback:

- `src-tauri/src/commands/page.rs:97` `list_pages` -> `store.list_pages()`
  (SQLite `pages` table). The journal panel derives its `pastDates` list from
  this (see `src/components/JournalPanel/JournalPanel.shared.tsx:66-72`), so a
  journal file that is absent from the `pages` table never appears.
- `src-tauri/src/commands/block.rs:207` `get_blocks` -> `store.get_blocks_by_page()`
  (SQLite `blocks` table). A page with missing/zero block rows returns an empty
  editor.
- `src-tauri/src/commands/page.rs:466` `open_page` -> `store.get_blocks_by_page().len()`.
  Reports block_count 0 for a page whose block rows are missing.

`sync_filesystem_to_db` (`page.rs:236`) only heals pages that are missing from
SQLite or have zero blocks — and it runs at STARTUP. A journal file that lands
on disk after boot (git pull, external editor, interrupted save) is therefore
never reconciled until an app restart. `ensure_today_journal` (`page.rs:836`)
only heals TODAY's file, not historical entries.

## Repro against a copy of the live vault

The live vault `~/StratumVault` was inspected with a read-only script
(`inspect_vault.py`, in the QA scratch dir): 95 journal files on disk, 95 in
the `pages` table, 0 drift at the time of inspection. So the drift shape is
manufactured against a COPY: a real journal `.md` (BlockNote-serialized, same
format as `~/StratumVault/journals/*.md`) is written to a temp vault while
SQLite is left unaware of it — exactly the post-boot `git pull` / external
edit scenario.

Evidence captured: `inspect_vault.py` output showing the live vault inventory
(see task attachment / QA scratch).

## Regression test

`src-tauri/tests/journal_disk_render_regression.rs` (added in this task):

1. `hidden_journal_file_absent_from_db_renders_in_journal_view` — a journal file
   on disk, absent from SQLite. Drives `list_pages`, `get_blocks`, `open_page`
   over the real Tauri IPC mock harness and asserts the on-disk journal renders
   (listed, 3 blocks served, block_count 3), exactly once.
2. `stale_page_with_missing_block_rows_renders_blocks_from_disk` — page row
   present, block rows missing, disk has content. Asserts read path self-heals
   and does not duplicate.
3. `existing_db_entry_is_unaffected_and_no_duplicates` — guards against
   over-correction: fully healthy DB entries read back unchanged, no duplicates.

### Expected result on THIS branch (pre-fix): FAIL

- Test 1: `list_pages` returns 0 pages (empty `pages` table) and
  `get_blocks`/`open_page` return 0 blocks — the hidden journal is invisible.
- Test 2: DB stays stale; `get_blocks` returns 0; `open_page` reports 0.
- Test 3: passes (healthy path is unaffected).

### Expected result on the fix branch (post-fix): PARTIAL — one gap remains

The sibling fix branch `wt/t_c6e43085` (commit `6d341c3`) adds
`get_blocks_heal_from_disk` wired into `get_blocks` and `open_page`. Running
this QA regression against that branch (verified 2026-09-23):

- Test 2 (`stale_page_with_missing_block_rows_renders_blocks_from_disk`): PASS
  — the block-row self-heal works.
- Test 3 (`existing_db_entry_is_unaffected_and_no_duplicates`): PASS — no
  over-correction.
- Test 1 (`hidden_journal_file_absent_from_db_renders_in_journal_view`):
  FAIL — `list_pages` still returns 0 pages.

GAP (now CLOSED by t_a7863cb6): `6d341c3` healed the `blocks` table on read,
but did NOT reconcile the `pages` table. `list_pages` (page.rs:110) called
`store.list_pages()` from SQLite only, so a journal `.md` that exists on disk
but has NO `pages` row was still invisible to the journal panel's `pastDates`.
The complete fix surfaces on-disk pages in `list_pages` (filesystem scan of
`journals/*.md` merged with the SQLite page list, deduplicated — `page.rs`
`list_pages`, helper `page_dto_from_path`), so the journal view renders
on-disk entries without an app restart.

## How to run

```
cd /home/shrtcrct/git/stratum/.worktrees/t_a7863cb6
# frontend dist/ must exist for the Tauri context macro:
#   npm run build   (if the tauri-build proc macro requires ../dist to exist)
cargo test -p stratum-tauri --test journal_disk_render_regression -- --nocapture
```

## Other notes

- Build prerequisite: when the Tauri proc macro `tauri_build` verifies
  `frontendDist` at compile time, an initial `npm run build` is required
  (`dist/` is gitignored). The `tauri::test` mock harness itself
  (`mock_context(noop_assets())`) does not need `dist/`; the requirement, when
  it surfaces, comes from the `frontendDist`/codegen check in the build script.
- The QA regression drives the real IPC commands (it compiled on the PRE-fix
  branch and failed at runtime). It is the reproduction artifact the fix must
  make green — not a duplicate of the fix-side unit tests in `command_tests.rs`.
