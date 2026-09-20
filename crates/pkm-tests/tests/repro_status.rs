use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn git(cwd: &Path, args: &[&str]) {
    let st = Command::new("git")
        .current_dir(cwd)
        .args(args)
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@t")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@t")
        .status()
        .unwrap();
    assert!(st.success(), "git {args:?} failed with {st}");
}

fn git_out(cwd: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .current_dir(cwd)
        .args(args)
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).to_string()
}

#[test]
fn repro_from_tree_in_engine() {
    let td = TempDir::new().unwrap();
    let remote = td.path().join("remote.git");
    git(
        td.path(),
        &["init", "--bare", "-b", "main", remote.to_str().unwrap()],
    );

    let vault = td.path().join("vault");
    std::fs::create_dir_all(vault.join(".pkm")).unwrap();
    std::fs::create_dir_all(vault.join("pages")).unwrap();
    pkm_block::BlockStore::open(&vault.join(".pkm").join("blocks.db")).unwrap();

    git(&vault, &["init", "-b", "main"]);
    let origin_url = format!("file://{}", remote.display());
    git(&vault, &["remote", "add", "origin", origin_url.as_str()]);
    std::fs::write(vault.join("README.md"), "# vault\n").unwrap();
    git(&vault, &["add", "."]);
    git(&vault, &["commit", "-m", "seed"]);

    // Use the engine's repo ref to reproduce the from_tree call.
    let engine = pkm_sync::GitEngine::init(&vault).unwrap();
    let repo = engine.repository();
    eprintln!("=== HEAD tree ===");
    eprintln!("{}", git_out(&vault, &["rev-parse", "HEAD^{tree}"]));
    match repo.head_tree_id_or_empty() {
        Ok(id) => {
            eprintln!("=== head_tree_id_or_empty = {id} ===");
            match repo.index_from_tree(&id) {
                Ok(_) => eprintln!("from_tree OK"),
                Err(e) => eprintln!("from_tree ERR: {e:?}"),
            }
        }
        Err(e) => eprintln!("head_tree_id_or_empty ERR: {e}"),
    }
    let _ = td;
}
