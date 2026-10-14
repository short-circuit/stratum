pub mod commands;
pub mod reminder_scheduler;

use commands::vault::{AppState, VaultState};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::Emitter;
use tauri::Manager;
use tauri_plugin_notification::NotificationExt;

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

/// Map a block id string to a stable 32-bit hash for use as a notification id.
fn reminder_hash_to_i32(s: &str) -> i32 {
    let mut h: i32 = 0;
    for b in s.bytes() {
        h = h.wrapping_mul(31).wrapping_add(b as i32);
    }
    h
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
        .setup(|app| {
            let vault_path = resolve_default_vault_path(app.handle());

            let _ = std::fs::create_dir_all(&vault_path);
            let _ = std::fs::create_dir_all(vault_path.join(".pkm"));

            let db_path = vault_path.join(".pkm").join("blocks.db");
            match commands::page::sync_filesystem_to_db(&vault_path, &db_path) {
                Ok(n) if n > 0 => {
                    tracing::info!("[stratum] Synced {} existing pages from filesystem", n)
                }
                Err(e) => tracing::warn!("[stratum] Filesystem sync skipped: {}", e),
                _ => {}
            }

            app.manage(Mutex::new(VaultState::new(vault_path.clone())) as AppState);

            // ── WASM plugins (spec §7.3 startup scan) ────────────────────
            // Initialize the plugin manager from `<vault>/.pkm/plugins/` on
            // boot so plugins are loaded when the app starts, not only after
            // an explicit vault re-init. A broken or missing plugins dir is
            // never fatal: scan failures surface through `plugins_list`.
            // Notably the path is shared with `setup_vault` (interactive open).
            {
                if let Ok(mut state) = app.state::<AppState>().lock() {
                    match crate::commands::plugins::PluginManager::init_for_vault(&vault_path) {
                        Ok(manager) => {
                            tracing::info!(
                                "[stratum] Plugin runtime ready ({} plugin(s) loaded from {:?})",
                                manager.len(),
                                vault_path.join(".pkm").join("plugins")
                            );
                            state.plugin_manager = Some(manager);
                        }
                        Err(e) => {
                            tracing::warn!(
                                "[stratum] Plugin runtime unavailable, plugins disabled: {}",
                                e
                            );
                        }
                    }
                }
            }

            let config_path = vault_path.join(".pkm").join("config.toml");
            // Resolve the effective config: load from `<vault>/.pkm/config.toml`
            // when present, else fall back to code defaults. A vault without a
            // config file is fully supported — every field has a safe default
            // (watcher.enabled defaults to true) — and must still get the
            // auto-commit engine, sync scheduler and file watcher. Previously
            // these were gated behind `config_path.exists()`, silently disabling
            // the file watcher on any default vault (E7.F1).
            let config = if config_path.exists() {
                pkm_core::Config::load(&config_path)
                    .or_else(|e| {
                        tracing::warn!(
                            "[stratum] Unreadable config at {} ({}); using defaults",
                            config_path.display(),
                            e
                        );
                        Ok::<_, std::convert::Infallible>(pkm_core::Config::default())
                    })
                    .unwrap_or_default()
            } else {
                pkm_core::Config::default()
            };
            {
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
                            tracing::info!(
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
                            tracing::info!("[stratum] Sync scheduler started");
                        }
                    }
                }

                // ── Cloud/folder backup scheduler (E10.F3) ───────────
                // Backup is a separate layer from git sync: only start the
                // scheduler when the user has explicitly enabled it.
                if config.sync.backup.enabled {
                    let mut scheduler = pkm_sync::BackupScheduler::new(
                        vault_path.clone(),
                        config.sync.backup.clone(),
                    );
                    scheduler.start();
                    if let Ok(mut state) = app.state::<AppState>().lock() {
                        state.backup_scheduler = Some(scheduler);
                        tracing::info!("[stratum] Backup scheduler started");
                    }
                }

                // ── File watcher ──────────────────────────────────────
                #[cfg(not(target_os = "android"))]
                if config.watcher.enabled {
                    let app_handle = app.handle().clone();
                    let vault_path_clone = vault_path.clone();
                    let on_event = Box::new(move |event: pkm_watcher::FileChangeEvent| {
                        tracing::debug!(
                            "[stratum] watcher event: kind={:?} path={}",
                            event.kind,
                            event.path.display()
                        );
                        // Skip events from our own recent saves (avoid reindex loops)
                        let state_guard = app_handle.state::<AppState>();
                        let state = match state_guard.lock() {
                            Ok(s) => s,
                            Err(_) => {
                                tracing::warn!("[stratum] watcher: state lock poisoned");
                                return;
                            }
                        };
                        // Skip events during bulk operations (normalize/reindex)
                        if state.is_indexing() {
                            tracing::warn!("[stratum] watcher: skipped while indexing");
                            return;
                        }
                        if state.watcher_last_save != std::time::SystemTime::UNIX_EPOCH {
                            if let Ok(elapsed) =
                                event.timestamp.duration_since(state.watcher_last_save)
                            {
                                if elapsed.as_millis() < 2000 {
                                    tracing::debug!(
                                        "[stratum] watcher: skipped own-save window ({}ms)",
                                        elapsed.as_millis()
                                    );
                                    return;
                                }
                            }
                        }
                        drop(state);

                        let rel = match event.path.strip_prefix(&vault_path_clone) {
                            Ok(r) => r.to_string_lossy().to_string(),
                            Err(_) => {
                                tracing::warn!(
                                    "[stratum] watcher: path outside vault {}",
                                    event.path.display()
                                );
                                return;
                            }
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
                                let store = match state.get_store() {
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
                                let store = match state.get_store() {
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

                        // The block store (and thus the recents list) changed.
                        // Emit so the frontend can refresh; the recents store
                        // debounces bursts, so a git pull / external edit storm
                        // collapses into a single reload.
                        let _ = app_handle.emit("pages-changed", ());
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
                            tracing::info!(
                                "[stratum] File watcher started (debounce={}ms)",
                                config.watcher.debounce_ms
                            );
                        }
                        Err(e) => {
                            tracing::error!("[stratum] Failed to start file watcher: {}", e);
                        }
                    }
                }
            }

            // ── Block reminders ────────────────────────────────────────
            // Runs an in-process scheduler (no daemon). It scans for due
            // reminders; when enabled it also registers a notification tap
            // handler and emits a `reminder:due` event for the in-app toast.
            if config.reminders.enabled {
                let app_handle = app.handle().clone();
                let last_due: std::sync::Arc<
                    std::sync::Mutex<
                        std::collections::HashMap<String, crate::reminder_scheduler::DueReminder>,
                    >,
                > = std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()));

                // Register the tap handler BEFORE any notification is shown:
                // the plugin only reports actions for notifications shown after
                // a handler is registered.
                let tap_app = app_handle.clone();
                let tap_last_due = last_due.clone();
                if let Err(e) = app_handle.notification().on_action(move |performed| {
                    if performed.action_id() != "tap" {
                        return;
                    }
                    // Show + focus the main window so the user sees the highlight.
                    if let Some(w) = tap_app.get_webview_window("main") {
                        let _ = w.show();
                        let _ = w.set_focus();
                    }
                    if let Some(n) = performed.notification() {
                        if let Some(block_id) =
                            n.extra().get("block_id").and_then(|v| v.as_str())
                        {
                            if let Ok(map) = tap_last_due.lock() {
                                if let Some(rem) = map.get(block_id) {
                                    let _ = tap_app.emit("reminder:open", rem.clone());
                                }
                            }
                        }
                    }
                }) {
                    tracing::warn!("[stratum] notification on_action registration failed: {e}");
                }

                if let Ok(state) = app.state::<AppState>().lock() {
                    if let Ok(store) = state.get_store() {
                        let notify_app = app.handle().clone();
                        let notify_last_due = last_due.clone();
                        let with_sound = config.reminders.sound;
                        let notify: crate::reminder_scheduler::NotifyFn =
                            Box::new(move |reminder| {
                                if let Ok(mut map) = notify_last_due.lock() {
                                    map.insert(reminder.block_id.clone(), reminder.clone());
                                }
                                // Body: line-wrapped content summary capped ~200
                                // chars, prefixed with the page title when known.
                                let mut body = reminder.content.replace('\n', " ");
                                if let Some(title) = &reminder.page_title {
                                    body = format!("{} — {}", title, body);
                                }
                                if body.chars().count() > 200 {
                                    body = body.chars().take(200).collect::<String>();
                                }
                                let payload =
                                    serde_json::to_string(reminder).unwrap_or_default();
                                let mut builder = notify_app
                                    .notification()
                                    .builder()
                                    .id(reminder_hash_to_i32(&reminder.block_id))
                                    .title("Stratum — Reminder")
                                    .body(body)
                                    .extra("reminder_payload", payload)
                                    .extra("block_id", reminder.block_id.clone());
                                if with_sound {
                                    // Default system notification sound (desktop).
                                    builder = builder.sound("default");
                                }
                                let _ = builder.show();
                                // In-app toast + refresh signal for the frontend.
                                let _ = notify_app.emit("reminder:due", reminder.clone());
                            });
                        let interval = std::time::Duration::from_secs(30);
                        let scheduler = crate::reminder_scheduler::ReminderScheduler::new(
                            store,
                            interval,
                            Box::new(chrono::Utc::now),
                            notify,
                        );
                        tauri::async_runtime::spawn(scheduler.run());
                        tracing::info!("[stratum] Reminder scheduler started");
                    }
                }
            }

            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        // The app installs its own global logger via
                        // `pkm_core::init_logging()` (tracing_subscriber, which
                        // forwards to the `log` crate). Skip the plugin's logger
                        // so it does not panic with "attempted to set a logger
                        // after the logging system was already initialized".
                        .skip_logger()
                        .build(),
                )?;
            }

            // ── System tray + global quick-capture shortcut (desktop) ─────
            #[cfg(desktop)]
            {
                if let Err(e) = crate::commands::tray::init(app.handle(), &config.tray) {
                    tracing::warn!("[stratum] Tray init failed: {}", e);
                }
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
            commands::page::repair_db_from_disk,
            commands::page::normalize_file,
            commands::page::normalize_all_files,
            // Importers
            commands::import::preview_import,
            commands::import::import_vault,
            // Assets
            commands::asset::import_asset_path,
            commands::asset::import_asset_bytes,
            commands::asset::read_asset,
            commands::asset::delete_asset,
            commands::asset::list_asset_refs,
            commands::asset::get_page_asset_refs,
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
            commands::search::get_backlink_snippet,
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
            commands::query::list_saved_queries,
            commands::query::save_saved_query,
            commands::query::rename_saved_query,
            commands::query::delete_saved_query,
            // Sync
            commands::sync::get_sync_status,
            commands::sync::sync_vault,
            commands::sync::sync_vault_with_passphrase,
            commands::sync::start_sync_scheduler,
            commands::sync::stop_sync_scheduler,
            commands::sync::get_commit_log,
            commands::sync::resolve_conflict_file,
            commands::sync::abort_merge,
            // Backup (E10.F3)
            commands::backup::run_backup,
            commands::backup::get_backup_status,
            commands::backup::test_webdav,
            // Templates
            commands::template::list_templates,
            commands::template::save_template,
            commands::template::apply_template,
            commands::template::install_starter_templates,
            commands::template::install_starter_sample,
            // Export
            commands::export::export_html,
            commands::export::export_json,
            commands::export::pdf::export_pdf,
            commands::export::zip::export_zip,
            // Flashcards
            commands::flashcards::generate_flashcards,
            commands::flashcards::generate_cards_from_page,
            commands::flashcards::review_card,
            // Kanban
            commands::kanban::get_kanban_blocks,
            commands::kanban::create_kanban_block,
            // Reminders
            commands::reminders::list_reminders,
            commands::reminders::snooze_reminder,
            commands::reminders::clear_reminder,
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
            commands::ai::ai_rag_query,
            commands::ai::discover_local_ai,
            commands::ai::ping_search_instance,
            // Dictation
            commands::dictation::dictation_start,
            commands::dictation::dictation_stop,
            commands::dictation::dictation_cancel,
            commands::dictation::dictation_transcribe,
            commands::dictation::dictation_insert_clip_ref,
            commands::dictation::mic_permission_status,
            commands::dictation::request_mic_permission,
            commands::dictation::speaker_list,
            commands::dictation::speaker_assign,
            commands::dictation::speaker_delete,
            commands::dictation::stt_test_connection,
            // TTS
            commands::tts::tts_synthesize,
            commands::tts::tts_speak,
            // Settings
            commands::settings::get_settings,
            commands::settings::save_settings,
            commands::settings::save_graph_settings,
            commands::settings::fetch_models,
            // Tray / quick capture
            #[cfg(desktop)]
            commands::tray::get_tray_status,
            #[cfg(desktop)]
            commands::tray::quick_capture_save,
            // Onboarding
            commands::settings::get_onboarding_state,
            commands::settings::mark_onboarding_done,
            commands::settings::set_onboarding_pending,
            // Plugins
            commands::plugins::plugins_list,
            commands::plugins::plugins_enable,
            commands::plugins::plugins_disable,
            commands::plugins::plugins_reload,
            commands::plugins::plugins_status,
            commands::plugins::plugins_install,
            commands::plugins::plugins_uninstall,
            commands::plugins::plugin_note_read,
            commands::plugins::plugin_http_request,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
