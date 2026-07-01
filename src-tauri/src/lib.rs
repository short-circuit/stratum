mod commands;

use commands::vault::{AppState, VaultState};
use std::path::PathBuf;
use std::sync::Mutex;
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
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let vault_path = resolve_default_vault_path(app.handle());

            let _ = std::fs::create_dir_all(&vault_path);
            let _ = std::fs::create_dir_all(vault_path.join(".pkm"));

            let db_path = vault_path.join(".pkm").join("blocks.db");
            match commands::page::sync_filesystem_to_db(&vault_path, &db_path) {
                Ok(n) if n > 0 => {
                    eprintln!("[stratum] Synced {} existing pages from filesystem", n)
                }
                Err(e) => eprintln!("[stratum] Filesystem sync skipped: {}", e),
                _ => {}
            }

            app.manage(Mutex::new(VaultState::new(vault_path)) as AppState);

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
            commands::vault::pick_vault_directory,
            // Pages
            commands::page::list_pages,
            commands::page::open_page,
            commands::page::save_page,
            commands::page::create_page,
            commands::page::delete_page,
            commands::page::reindex_vault,
            commands::page::reindex_page,
            // Blocks
            commands::block::get_blocks,
            commands::block::build_markdown,
            commands::block::save_blocks,
            commands::block::update_block,
            commands::block::delete_block,
            commands::block::insert_block,
            // Search
            commands::search::search_blocks,
            commands::search::search_by_tag,
            commands::search::rebuild_search_index,
            commands::search::get_backlinks,
            commands::search::get_page_backlinks,
            commands::search::autocomplete,
            commands::search::suggest_connections,
            commands::search::get_backlink_context,
            // Graph
            commands::graph::get_graph_data,
            commands::graph::get_connected_components,
            commands::graph::get_orphaned_notes,
            commands::graph::rebuild_graph,
            commands::graph::resolve_link_target,
            // Query
            commands::query::run_query,
            // Sync
            commands::sync::get_sync_status,
            commands::sync::sync_vault,
            // Templates
            commands::template::list_templates,
            commands::template::save_template,
            commands::template::apply_template,
            // Export
            commands::export::export_html,
            commands::export::export_json,
            // Flashcards
            commands::flashcards::generate_flashcards,
            commands::flashcards::generate_cards_from_page,
            commands::flashcards::review_card,
            // Whiteboards
            commands::whiteboard::list_whiteboards,
            commands::whiteboard::save_whiteboard,
            commands::whiteboard::load_whiteboard,
            commands::whiteboard::save_library,
            commands::whiteboard::load_library,
            // AI
            commands::ai::ai_transform_block,
            commands::ai::ai_research,
            commands::ai::ai_interlink_notes,
            commands::ai::generate_mermaid,
            // Settings
            commands::settings::get_settings,
            commands::settings::save_settings,
            commands::settings::save_graph_settings,
            commands::settings::fetch_models,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
