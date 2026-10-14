# Git Sync

Stratum uses Git to version your notes, sync across devices, and provide backup.

<!-- SCREENSHOT: [sync-status] Sync status panel showing branch, ahead/behind, and conflicts -->

## How It Works

Your vault is a Git repository. Stratum uses `git2` (libgit2 bindings) for Git operations directly from the Rust backend — no CLI dependency needed.

## Sync Modes

| Mode | Description |
|------|-------------|
| **Manual** | You control when to commit and sync |
| **AutoCommit** | Stratum automatically commits changes on a timer |
| **AutoSync** | Auto-commit + push/pull on a timer |

!!! note "Tray Sync Now"
    The **Sync Now** item in the system tray menu triggers the same sync flow as
    the **Sync Now** button in the Settings → Sync tab. See
    [Quick Capture → System Tray](quick-capture.md#system-tray).

## Setting Up Sync

### Via the CLI

```bash
# Initialize a vault
stratum init

# Set up a Git remote
cd ~/StratumVault
git init
git remote add origin git@github.com:user/vault.git
git add .
git commit -m "Initial commit"
git push -u origin main
```

### Via Configuration

Edit `.pkm/config.toml`:

```toml
[sync]
mode = "AutoCommit"
remote_url = "git@github.com:user/vault.git"
branch = "main"
auto_commit_interval_secs = 300
auto_sync_interval_secs = 1800
```

## Sync Status

Open **Settings** to view sync status:

| Field | Description |
|-------|-------------|
| Status | `ok`, `conflicts`, or `no_repo` |
| Branch | Current Git branch |
| Ahead | Commits to push |
| Behind | Commits to pull |
| Conflicts | Files with merge conflicts |

<!-- SCREENSHOT: [sync-status-detail] Detailed sync status in Settings -->

## Manual Sync

Use the CLI for manual sync:

```bash
# Check status
stratum sync status

# Push changes
stratum sync push

# Pull changes
stratum sync pull

# Full sync
stratum sync sync
```

## Conflict Resolution

When conflicts are detected:

1. Stratum lists the conflicting files
2. Open each file in your editor of choice
3. Resolve the conflict markers (`<<<<<<<`, `=======`, `>>>>>>>`)
4. Stage and commit the resolution

```bash
# After resolving conflicts
git add resolved-file.md
git commit -m "Resolved merge conflict"
git push
```

## Tips

- **Commit often** — automatic commits every 5 minutes mean no lost work
- **Use a private Git repository** for your vault (GitHub, GitLab, your own server)
- **`.pkm/` is in `.gitignore`** by default — only your `.md` files are versioned
- **Cross-platform** — notes sync seamlessly between Linux, macOS, and Windows
- **Pair with [restic](https://restic.net/) or [borg](https://www.borgbackup.org/)** for full backup strategy

## Cloud Backup (WebDAV + Folder)

In addition to Git sync, Stratum can mirror your vault to a **local folder** or a
**WebDAV** endpoint as an independent, periodic backup. Backup is a *separate
layer* from Git sync: it never touches Git state or the search index, so your
version history and your external backup stay fully independent.

### Targets

| Target | Description |
|--------|-------------|
| **Folder** | A local directory (e.g. an external drive or another machine's share). |
| **WebDAV** | A remote WebDAV endpoint (Nextcloud, a NAS, any WebDAV server). |
| **Both** | Runs the folder target first, then the WebDAV target. |

### How It Works

Backup is a **differential per-file mirror**, not a zip archive:

- Files whose size and modification time are unchanged are skipped; only changed
  or new files are copied.
- A `manifest.json` is written at the target root recording the last-known state.
- **Delete propagation** removes files at the target only if they were recorded
  in the previous manifest and no longer exist locally. Files at the target that
  were never part of the manifest are left untouched, so the destination can
  safely contain unrelated data.
- When the destination folder lives **inside** the vault, it is automatically
  excluded from the source walk to prevent recursive self-copy.

### Configuration

Enable and configure backup via **Settings → Sync → Backup**, or by editing
`.pkm/config.toml`:

```toml
[sync.backup]
enabled = true
mode = "both"            # "folder" | "webdav" | "both"
interval_minutes = 60

[sync.backup.webdav]
url = "https://dav.example.com/backup"
username = "user"
password = "secret"

[sync.backup.folder]
path = "/mnt/backup/stratum"
```

Excluded paths are vault-relative and comma-separated in the UI; `.git` is always
excluded even if you omit it. The default excludes are `.git`, `.pkm/blocks.db`,
`.pkm/search`, `.pkm/backup_state.json`, `.pkm/sync_state.json` and `.DS_Store`.

!!! note "Secrets"
    The WebDAV password is stored in plaintext in `.pkm/config.toml` for now.
    Using the OS keyring is planned (see ADR-0007).

### Running & Status

Use **Run backup now** in Settings → Sync → Backup to trigger a backup manually
(regardless of whether the scheduler is enabled), and **Test connection** to
verify WebDAV reachability/auth. The status panel shows the last run time, files
copied/removed, and any error. See
[ADR-0007](../development/adr/0007-cloud-backup.md) for the design rationale.
