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
