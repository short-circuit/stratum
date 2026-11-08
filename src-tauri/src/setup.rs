//! Application boot: vault bootstrap, background schedulers, plugin scan,
//! reminder scheduler and tray initialization.
//!
//! Extracted from `lib.rs` to keep the entry point small. All business logic is
//! invoked through [`stratum_app`]; this module only wires the Tauri runtime
//! (managed state, watcher callbacks, notifications, tray).

use stratum_app::StratumService;
use tauri::Emitter;
use tauri::Manager;
use tauri_plugin_notification::NotificationExt;

/// Map a block id string to a stable 32-bit hash for use as a notification id.
fn reminder_hash_to_i32(s: &str) -> i32 {
    let mut h: i32 = 0;
    for b in s.bytes() {
        h = h.wrapping_mul(31).wrapping_add(b as i32);
    }
    h
}

/// Tauri `setup` hook body.
pub fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let vault_path = crate::resolve_default_vault_path(app.handle());

    let _ = std::fs::create_dir_all(&vault_path);
    let _ = std::fs::create_dir_all(vault_path.join(".pkm"));

    let db_path = vault_path.join(".pkm").join("blocks.db");
    match crate::commands::page::sync_filesystem_to_db(&vault_path, &db_path) {
        Ok(n) if n > 0 => {
            tracing::info!("[stratum] Synced {} existing pages from filesystem", n)
        }
        Err(e) => tracing::warn!("[stratum] Filesystem sync skipped: {}", e),
        _ => {}
    }

    app.manage(StratumService::new(vault_path.clone()));

    // ── WASM plugins (spec §7.3 startup scan) ────────────────────
    // Initialize the plugin manager from `<vault>/.pkm/plugins/` on
    // boot so plugins are loaded when the app starts, not only after
    // an explicit vault re-init. A broken or missing plugins dir is
    // never fatal: scan failures surface through `plugins_list`.
    // Notably the path is shared with `setup_vault` (interactive open).
    {
        if let Ok(mut state) = app.state::<StratumService>().lock() {
            match stratum_app::plugins::PluginManager::init_for_vault(&vault_path) {
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
                let auto_commit =
                    pkm_sync::AutoCommitEngine::new(git, config.sync.auto_commit_interval_secs);
                if let Ok(mut state) = app.state::<StratumService>().lock() {
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
                if let Ok(mut state) = app.state::<StratumService>().lock() {
                    state.sync_scheduler = Some(scheduler);
                    tracing::info!("[stratum] Sync scheduler started");
                }
            }
        }

        // ── Cloud/folder backup scheduler (E10.F3) ───────────
        // Backup is a separate layer from git sync: only start the
        // scheduler when the user has explicitly enabled it.
        if config.sync.backup.enabled {
            let mut scheduler =
                pkm_sync::BackupScheduler::new(vault_path.clone(), config.sync.backup.clone());
            scheduler.start();
            if let Ok(mut state) = app.state::<StratumService>().lock() {
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
                let state_guard = app_handle.state::<StratumService>();
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
                    if let Ok(elapsed) = event.timestamp.duration_since(state.watcher_last_save) {
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
                        let state_guard = app_handle.state::<StratumService>();
                        let mut state = match state_guard.lock() {
                            Ok(s) => s,
                            Err(_) => return,
                        };
                        let store = match state.get_store() {
                            Ok(s) => s,
                            Err(_) => return,
                        };
                        let vp = state.vault_path.clone();
                        // Sync page data into SQLite; if the file is
                        // gone (old-path half of a rename, or a delete
                        // surfaced as Modify) this deletes the stale
                        // rows instead of failing on the missing file.
                        let synced = crate::commands::page::handle_renamed_or_created(
                            &store, &rel, &vp, None,
                        );
                        // Drop BlockStore and cached BlockIndex before
                        // accessing IndexEngine (same Tantivy dir).
                        drop(store);
                        drop(state.block_index.take());
                        if let Ok(ie) = state.ensure_index() {
                            match synced {
                                // File is gone: purge search index
                                // (mirrors the Deleted arm).
                                Ok(false) => {
                                    tracing::debug!(
                                        "[stratum] watcher: {} gone, purging stale rows + index",
                                        rel
                                    );
                                    let _ = ie.remove_note(&rel);
                                }
                                // File present (synced) or transient
                                // read error: refresh as before.
                                Ok(true) | Err(_) => {
                                    let _ = ie.refresh_page(&rel, &vp);
                                }
                            }
                        }
                    }
                    pkm_core::FileEvent::Deleted => {
                        let state_guard = app_handle.state::<StratumService>();
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
                    if let Ok(mut state) = app.state::<StratumService>().lock() {
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
                std::collections::HashMap<String, stratum_app::reminders::DueReminder>,
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
                if let Some(block_id) = n.extra().get("block_id").and_then(|v| v.as_str()) {
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

        if let Ok(state) = app.state::<StratumService>().lock() {
            if let Ok(store) = state.get_store() {
                let notify_app = app.handle().clone();
                let notify_last_due = last_due.clone();
                let with_sound = config.reminders.sound;
                let notify: stratum_app::reminders::NotifyFn = Box::new(move |reminder| {
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
                    let payload = serde_json::to_string(reminder).unwrap_or_default();
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
                let scheduler = stratum_app::reminders::ReminderScheduler::new(
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
}
