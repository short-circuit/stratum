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
    let builder = tauri::Builder::default().plugin(tauri_plugin_dialog::init());

    #[cfg(target_os = "android")]
    let builder = builder.plugin(tauri_plugin_android_fs::init());

    builder
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

            app.manage(Mutex::new(VaultState::new(vault_path.clone())) as AppState);

            let config_path = vault_path.join(".pkm").join("config.toml");
            if config_path.exists() {
                if let Ok(config) = pkm_core::Config::load(&config_path) {
                    if config.sync.mode == pkm_core::SyncMode::AutoCommit
                        || config.sync.mode == pkm_core::SyncMode::AutoSync
                        || config.sync.mode == pkm_core::SyncMode::Background
                    {
                        if let Ok(git) = pkm_sync::git::GitEngine::init(&vault_path) {
                            let auto_commit = pkm_sync::AutoCommitEngine::new(
                                git,
                                config.sync.auto_commit_interval_secs,
                            );
                            if let Ok(mut state) = app.state::<AppState>().lock() {
                                state.auto_commit_engine = Some(auto_commit);
                                eprintln!(
                                    "[stratum] Auto-commit engine initialized (interval={}s)",
                                    config.sync.auto_commit_interval_secs
                                );
                            }
                        }
                    }

                    if config.sync.mode == pkm_core::SyncMode::AutoSync
                        || config.sync.mode == pkm_core::SyncMode::Background
                    {
                        if let Ok(git) = pkm_sync::git::GitEngine::init(&vault_path) {
                            let sched_config = pkm_sync::SchedulerConfig {
                                remote: "origin".to_string(),
                                branch: config.sync.branch.clone(),
                                interval_secs: config.sync.auto_sync_interval_secs,
                                ssh_key_path: config
                                    .sync
                                    .ssh_key_path
                                    .clone()
                                    .map(std::path::PathBuf::from),
                            };
                            let mut scheduler = pkm_sync::SyncScheduler::new(git, sched_config);
                            scheduler.start();
                            if let Ok(mut state) = app.state::<AppState>().lock() {
                                state.sync_scheduler = Some(scheduler);
                                eprintln!("[stratum] Sync scheduler started");
                            }
                        }
                    }

                    // ── File watcher ──────────────────────────────────────
                    if config.watcher.enabled {
                        let app_handle = app.handle().clone();
                        let vault_path_clone = vault_path.clone();
                        let on_event = Box::new(move |event: pkm_watcher::FileChangeEvent| {
                            // Skip events from our own recent saves (avoid reindex loops)
                            let state_guard = app_handle.state::<AppState>();
                            let state = match state_guard.lock() {
                                Ok(s) => s,
                                Err(_) => return,
                            };
                            if state.watcher_last_save != std::time::SystemTime::UNIX_EPOCH {
                                if let Ok(elapsed) =
                                    event.timestamp.duration_since(state.watcher_last_save)
                                {
                                    if elapsed.as_millis() < 2000 {
                                        return;
                                    }
                                }
                            }
                            drop(state);

                            let rel = match event.path.strip_prefix(&vault_path_clone) {
                                Ok(r) => r.to_string_lossy().to_string(),
                                Err(_) => return,
                            };

                            match event.kind {
                                pkm_core::FileEvent::Created
                                | pkm_core::FileEvent::Modified
                                | pkm_core::FileEvent::Renamed => {
                                    let state_guard = app_handle.state::<AppState>();
                                    let mut state = match state_guard.lock() {
                                        Ok(s) => s,
                                        Err(_) => return,
                                    };
                                    let store = match pkm_block::BlockStore::open(&state.db_path) {
                                        Ok(s) => s,
                                        Err(_) => return,
                                    };
                                    let vp = state.vault_path.clone();
                                    // Sync page data into SQLite
                                    let _ = crate::commands::page::sync_page_from_disk(
                                        &store, &rel, &vp, None,
                                    );
                                    // Drop BlockStore and cached BlockIndex before
                                    // accessing IndexEngine (same Tantivy dir).
                                    drop(store);
                                    drop(state.block_index.take());
                                    if let Ok(ie) = state.ensure_index() {
                                        let _ = ie.refresh_page(&rel, &vp);
                                    }
                                }
                                pkm_core::FileEvent::Deleted => {
                                    let state_guard = app_handle.state::<AppState>();
                                    let mut state = match state_guard.lock() {
                                        Ok(s) => s,
                                        Err(_) => return,
                                    };
                                    let store = match pkm_block::BlockStore::open(&state.db_path) {
                                        Ok(s) => s,
                                        Err(_) => return,
                                    };
                                    let _ = store.delete_blocks_by_page(&rel);
                                    let _ = store.delete_page(&rel);
                                    drop(store);
                                    drop(state.block_index.take());
                                    if let Ok(ie) = state.ensure_index() {
                                        let _ = ie.remove_note(&rel);
                                    }
                                }
                            }
                        });

                        let mut watcher = pkm_watcher::FileWatcher::new(
                            vault_path.clone(),
                            config.watcher.debounce_ms,
                            on_event,
                        );
                        match watcher.start() {
                            Ok(()) => {
                                if let Ok(mut state) = app.state::<AppState>().lock() {
                                    state.watcher = Some(watcher);
                                }
                                eprintln!(
                                    "[stratum] File watcher started (debounce={}ms)",
                                    config.watcher.debounce_ms
                                );
                            }
                            Err(e) => {
                                eprintln!("[stratum] Failed to start file watcher: {}", e);
                            }
                        }
                    }
                }
            }

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
            commands::vault::init_vault,
            commands::vault::init_default_vault,
            #[cfg(desktop)]
            commands::vault::pick_vault_directory,
            #[cfg(target_os = "android")]
            commands::vault::pick_android_directory,
            // Pages
            commands::page::list_pages,
            commands::page::open_page,
            commands::page::save_page,
            commands::page::create_page,
            commands::page::ensure_today_journal,
            commands::page::delete_page,
            commands::page::reindex_vault,
            commands::page::reindex_page,
            commands::page::normalize_file,
            commands::page::normalize_all_files,
            // Blocks
            commands::block::get_blocks,
            commands::block::build_markdown,
            commands::block::save_blocks,
            commands::block::update_block,
            commands::block::delete_block,
            commands::block::insert_block,
            commands::block::toggle_block_marker,
            commands::block::clear_block_marker,
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
            commands::graph::get_graph_panel_data,
            commands::graph::resolve_link_target,
            // Query
            commands::query::run_query,
            // Sync
            commands::sync::get_sync_status,
            commands::sync::sync_vault,
            commands::sync::sync_vault_with_passphrase,
            commands::sync::start_sync_scheduler,
            commands::sync::stop_sync_scheduler,
            commands::sync::get_commit_log,
            commands::sync::resolve_conflict_file,
            commands::sync::abort_merge,
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
            // Kanban
            commands::kanban::get_kanban_blocks,
            commands::kanban::create_kanban_block,
            // Whiteboards
            commands::whiteboard::list_whiteboards,
            commands::whiteboard::save_whiteboard,
            commands::whiteboard::load_whiteboard,
            commands::whiteboard::rename_whiteboard,
            commands::whiteboard::delete_whiteboard,
            commands::whiteboard::save_library,
            commands::whiteboard::load_library,
            commands::whiteboard::load_extra_libraries,
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
