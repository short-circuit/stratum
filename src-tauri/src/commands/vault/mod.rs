//! Vault management command adapters.
//!
//! Thin adapters over [`stratum_app::vault`]. The shared state type is
//! [`stratum_app::StratumService`]; `VaultState` and `IndexingGuard` are
//! re-exported so the rest of the Tauri tree keeps its original import paths.

use stratum_app::StratumService;

pub use stratum_app::state::{ActiveRecording, DictationSession, IndexingGuard, VaultState};
pub use stratum_app::vault::{
    percent_decode, resolve_picked_path, resolve_saf_content_uri, VaultInfo,
};

/// Legacy alias retained for modules that still import `commands::vault::AppState`.
pub type AppState = StratumService;

#[tauri::command]
pub async fn get_vault_info(state: tauri::State<'_, StratumService>) -> Result<VaultInfo, String> {
    state.get_vault_info().await
}

#[tauri::command]
pub async fn set_vault_path(
    path: String,
    state: tauri::State<'_, StratumService>,
) -> Result<(), String> {
    state.set_vault_path(path).await
}

/// Initialize vault at a user-picked path.
/// Frontend calls `open({ directory: true })` and passes the result here.
#[tauri::command]
pub async fn init_vault(
    path: String,
    state: tauri::State<'_, StratumService>,
) -> Result<VaultInfo, String> {
    state.init_vault(path).await
}

/// Initialize vault at the application's default data directory
/// (app-private storage on Android, ~/StratumVault on desktop).
/// This is the command called on mobile instead of the folder picker.
#[tauri::command]
pub async fn init_default_vault(
    app: tauri::AppHandle,
    state: tauri::State<'_, StratumService>,
) -> Result<VaultInfo, String> {
    let vault_path = crate::resolve_default_vault_path(&app);
    std::fs::create_dir_all(&vault_path)
        .map_err(|e| format!("Failed to create vault directory: {}", e))?;
    state.init_vault_at(vault_path).await
}

/// Pick a directory on Android via SAF, then init vault at that location.
/// Requires MANAGE_EXTERNAL_STORAGE permission for raw file access.
#[cfg(target_os = "android")]
#[tauri::command]
pub async fn pick_android_directory(
    app: tauri::AppHandle,
    state: tauri::State<'_, StratumService>,
) -> Result<VaultInfo, String> {
    use tauri_plugin_android_fs::AndroidFsExt;

    let api = app.android_fs_async();
    let selected = api
        .picker()
        .pick_dir(None, false)
        .await
        .map_err(|e| format!("Folder picker failed: {}", e))?
        .ok_or("No folder selected".to_string())?;

    api.picker().persist_uri_permission(&selected).await.ok();

    // Try MANAGE_EXTERNAL_STORAGE raw path first, fall back to private storage
    let vault_path = match resolve_picked_path(&selected.uri) {
        Ok(p) if p.exists() || std::fs::create_dir_all(&p).is_ok() => p,
        _ => {
            // Fall back to default private path
            std::path::PathBuf::from("/data/user/0/app.stratum/StratumVault")
        }
    };

    state.init_vault_at(vault_path).await
}

#[cfg(desktop)]
#[tauri::command]
pub async fn pick_vault_directory(
    app: tauri::AppHandle,
    state: tauri::State<'_, StratumService>,
) -> Result<VaultInfo, String> {
    use tauri_plugin_dialog::DialogExt;

    let (tx, rx) = std::sync::mpsc::channel();

    app.dialog().file().pick_folder(move |path| {
        let _ = tx.send(path);
    });

    let picked = rx
        .recv()
        .map_err(|_| "Folder picker cancelled".to_string())?;
    let picked_path = picked.ok_or("No folder selected".to_string())?;

    let vault_path = std::path::PathBuf::from(picked_path.to_string());
    if !vault_path.exists() {
        return Err(format!(
            "Selected path does not exist: {}",
            vault_path.display()
        ));
    }

    state.init_vault_at(vault_path).await
}

#[cfg(test)]
mod tests;
