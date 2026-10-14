//! Cloud/folder backup commands (E10.F3).
//!
//! Thin glue over `pkm_sync::backup`: a manual `run_backup` trigger, a
//! `get_backup_status` status read, and a `test_webdav` connectivity check.
//! The ledger is persisted to `<vault>/.pkm/backup_state.json` so the status
//! survives restarts; backup remains fully independent of git state.

use crate::commands::vault::AppState;
use pkm_core::BackupMode;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Status reported to the frontend after a backup run or from the ledger.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupStatusDto {
    pub enabled: bool,
    /// `"folder"` | `"webdav"` | `"both"`
    pub mode: String,
    pub last_run: Option<String>,
    pub last_success: Option<bool>,
    pub last_error: Option<String>,
    pub files_copied: Option<usize>,
    pub files_deleted: Option<usize>,
}

/// On-disk backup ledger stored in `<vault>/.pkm/backup_state.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BackupStateFile {
    pub version: u32,
    pub last_run: Option<String>,
    pub last_success: Option<bool>,
    pub last_error: Option<String>,
    pub files_copied: Option<usize>,
    pub files_deleted: Option<usize>,
    pub target: Option<String>,
    pub mode: Option<String>,
}

fn backup_state_path(vault_path: &std::path::Path) -> PathBuf {
    vault_path.join(".pkm").join("backup_state.json")
}

fn mode_str(mode: BackupMode) -> &'static str {
    match mode {
        BackupMode::Folder => "folder",
        BackupMode::Webdav => "webdav",
        BackupMode::Both => "both",
    }
}

fn read_backup_state(vault_path: &std::path::Path) -> Option<BackupStateFile> {
    let content = std::fs::read_to_string(backup_state_path(vault_path)).ok()?;
    serde_json::from_str(&content).ok()
}

fn write_backup_state(vault_path: &std::path::Path, result: &pkm_sync::BackupResult, mode: &str) {
    let state = BackupStateFile {
        version: 1,
        last_run: Some(result.finished_at.to_rfc3339()),
        last_success: Some(result.success),
        last_error: result.error.clone(),
        files_copied: Some(result.files_copied),
        files_deleted: Some(result.files_deleted),
        target: Some(result.target.clone()),
        mode: Some(mode.to_string()),
    };
    let path = backup_state_path(vault_path);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(&state) {
        let _ = std::fs::write(&path, json);
    }
}

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

fn dto_from_result(
    backup: &pkm_core::BackupConfig,
    result: &pkm_sync::BackupResult,
) -> BackupStatusDto {
    BackupStatusDto {
        enabled: backup.enabled,
        mode: mode_str(backup.mode).to_string(),
        last_run: Some(result.finished_at.to_rfc3339()),
        last_success: Some(result.success),
        last_error: result.error.clone(),
        files_copied: Some(result.files_copied),
        files_deleted: Some(result.files_deleted),
    }
}

/// Run a backup now, regardless of whether the scheduler is enabled.
#[tauri::command]
pub async fn run_backup(state: tauri::State<'_, AppState>) -> Result<BackupStatusDto, String> {
    // Clone the path and drop the lock before doing any I/O.
    let vault_path = state.lock().map_err(|e| e.to_string())?.vault_path.clone();
    let config = read_config(&vault_path)?;
    let backup = config.sync.backup.clone();

    let scheduler = pkm_sync::BackupScheduler::new(vault_path.clone(), backup.clone());
    let result = scheduler.run_now();
    write_backup_state(&vault_path, &result, mode_str(backup.mode));
    Ok(dto_from_result(&backup, &result))
}

/// Read the backup status, returning defaults when no run has happened yet.
#[tauri::command]
pub async fn get_backup_status(
    state: tauri::State<'_, AppState>,
) -> Result<BackupStatusDto, String> {
    let vault_path = state.lock().map_err(|e| e.to_string())?.vault_path.clone();
    let config = read_config(&vault_path).unwrap_or_default();
    let backup = config.sync.backup;
    let saved = read_backup_state(&vault_path);
    Ok(BackupStatusDto {
        enabled: backup.enabled,
        mode: mode_str(backup.mode).to_string(),
        last_run: saved.as_ref().and_then(|s| s.last_run.clone()),
        last_success: saved.as_ref().and_then(|s| s.last_success),
        last_error: saved.as_ref().and_then(|s| s.last_error.clone()),
        files_copied: saved.as_ref().and_then(|s| s.files_copied),
        files_deleted: saved.as_ref().and_then(|s| s.files_deleted),
    })
}

/// Test connectivity + auth against a WebDAV endpoint. No vault lock is needed.
#[tauri::command]
pub async fn test_webdav(
    url: String,
    username: Option<String>,
    password: Option<String>,
) -> Result<String, String> {
    pkm_sync::backup::webdav::test_webdav_connection(&url, username.as_deref(), password.as_deref())
        .await
        .map(|_| "ok".to_string())
        .map_err(|e| e.to_string())
}
