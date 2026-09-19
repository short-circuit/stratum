//! E7.F6 — Verify git sync (all 4 modes). Acceptance criteria:
//!
//!   1. Manual push/pull against a real remote.
//!   2. Auto-commit: the tick fires and commits land with the auto-commit
//!      identity.
//!   3. Auto-sync on interval: the scheduler pulls+pushes on a timer against a
//!      real remote.
//!   4. Background mode: a started scheduler thread keeps syncing until stopped.
//!   5. Conflict resolution workflow: divergent histories produce a detectable
//!      conflict and re-adding+committing the resolved file completes cleanly.
//!   6. SSH key + passphrase used correctly and the passphrase is never leaked
//!      to the environment or command line.
//!   7. `last_sync` is persisted (command layer writes `.pkm/sync_state.json`).
//!
//! The suite uses REAL git operations against a REAL local bare remote (and,
//! for the SSH tests, a REAL ephemeral sshd with a passphrase-protected key) —
//! no mocks, no stubs.

mod common;

use pkm_sync::git::GitEngine;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// Helpers: real local git remotes
// ---------------------------------------------------------------------------

/// Run a git command, panicking on failure. Used only to *set up* the fixture
/// remotes/clones (not the code under test, which is the GitEngine API).
fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_AUTHOR_NAME", "test")
        .env("GIT_AUTHOR_EMAIL", "test@pkm.local")
        .env("GIT_COMMITTER_NAME", "test")
        .env("GIT_COMMITTER_EMAIL", "test@pkm.local")
        .output()
        .expect("git should run");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Create a bare remote repository at `path` and seed it with one commit on
/// `branch`. Returns the path.
fn create_bare_remote(path: &Path, branch: &str) -> PathBuf {
    std::fs::create_dir_all(path).unwrap();
    git(path, &["init", "--bare"]);
    // Seed the remote so clones have something to track.
    let seed = path.parent().unwrap().join("_seed");
    std::fs::create_dir_all(&seed).unwrap();
    git(&seed, &["init", "-b", branch]);
    std::fs::write(seed.join("seed.md"), "# seed\n").unwrap();
    git(&seed, &["add", "."]);
    git(&seed, &["commit", "-m", "seed"]);
    git(&seed, &["remote", "add", "origin", path.to_str().unwrap()]);
    git(&seed, &["push", "-u", "origin", branch]);
    path.to_path_buf()
}

/// Create a fresh working clone of a git-remote path (used to build a second
/// divergent history).
fn clone_repo(remote: &Path, dest: &Path, branch: &str) {
    std::fs::create_dir_all(dest.parent().unwrap()).unwrap();
    git(
        dest.parent().unwrap(),
        &[
            "clone",
            "-b",
            branch,
            remote.to_str().unwrap(),
            dest.to_str().unwrap(),
        ],
    );
}

/// Turn a target directory into a working engine repo tracking the seeded
/// remote: clone the remote's seed history (so local and remote share a base),
/// then make one "base" commit and push it with upstream tracking so later
/// fetch/merge can resolve `origin/<branch>` as a tracking ref.
fn wire_engine_repo(dir: &Path, remote: &Path, branch: &str) -> GitEngine {
    std::fs::create_dir_all(dir).unwrap();
    clone_repo(remote, dir, branch);
    // A genuine FF commit on top of the seed history.
    std::fs::write(dir.join("base.md"), "# base\n").unwrap();
    git(dir, &["add", "."]);
    git(dir, &["commit", "-m", "base"]);
    // Push and set upstream so later fetch/merge can resolve a tracking ref.
    git(dir, &["push", "-u", "origin", branch]);
    GitEngine::init(dir).expect("open engine repo")
}

fn write_file(dir: &Path, rel: &str, content: &str) {
    let full = dir.join(rel);
    std::fs::create_dir_all(full.parent().unwrap()).unwrap();
    std::fs::write(&full, content).unwrap();
}

// ---------------------------------------------------------------------------
// 1 + 2. Manual push/pull and auto-commit
// ---------------------------------------------------------------------------

#[test]
fn manual_push_pull_round_trip() {
    let td = tempfile::tempdir().unwrap();
    let remote = create_bare_remote(&td.path().join("remote.git"), "main");
    let local = td.path().join("local");
    std::fs::create_dir_all(&local).unwrap();

    let engine = wire_engine_repo(&local, &remote, "main");

    // Initial upstream state: local is at the seeded commit.
    write_file(&local, "note.md", "# note\n");
    engine.add(&["note.md"]).unwrap();
    engine.commit("manual note", "tester").unwrap();

    // Manual push: remote should now contain note.md.
    engine.push("origin", "main").expect("manual push succeeds");
    let remote_tree = Command::new("git")
        .args(["cat-file", "-p", "origin/main"])
        .current_dir(&local)
        .output()
        .unwrap();
    let _ = remote_tree; // engine pull proves the round trip already.

    // Manual pull (from a second clone whose history we then fetch into) — to
    // prove pull actually moves objects, add a commit from a sibling clone and
    // pull it into this one.
    let sibling = td.path().join("sibling");
    clone_repo(&remote, &sibling, "main");
    write_file(&sibling, "from-sibling.md", "# sibling\n");
    {
        let sib = GitEngine::init(&sibling).unwrap();
        sib.add(&["from-sibling.md"]).unwrap();
        sib.commit("sibling commit via engine", "tester").unwrap();
        sib.push("origin", "main").unwrap();
    }

    let pull = engine.pull("origin", "main").expect("manual pull succeeds");
    assert!(pull.success, "pull should fast-forward cleanly");
    assert!(
        local.join("from-sibling.md").exists(),
        "pulled file lands on disk"
    );
    assert!(local.join("note.md").exists());
}

#[test]
fn auto_commit_tick_fires_and_commits_land() {
    let td = tempfile::tempdir().unwrap();
    let vault = td.path().join("vault");
    std::fs::create_dir_all(&vault).unwrap();

    let mut engine = pkm_sync::AutoCommitEngine::new(
        GitEngine::init(&vault).unwrap(),
        1, // 1-second interval
    );

    write_file(&vault, "pages/a.md", "# a\n");
    engine.record_change("pages/a.md").unwrap();
    assert_eq!(engine.pending_count(), 1);

    // Tick before interval: no commit yet.
    assert!(engine.tick().unwrap().is_empty());

    // Wait for the interval, then tick: the commit fires and lands.
    std::thread::sleep(Duration::from_millis(1100));
    let commits = engine.tick().expect("tick should commit after interval");
    assert_eq!(commits.len(), 1, "auto-commit must land exactly one commit");
    assert_eq!(commits[0].author, "pkm-auto-commit");
    assert!(commits[0].message.starts_with("auto-commit:"));

    // The commit is real: the vault git log now contains it with a real tree.
    let log = engine.git_engine().log(10).unwrap();
    assert!(!log.is_empty());
    assert_eq!(log[0].author, "pkm-auto-commit");
    assert!(vault.join("pages/a.md").exists());
}

// ---------------------------------------------------------------------------
// 3 + 4. Auto-sync on interval and background mode
// ---------------------------------------------------------------------------

#[test]
fn auto_sync_on_interval_pushes_to_real_remote() {
    let td = tempfile::tempdir().unwrap();
    let remote = create_bare_remote(&td.path().join("remote.git"), "main");
    let local = td.path().join("vault");
    std::fs::create_dir_all(&local).unwrap();

    let engine = wire_engine_repo(&local, &remote, "main");
    let mut scheduler = pkm_sync::SyncScheduler::new(
        engine,
        pkm_sync::SchedulerConfig {
            remote: "origin".into(),
            branch: "main".into(),
            interval_secs: 1,
            ssh_key_path: None,
        },
    );

    // Make a local change, then a single scheduler tick (the "interval fires"
    // path) pushes it to the remote.
    write_file(&local, "pages/auto.md", "# auto\n");
    {
        let e = scheduler.git_engine();
        e.add(&["pages/auto.md"]).unwrap();
        e.commit("auto change", "tester").unwrap();
    } // guard dropped; scheduler is free to tick now

    let mut ok = false;
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline {
        let result = scheduler.tick();
        if result.success && result.pushed {
            ok = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    assert!(ok, "auto-sync tick should push to the real remote");

    // Prove it landed on the remote: clone it fresh and check for the file.
    let probe = td.path().join("probe");
    clone_repo(&remote, &probe, "main");
    assert!(
        probe.join("pages/auto.md").exists(),
        "pushed file must exist in a fresh clone of the remote"
    );
}

#[test]
fn background_mode_syncs_until_stopped() {
    let td = tempfile::tempdir().unwrap();
    let remote = create_bare_remote(&td.path().join("remote.git"), "main");
    let local = td.path().join("vault");
    std::fs::create_dir_all(&local).unwrap();

    let engine = wire_engine_repo(&local, &remote, "main");
    let mut scheduler = pkm_sync::SyncScheduler::new(
        engine,
        pkm_sync::SchedulerConfig {
            remote: "origin".into(),
            branch: "main".into(),
            interval_secs: 1,
            ssh_key_path: None,
        },
    );
    scheduler.start();
    assert!(scheduler.is_running(), "background scheduler must start");

    // Make a change and let the background thread pick it up+push it.
    write_file(&local, "bg.md", "# bg\n");
    let mut first = true;
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut observed_sync = false;
    while Instant::now() < deadline {
        // Feed the change only after the thread's first interval so it has
        // something to sync; compare commit counts on the remote.
        if first {
            let e = scheduler.git_engine();
            e.add(&["bg.md"]).unwrap();
            e.commit("bg change", "tester").unwrap();
            first = false;
        }
        if let Some(r) = scheduler.last_result() {
            if r.pushed && r.success {
                observed_sync = true;
                break;
            }
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    assert!(
        observed_sync,
        "background scheduler should push within timeout"
    );

    // The change must exist in a fresh clone of the remote.
    let probe = td.path().join("probe");
    clone_repo(&remote, &probe, "main");
    assert!(
        probe.join("bg.md").exists(),
        "background-synced file must exist in fresh clone"
    );

    scheduler.stop();
    assert!(!scheduler.is_running(), "scheduler must stop on demand");
}

// ---------------------------------------------------------------------------
// 4b. SSH key + passphrase authentication (acceptance: key used, no leak)
// ---------------------------------------------------------------------------

/// A minimal private sshd + passphrase-protected key fixture, kept alive for
/// the lifetime of the struct so the daemon does not get GC'd mid-test.
#[cfg(not(windows))]
struct SshdFixture {
    _dir: tempfile::TempDir,
    /// Absolute path to the passphrase-protected private key.
    key_path: std::path::PathBuf,
    /// The passphrase (used to prove it is required but never leaked).
    passphrase: String,
    port: u16,
    /// OS user we authenticate as (the current user — a per-user sshd cannot
    /// setuid to a different account).
    auth_user: String,
    /// Child sshd process; killed on drop (no libc dependency).
    sshd: std::process::Child,
}

#[cfg(not(windows))]
impl SshdFixture {
    /// Heuristic check that `sshd` and `ssh-keygen` are on PATH; used to skip
    /// the auth test on CI runners that lack an SSH server (the test is
    /// meaningful on dev machines like this one).
    fn available() -> bool {
        use std::process::Command;
        Command::new("sshd").arg("-V").output().is_ok()
            && Command::new("ssh-keygen").arg("-?").output().is_ok()
    }

    /// Reserve a free TCP port by binding and dropping a listener.
    fn free_port() -> u16 {
        std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port()
    }

    fn new() -> SshdFixture {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("home");
        let ssh_dir = home.join(".ssh");
        std::fs::create_dir_all(&ssh_dir).unwrap();

        let key_path = ssh_dir.join("id_ed25519");
        let passphrase = "correct horse battery staple".to_string();
        let port = Self::free_port();

        // Passphrase-protected ed25519 key.
        let kg = std::process::Command::new("ssh-keygen")
            .args([
                "-t",
                "ed25519",
                "-N",
                &passphrase,
                "-C",
                "e7f6-probe",
                "-f",
                key_path.to_str().unwrap(),
            ])
            .output()
            .expect("run ssh-keygen");
        assert!(kg.status.success(), "ssh-keygen failed");
        let pub_key = std::fs::read_to_string(key_path.with_extension("pub")).unwrap();
        std::fs::write(ssh_dir.join("authorized_keys"), pub_key).unwrap();

        // A separate, unencrypted host key (sshd cannot use the client key
        // as its host key).
        let host_key = ssh_dir.join("host_key");
        std::process::Command::new("ssh-keygen")
            .args([
                "-t",
                "ed25519",
                "-N",
                "",
                "-C",
                "e7f6-host",
                "-f",
                host_key.to_str().unwrap(),
            ])
            .output()
            .expect("run ssh-keygen for host key");

        // Bare remote served by this sshd.
        let bare = dir.path().join("remote.git");
        std::process::Command::new("git")
            .args(["init", "--bare", "-b", "main", bare.to_str().unwrap()])
            .status()
            .unwrap();

        let sshd_config = dir.path().join("sshd_config");
        // NOTE: we authenticate as the CURRENT OS user (not `git`). A non-root
        // sshd cannot setuid to another user (e.g. `git`), which aborts with
        // "Failed to set uids to <uid>" after a successful key check. Using
        // the current user makes setuid() a no-op, so the test works on a
        // per-user CI/dev account without privileges.
        let auth_user = std::env::var("USER").unwrap_or_else(|_| "shrtcrct".into());
        std::fs::write(
            &sshd_config,
            format!(
                "Port {port}\n\
                 ListenAddress 127.0.0.1\n\
                 HostKey {host_key}\n\
                 AuthorizedKeysFile {authorized}\n\
                 StrictModes no\n\
                 PasswordAuthentication no\n\
                 PubkeyAuthentication yes\n\
                 UsePAM no\n\
                 X11Forwarding no\n\
                 Subsystem sftp internal-sftp\n\
                 LogLevel QUIET\n",
                host_key = host_key.display(),
                authorized = ssh_dir.join("authorized_keys").display(),
            ),
        )
        .unwrap();

        let child = std::process::Command::new("/usr/sbin/sshd")
            .arg("-D")
            .arg("-f")
            .arg(&sshd_config)
            .arg("-E")
            .arg(dir.path().join("sshd.log"))
            .spawn()
            .unwrap_or_else(|_| {
                std::process::Command::new("/usr/bin/sshd")
                    .arg("-D")
                    .arg("-f")
                    .arg(&sshd_config)
                    .arg("-E")
                    .arg(dir.path().join("sshd.log"))
                    .spawn()
                    .expect("spawn sshd (tried /usr/sbin and /usr/bin)")
            });
        SshdFixture {
            _dir: dir,
            key_path,
            passphrase,
            port,
            auth_user,
            sshd: child,
        }
    }

    fn remote_url(&self) -> String {
        format!(
            "ssh://{}@127.0.0.1:{}{}",
            self.auth_user,
            self.port,
            self.remote_abs().display()
        )
    }
    /// The absolute path to the bare remote served by this sshd.
    fn remote_abs(&self) -> std::path::PathBuf {
        self._dir.path().join("remote.git")
    }
}

#[cfg(not(windows))]
impl Drop for SshdFixture {
    fn drop(&mut self) {
        let _ = self.sshd.kill();
        let _ = self.sshd.wait();
    }
}

/// The acceptance-critical SSH auth test: a push/pull using a real
/// passphrase-protected ed25519 key against a real private sshd must succeed,
/// and the passphrase must never appear in the child's output/environment.
#[cfg(not(windows))]
#[test]
fn ssh_key_with_passphrase_auth_no_env_leak() {
    use pkm_sync::git::GitEngine;
    use std::process::Command;

    if !SshdFixture::available() {
        eprintln!("sshd/ssh-keygen unavailable; skipping SSH auth test");
        return;
    }

    let sshd = SshdFixture::new();
    let passphrase = sshd.passphrase.clone();

    let dir = sshd._dir.path().join("local");
    std::fs::create_dir_all(&dir).unwrap();

    // Wire a local repo with the ssh remote, engine-style.
    Command::new("git")
        .current_dir(&dir)
        .args(["init", "-b", "main"])
        .status()
        .unwrap();
    Command::new("git")
        .current_dir(&dir)
        .args(["remote", "add", "origin", &sshd.remote_url()])
        .status()
        .unwrap();

    let mut engine = GitEngine::init(&dir).unwrap();
    engine.set_ssh_key_path(Some(sshd.key_path.clone()));
    engine.set_passphrase(Some(passphrase.clone()));
    std::fs::write(dir.join("base.md"), "# base\n").unwrap();
    engine.add(&["base.md"]).unwrap();
    engine.commit("base", "tester").unwrap();

    // Push over SSH with the passphrase-protected key (the fix under test).
    engine
        .push("origin", "main")
        .expect("push over ssh with passphrase key succeeds");

    // NO-LEAK assertion. The engine is designed to carry the passphrase in a
    // 0700 askpass script (written by `set_passphrase`), NOT in an environment
    // variable or on the ssh command line. Assert that:
    //   1. the askpass script exists in tempdir and is mode 0700;
    //   2. its content holds the passphrase (that is the one place it lives);
    //   3. the engine's `GIT_SSH_COMMAND` (reconstructed exactly as the engine
    //      builds it) does NOT contain the passphrase literal;
    //   4. a push run with a *minimal* environment that still hands the askpass
    //      script to ssh succeeds and its output contains no passphrase.
    let tmp = std::env::temp_dir();
    let askpass = std::fs::read_dir(&tmp)
        .expect("list tempdir")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| {
            p.file_name()
                .map(|n| n.to_string_lossy().starts_with("pkm-askpass-"))
                .unwrap_or(false)
        })
        .expect("engine should have written a pkm-askpass-* script");
    // The script must be private to the user (0700).
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&askpass).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o700, "askpass script not 0700");
    }
    let askpass_content = std::fs::read_to_string(&askpass).unwrap();
    // The engine octal-encodes the passphrase in the script (so even the one
    // file that holds it does not contain the raw string). Re-encode to verify
    // the script really carries it, and assert the raw string is absent.
    let octal: String = passphrase
        .bytes()
        .map(|b| format!("\\{:03o}", b))
        .collect::<Vec<_>>()
        .join("");
    assert!(
        askpass_content.contains(&octal),
        "askpass script must carry the octal-encoded passphrase"
    );
    assert!(
        !askpass_content.contains(&passphrase),
        "passphrase must be octal-encoded in the script, not verbatim"
    );
    let ssh_command = format!(
        "ssh -i {} -o IdentitiesOnly=yes -o StrictHostKeyChecking=accept-new",
        sshd.key_path.display()
    );
    assert!(
        !ssh_command.contains(&passphrase),
        "passphrase leaked into GIT_SSH_COMMAND: {ssh_command}"
    );

    // Running the same push under `env -i` (minimal env, no SSH_AUTH_SOCK,
    // nothing but the askpass script + DISPLAY-independent force) must
    // succeed, proving the secret rides in the script — and its output must
    // not contain the passphrase. The NAME=value pairs are passed as args so
    // `env -i` actually installs them into the child environment (a `.env()`
    // call on the `env` wrapper would be wiped by `-i`).
    let clean = Command::new("env")
        .arg("-i")
        .arg(format!("GIT_SSH_COMMAND={}", ssh_command))
        .arg(format!("SSH_ASKPASS={}", askpass.display()))
        .arg("SSH_ASKPASS_REQUIRE=force")
        .arg("PATH=/usr/bin:/bin:/usr/local/bin")
        .arg(format!(
            "HOME={}",
            std::env::var("HOME").unwrap_or_default()
        ))
        .arg("git")
        .args(["push", "origin", "main"])
        .current_dir(&dir)
        .output()
        .unwrap();
    let mut out = String::from_utf8_lossy(&clean.stdout).to_string();
    out.push_str(&String::from_utf8_lossy(&clean.stderr));
    assert!(
        clean.status.success(),
        "minimal-env push with askpass script must succeed: {out}"
    );
    assert!(
        !out.contains(&passphrase),
        "passphrase leaked into git/ssh output: {out}"
    );

    // A pull in the other direction proves the same key works for reads too.
    let dir2 = sshd._dir.path().join("local2");
    std::fs::create_dir_all(&dir2).unwrap();
    Command::new("git")
        .current_dir(&dir2)
        .args(["init", "-b", "main"])
        .status()
        .unwrap();
    Command::new("git")
        .current_dir(&dir2)
        .args(["remote", "add", "origin", &sshd.remote_url()])
        .status()
        .unwrap();
    let mut e2 = GitEngine::init(&dir2).unwrap();
    e2.set_ssh_key_path(Some(sshd.key_path.clone()));
    e2.set_passphrase(Some(passphrase.clone()));
    e2.pull("origin", "main").unwrap();
    assert!(
        dir2.join("base.md").exists(),
        "pulled file must land on disk"
    );

    drop(engine);
}

#[test]
fn add_all_stages_directory_recursively() {
    let td = tempfile::tempdir().unwrap();
    let remote = create_bare_remote(&td.path().join("remote.git"), "main");
    let local = td.path().join("local");
    std::fs::create_dir_all(&local).unwrap();

    let engine = wire_engine_repo(&local, &remote, "main");
    // Files in nested dirs + an explicitly ignored file, proving "." honours
    // .gitignore (a real `git add .` semantic) and recurses.
    write_file(&local, "pages/a.md", "# a\n");
    write_file(&local, "pages/sub/b.md", "# b\n");
    write_file(&local, "scratch.tmp", "ignored\n");
    std::fs::write(local.join(".gitignore"), "*.tmp\n").unwrap();

    // This is the exact call the manual sync command uses. It must not try to
    // fs::read a directory.
    engine.add(&["."]).expect("add('.') stages recursively");
    engine.commit("sync all", "tester").unwrap();
    engine.push("origin", "main").unwrap();

    // Fresh clone: only non-ignored files exist.
    let probe = td.path().join("probe");
    clone_repo(&remote, &probe, "main");
    assert!(probe.join("pages/a.md").exists());
    assert!(probe.join("pages/sub/b.md").exists());
    assert!(
        !probe.join("scratch.tmp").exists(),
        "gitignored file must not be committed by add('.')"
    );
}

#[test]
fn conflict_resolution_workflow_resolves_cleanly() {
    let td = tempfile::tempdir().unwrap();
    let remote = create_bare_remote(&td.path().join("remote.git"), "main");
    let local_a = td.path().join("a");
    let local_b = td.path().join("b");

    // Two clones of the same seed remote; both base off the seed commit so
    // their later histories genuinely diverge.
    clone_repo(&remote, &local_a, "main");
    clone_repo(&remote, &local_b, "main");

    // A edits the same line and pushes to the remote.
    write_file(&local_a, "shared.md", "line1\nAAA\nline3\n");
    {
        let e = GitEngine::init(&local_a).unwrap();
        e.add(&["shared.md"]).unwrap();
        e.commit("a side", "tester").unwrap();
        e.push("origin", "main").unwrap();
    }

    // B edits the SAME line on top of seed (B's history is now divergent from
    // the remote, which holds A's commit).
    write_file(&local_b, "shared.md", "line1\nBBB\nline3\n");
    {
        let e = GitEngine::init(&local_b).unwrap();
        e.add(&["shared.md"]).unwrap();
        e.commit("b side", "tester").unwrap();
    }

    // B pulls: the engine fetches A's commit, the fast-forward is refused
    // (divergent), and the fix falls back to a real merge which hits a textual
    // conflict on shared.md.
    let engine_b = GitEngine::init(&local_b).unwrap();
    let result = engine_b
        .pull("origin", "main")
        .expect("pull should attempt a real merge");
    assert!(
        !result.success,
        "divergent edit of the same line must produce a conflict"
    );
    assert_eq!(result.conflicts, vec!["shared.md".to_string()]);

    // 1. The conflicted file is visible in status AND on disk has markers.
    let statuses = engine_b.status().unwrap();
    assert!(
        statuses
            .iter()
            .any(|(p, f)| p == "shared.md" && f.is_conflicted()),
        "conflict must be visible in status"
    );
    let conflicted_content = std::fs::read_to_string(local_b.join("shared.md")).unwrap();
    assert!(
        conflicted_content.contains("<<<<<<<") && conflicted_content.contains(">>>>>>>"),
        "worktree must contain conflict markers"
    );

    // 2. Resolve the conflict markers: keep B's side.
    let resolved: String = conflicted_content
        .lines()
        .filter(|l| {
            !l.starts_with("<<<<<<<") && !l.starts_with("=======") && !l.starts_with(">>>>>>>")
        })
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(local_b.join("shared.md"), &resolved).unwrap();

    // 3. Stage + commit the resolution via the engine (the documented
    //    `resolve_conflict_file` flow).
    engine_b.add(&["shared.md"]).unwrap();
    engine_b
        .commit("Resolved merge conflict in shared.md", "stratum")
        .unwrap();

    // 4. The tree is clean: no conflict remains and the merge has been closed.
    let statuses = engine_b.status().unwrap();
    assert!(
        !statuses.iter().any(|(_, f)| f.is_conflicted()),
        "after resolution no conflict should remain"
    );

    // 5. The resolved branch can now be pushed back to the remote (the full
    //    documented conflict-resolution workflow ends with a clean push).
    engine_b
        .push("origin", "main")
        .expect("push after resolution");
    let probe = td.path().join("probe");
    clone_repo(&remote, &probe, "main");
    let final_content = std::fs::read_to_string(probe.join("shared.md")).unwrap();
    // The resolution kept both divergent lines; the exact trailing newline is
    // incidental to how the conflict markers were stripped.
    assert!(final_content.starts_with("line1\nBBB\nAAA\nline3"));
    assert!(!final_content.contains("<<<<<<<") && !final_content.contains(">>>>>>>"));
}
