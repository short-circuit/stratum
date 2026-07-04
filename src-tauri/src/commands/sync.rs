//! Git sync commands.

use crate::commands::vault::AppState;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SyncStatusDto {
    pub status: String,
    pub branch: Option<String>,
    pub ahead: usize,
    pub behind: usize,
    pub conflicts: Vec<String>,
    pub last_sync_time: Option<String>,
    pub last_sync_success: Option<bool>,
    pub pending_commits: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CommitLogEntry {
    pub hash: String,
    pub author: String,
    pub message: String,
    pub timestamp: String,
}

/// Read the vault config from disk, returning defaults if none exists.
fn read_config(vault_path: &std::path::Path) -> Result<pkm_core::Config, String> {
    let config_path = vault_path.join(".pkm").join("config.toml");
    if config_path.exists() {
        pkm_core::Config::load(&config_path).map_err(|e| e.to_string())
    } else {
        Ok(pkm_core::Config {
            vault_path: vault_path.to_path_buf(),
            ..Default::default()
        })
    }
}

#[tauri::command]
pub async fn get_sync_status(state: tauri::State<'_, AppState>) -> Result<SyncStatusDto, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let vault_path = &state.vault_path;

    match pkm_sync::git::GitEngine::init(vault_path) {
        Ok(engine) => {
            let statuses = engine.status().map_err(|e| e.to_string())?;
            let conflicts: Vec<String> = statuses
                .iter()
                .filter(|(_, s)| s.is_conflicted())
                .map(|(p, _)| p.clone())
                .collect();

            let pending_commits = statuses.iter().filter(|(_, s)| !s.is_conflicted()).count();

            let branch = engine.get_current_branch();

            // Try to get ahead/behind if we have a config
            let config = read_config(vault_path).ok();
            let ahead_behind = config.as_ref().and_then(|c| {
                let branch_name = branch.as_deref().unwrap_or(&c.sync.branch);
                engine.ahead_behind(branch_name).ok()
            });

            let (ahead, behind) = ahead_behind.unwrap_or((0, 0));

            Ok(SyncStatusDto {
                status: if conflicts.is_empty() {
                    "ok".into()
                } else {
                    "conflicts".into()
                },
                branch,
                ahead,
                behind,
                conflicts,
                last_sync_time: None,
                last_sync_success: None,
                pending_commits,
            })
        }
        Err(_) => Ok(SyncStatusDto {
            status: "no_repo".into(),
            branch: None,
            ahead: 0,
            behind: 0,
            conflicts: Vec::new(),
            last_sync_time: None,
            last_sync_success: None,
            pending_commits: 0,
        }),
    }
}

#[tauri::command]
pub async fn sync_vault(state: tauri::State<'_, AppState>) -> Result<SyncStatusDto, String> {
    let vault_path = state.lock().map_err(|e| e.to_string())?.vault_path.clone();
    let passphrase = state.lock().map_err(|e| e.to_string())?.passphrase.clone();

    let config = read_config(&vault_path)?;

    let mut engine = pkm_sync::git::GitEngine::init(&vault_path).map_err(|e| e.to_string())?;

    // Set SSH key path from config
    if let Some(ref key_path) = config.sync.ssh_key_path {
        engine.set_ssh_key_path(Some(std::path::PathBuf::from(key_path)));
    }
    // Set passphrase if cached
    if let Some(ref pp) = passphrase {
        engine.set_passphrase(Some(pp.clone()));
    }

    // Stage all
    engine.add(&["."]).map_err(|e| e.to_string())?;

    // Classify pending files and check for conflicts
    let statuses = engine.status().map_err(|e| e.to_string())?;
    let conflicts: Vec<String> = statuses
        .iter()
        .filter(|(_, s)| s.is_conflicted())
        .map(|(p, _)| p.clone())
        .collect();

    if !conflicts.is_empty() {
        return Ok(SyncStatusDto {
            status: "conflicts".into(),
            branch: engine.get_current_branch(),
            ahead: 0,
            behind: 0,
            conflicts,
            last_sync_time: None,
            last_sync_success: Some(false),
            pending_commits: statuses.len(),
        });
    }

    let pending_count = statuses.iter().filter(|(_, s)| !s.is_conflicted()).count();

    if pending_count > 0 {
        let mut editedfiles = Vec::new();
        let mut newfiles = Vec::new();
        let mut deletedfiles = Vec::new();

        for (path, st) in &statuses {
            if st.is_conflicted() {
                continue;
            }
            use git2::Status;
            if st.intersects(Status::INDEX_NEW | Status::WT_NEW) {
                newfiles.push(path.clone());
            } else if st.intersects(Status::INDEX_DELETED | Status::WT_DELETED) {
                deletedfiles.push(path.clone());
            } else {
                editedfiles.push(path.clone());
            }
        }

        let datetime = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();
        let count = pending_count.to_string();
        let edited_str = editedfiles.join(", ");
        let new_str = newfiles.join(", ");
        let del_str = deletedfiles.join(", ");

        let mut msg = config
            .sync
            .commit_template
            .replace("{datetime}", &datetime)
            .replace("{editedfiles}", &edited_str)
            .replace("{newfiles}", &new_str)
            .replace("{deletedfiles}", &del_str)
            .replace("{count}", &count);

        // Clean up empty clauses
        for suffix in &["edited", "added", "deleted"] {
            msg = msg.replace(&format!(",  {}", suffix), "");
        }
        msg = msg.replace("  edited, ", "");
        msg = msg.replace("  added, ", "");
        msg = msg.replace("  deleted", "");
        while msg.contains("  ") {
            msg = msg.replace("  ", " ");
        }
        let msg = msg.trim().to_string();
        let message = if msg.is_empty() {
            format!("auto-commit: {} files", count)
        } else {
            msg
        };

        engine
            .commit(&message, "pkm-sync")
            .map_err(|e| e.to_string())?;
    }

    // Pull with credentials
    if config.sync.remote_url.is_some() {
        let _ = engine.pull("origin", &config.sync.branch);
        // Check for pull conflicts
        let statuses = engine.status().map_err(|e| e.to_string())?;
        let conflicts: Vec<String> = statuses
            .iter()
            .filter(|(_, s)| s.is_conflicted())
            .map(|(p, _)| p.clone())
            .collect();
        if !conflicts.is_empty() {
            return Ok(SyncStatusDto {
                status: "conflicts".into(),
                branch: engine.get_current_branch(),
                ahead: 0,
                behind: 0,
                conflicts,
                last_sync_time: None,
                last_sync_success: Some(false),
                pending_commits: statuses.len(),
            });
        }

        let _ = engine.push("origin", &config.sync.branch);
    }

    // Return final status
    let final_statuses = engine.status().map_err(|e| e.to_string())?;
    let final_conflicts: Vec<String> = final_statuses
        .iter()
        .filter(|(_, s)| s.is_conflicted())
        .map(|(p, _)| p.clone())
        .collect();
    let conflicts_ok = final_conflicts.is_empty();

    Ok(SyncStatusDto {
        status: if conflicts_ok {
            "ok".into()
        } else {
            "conflicts".into()
        },
        branch: engine.get_current_branch(),
        ahead: 0,
        behind: 0,
        conflicts: final_conflicts,
        last_sync_time: None,
        last_sync_success: Some(conflicts_ok),
        pending_commits: final_statuses.len(),
    })
}

#[tauri::command]
pub async fn sync_vault_with_passphrase(
    passphrase: String,
    state: tauri::State<'_, AppState>,
) -> Result<SyncStatusDto, String> {
    // Cache passphrase in state
    {
        let mut s = state.lock().map_err(|e| e.to_string())?;
        s.set_passphrase(passphrase);
    }
    // Then call sync_vault logic
    sync_vault(state).await
}

#[tauri::command]
pub async fn start_sync_scheduler(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let vault_path = state.lock().map_err(|e| e.to_string())?.vault_path.clone();
    let passphrase = state.lock().map_err(|e| e.to_string())?.passphrase.clone();

    let config = read_config(&vault_path)?;
    let ssh_key_path = config
        .sync
        .ssh_key_path
        .clone()
        .map(std::path::PathBuf::from);

    let mut engine = pkm_sync::git::GitEngine::init(&vault_path).map_err(|e| e.to_string())?;
    engine.set_ssh_key_path(ssh_key_path);
    if let Some(ref pp) = passphrase {
        engine.set_passphrase(Some(pp.clone()));
    }

    let interval = match config.sync.mode {
        pkm_core::SyncMode::AutoSync | pkm_core::SyncMode::Background => {
            config.sync.auto_sync_interval_secs
        }
        _ => 300,
    };

    let sched_config = pkm_sync::scheduler::SchedulerConfig {
        remote: "origin".to_string(),
        branch: config.sync.branch.clone(),
        interval_secs: interval,
        ssh_key_path: None, // SSH key is already set on the engine
    };

    let mut scheduler = pkm_sync::SyncScheduler::new(engine, sched_config);
    scheduler.start();

    let mut s = state.lock().map_err(|e| e.to_string())?;
    s.sync_scheduler = Some(scheduler);
    Ok(())
}

#[tauri::command]
pub async fn stop_sync_scheduler(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut s = state.lock().map_err(|e| e.to_string())?;
    if let Some(mut scheduler) = s.sync_scheduler.take() {
        scheduler.stop();
    }
    Ok(())
}

#[tauri::command]
pub async fn get_commit_log(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<CommitLogEntry>, String> {
    let vault_path = state.lock().map_err(|e| e.to_string())?.vault_path.clone();
    let engine = pkm_sync::git::GitEngine::init(&vault_path).map_err(|e| e.to_string())?;
    let commits = engine.log(50).map_err(|e| e.to_string())?;

    Ok(commits
        .iter()
        .map(|c| CommitLogEntry {
            hash: c.hash.clone(),
            author: c.author.clone(),
            message: c.message.clone(),
            timestamp: c.timestamp.to_rfc3339(),
        })
        .collect())
}

#[tauri::command]
pub async fn resolve_conflict_file(
    path: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let vault_path = state.lock().map_err(|e| e.to_string())?.vault_path.clone();
    let engine = pkm_sync::git::GitEngine::init(&vault_path).map_err(|e| e.to_string())?;
    engine.add(&[&path]).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn abort_merge(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let vault_path = state.lock().map_err(|e| e.to_string())?.vault_path.clone();
    let engine = pkm_sync::git::GitEngine::init(&vault_path).map_err(|e| e.to_string())?;
    let repo = engine.repository();

    // Cleanup merge state
    repo.cleanup_state()
        .map_err(|e| format!("cleanup state: {e}"))?;

    // Force checkout HEAD to restore working directory
    repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
        .map_err(|e| format!("checkout HEAD: {e}"))?;

    Ok(())
}
