mod commands;

use commands::vault::{AppState, VaultState};
use std::path::PathBuf;
use std::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let default_vault = dirs::home_dir()
        .map(|h| h.join("StratumVault"))
        .or_else(|| {
            std::env::current_dir()
                .ok()
                .map(|d| d.join("vault"))
        });

    let vault_path = default_vault.unwrap_or_else(|| PathBuf::from("vault"));
    // Ensure vault directory exists
    let _ = std::fs::create_dir_all(&vault_path);
    let _ = std::fs::create_dir_all(vault_path.join(".pkm"));

    tauri::Builder::default()
        .manage(Mutex::new(VaultState::new(vault_path)) as AppState)
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Vault
            commands::vault::get_vault_info,
            commands::vault::set_vault_path,
            // Pages
            commands::page::list_pages,
            commands::page::open_page,
            commands::page::save_page,
            commands::page::create_page,
            commands::page::delete_page,
            // Blocks
            commands::block::get_blocks,
            commands::block::build_markdown,
            commands::block::update_block,
            commands::block::delete_block,
            commands::block::insert_block,
            commands::block::get_block_properties,
            commands::block::set_block_property,
            // Search
            commands::search::search_blocks,
            commands::search::get_backlinks,
            commands::search::get_page_backlinks,
            commands::search::autocomplete,
            // Query
            commands::query::run_query,
            // Sync
            commands::sync::get_sync_status,
            commands::sync::sync_vault,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
