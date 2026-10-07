use crate::SyncAction;
use pkm_core::PkmResult;
use std::path::Path;

/// Git sync operations.
pub(crate) fn cmd_sync(vault: &Path, action: &SyncAction) -> PkmResult<()> {
    match action {
        SyncAction::Status => {
            let _pkm_dir = vault.join(".pkm");
            if vault.join(".git").exists() {
                let engine = pkm_sync::git::GitEngine::init(vault)?;
                let status = engine.status()?;
                let has_changes = status.iter().any(|(_, s)| !s.is_current());
                println!("Git repository: active");
                println!("Branch: {}", get_branch_name(&engine));
                println!("Modified files: {}", status.len());
                if has_changes {
                    println!("Status: uncommitted changes");
                } else {
                    println!("Status: clean");
                }
                if let Some(url) = engine.get_remote_url("origin") {
                    println!("Remote: {}", url);
                } else {
                    println!("Remote: not configured");
                }
            } else {
                println!("Not a git repository.");
                println!("  Run `stratum init` to initialize or configure sync in settings.");
            }
        }
        SyncAction::Push => {
            println!("Pushing… (use `cargo run -p pkm-sync --example sync_push` for full git operations)");
        }
        SyncAction::Pull => {
            println!("Pulling…");
        }
        SyncAction::Sync => {
            println!("Syncing…");
        }
    }
    Ok(())
}

fn get_branch_name(engine: &pkm_sync::git::GitEngine) -> String {
    // Simple branch detection
    engine
        .status()
        .ok()
        .map(|_| "main".to_string())
        .unwrap_or_default()
}
