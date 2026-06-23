//! Git sync commands.

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SyncStatusDto {
    pub status: String,
    pub branch: Option<String>,
    pub ahead: usize,
    pub behind: usize,
    pub conflicts: Vec<String>,
}

#[tauri::command]
pub async fn get_sync_status(
    state: tauri::State<'_, crate::commands::vault::AppState>,
) -> Result<SyncStatusDto, String> {
    let state = state.lock().map_err(|e| e.to_string())?;

    match pkm_sync::git::GitEngine::init(&state.vault_path) {
        Ok(engine) => {
            let status = engine.status().map_err(|e| e.to_string())?;
            let conflicts: Vec<String> = status
                .iter()
                .filter(|(_, s)| s.is_conflicted())
                .map(|(path, _)| path.clone())
                .collect();

            Ok(SyncStatusDto {
                status: if conflicts.is_empty() {
                    "ok".into()
                } else {
                    "conflicts".into()
                },
                branch: Some("main".into()), // Simplified
                ahead: 0,
                behind: 0,
                conflicts,
            })
        }
        Err(_) => Ok(SyncStatusDto {
            status: "no_repo".into(),
            branch: None,
            ahead: 0,
            behind: 0,
            conflicts: Vec::new(),
        }),
    }
}

#[tauri::command]
pub async fn sync_vault(
    state: tauri::State<'_, crate::commands::vault::AppState>,
) -> Result<SyncStatusDto, String> {
    let state = state.lock().map_err(|e| e.to_string())?;

    let engine = pkm_sync::git::GitEngine::init(&state.vault_path).map_err(|e| e.to_string())?;

    // Stage all changes
    engine.add(&["."]).map_err(|e| e.to_string())?;

    // Commit
    let _ = engine
        .commit("Sync from Stratum", "Stratum <stratum@local>")
        .map_err(|e| e.to_string())?;

    // Try push
    let _ = engine.push("origin", "main");

    // Try pull
    let _ = engine.pull("origin", "main");

    let status = engine.status().map_err(|e| e.to_string())?;
    let conflicts: Vec<String> = status
        .iter()
        .filter(|(_, s)| s.is_conflicted())
        .map(|(path, _)| path.clone())
        .collect();

    Ok(SyncStatusDto {
        status: if conflicts.is_empty() {
            "ok".into()
        } else {
            "conflicts".into()
        },
        branch: Some("main".into()),
        ahead: 0,
        behind: 0,
        conflicts,
    })
}
