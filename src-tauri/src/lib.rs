//! Stratum desktop/mobile shell.
//!
//! This crate is a thin Tauri host: every command delegates to
//! [`stratum_app::StratumService`], boot logic lives in [`setup`], and the IPC
//! registry in [`invoke`]. Business logic lives in `stratum-app`.

pub mod commands;
pub mod invoke;
pub mod setup;

use std::path::PathBuf;
use tauri::Manager;

fn resolve_default_vault_path(_app: &tauri::AppHandle) -> PathBuf {
    #[cfg(target_os = "android")]
    {
        _app.path()
            .app_data_dir()
            .unwrap_or_else(|_| PathBuf::from("vault"))
            .join("StratumVault")
    }

    #[cfg(not(target_os = "android"))]
    {
        dirs::home_dir()
            .map(|h| h.join("StratumVault"))
            .or_else(|| std::env::current_dir().ok().map(|d| d.join("vault")))
            .unwrap_or_else(|| PathBuf::from("vault"))
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _ = pkm_core::init_logging();
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init());

    // Single-instance must be registered before any other plugin that can open
    // a second window (notably dialog). Empty crates on mobile — desktop only.
    // On a second launch it shows + focuses the existing main window instead of
    // starting a new process.
    #[cfg(desktop)]
    let builder = builder
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build());

    #[cfg(target_os = "android")]
    let builder = builder.plugin(tauri_plugin_android_fs::init());

    builder
        .setup(setup::setup)
        .invoke_handler(invoke::invoke_handler!())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
