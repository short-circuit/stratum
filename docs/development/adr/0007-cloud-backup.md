# ADR-0007 — Cloud Backup (WebDAV + Folder-watch)

- **Status:** accepted
- **Deciders:** backend-dev, architect
- **Date:** 2026-10-13
- **Related:** `docs/guide/git-sync.md` (user guide), `crates/pkm-sync/src/backup/`
  (implementation), `pkm-core` `BackupConfig` (configuration)

## Context

The product requirement D4 asks for an off-site backup of the vault that is
independent of the Git remote and does not require network access to the Git
host. Git sync is the version-history layer; backup is a mirror for disaster
recovery. The two must stay decoupled so a broken or unavailable Git remote
never blocks backup, and so backup never mutates Git state or the derived
search index.

## Decision

Implement backup as a **separate periodic mirror layer** in `crates/pkm-sync/src/backup/`,
with two independent targets — a local folder and a WebDAV endpoint — plus a
"both" mode that runs folder first then WebDAV.

The mirror is **differential per-file**, not a zip archive:

- Only files whose size or modification time changed are copied; unchanged files
  are skipped.
- A `manifest.json` at the target root records `{relpath: {size, mtime}}` for
  the last-known state of the mirrored set (as a `BTreeMap` for deterministic
  output).
- **Delete propagation** removes a target file only if it was recorded in the
  *previous* manifest and no longer exists locally. Target files never recorded
  in a manifest are left untouched, so the destination may safely contain
  unrelated data (folder-watch safe when the destination holds other content).

## Alternatives

- **Zip archive:** unbounded growth on each run, and verifying integrity of a
  large monolithic archive is expensive and opaque. Rejected.
- **Full copy every run:** wasteful when only a handful of files changed.
  Rejected.

## Delete-propagation policy

Only manifest-tracked files are ever deleted at the target. This is the key
safety property: a folder that also contains unrelated files, or a WebDAV
collection that other tools write to, is never pruned by Stratum.

## Secrets

The WebDAV password is stored in plaintext in `.pkm/config.toml` for now, and
the UI only ever receives a masked value (`****` pattern). The same masked
value is preserved on save (see `save_settings` in `src-tauri`) so editing other
settings never clobbers the stored secret. Integrating the OS keyring
(`keyring-rs`) is tracked as a follow-up; this ADR records the current state.

## Dependencies

- `reqwest_dav` — WebDAV client. License: MIT OR Apache-2.0 (AGPL-compatible).
- `dav-server` — in-process WebDAV server used only in dev/tests. License:
  Apache-2.0 (AGPL-compatible).

## Consequences

- Backup is fully independent of Git; the scheduler runs in its own thread
  (`pkm-sync::BackupScheduler`), separate from `SyncScheduler`.
- Running backup never blocks on the Git remote and never modifies the index.
- Deleting a note locally will remove it from the backup on the next run; files
  removed manually at the target are restored on the next run because they are
  re-recorded in the manifest only when present locally (so a manual deletion at
  the target of a still-present local file is re-copied).
