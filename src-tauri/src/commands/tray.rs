//! System tray + Quick Capture.
//!
//! Wire the desktop tray (defined in `tauri.conf.json`, id "main") with its
//! menu, register the global Quick Capture shortcut, and expose the
//! `get_tray_status` / `quick_capture_save` IPC commands that the frontend
//! Quick Capture dialog and Tray settings tab use.
//!
//! The tray itself is desktop-only: the menu/shortcut registration is behind
//! `#[cfg(desktop)]`, while the two commands compile and run on every platform
//! (mobile simply reports the tray as disabled). `quick_capture_save` performs
//! its own persistence — store insert, `.md` write and search-index update —
//! rather than relying on the file watcher.

use crate::commands::sync::sync_vault_core;
use crate::commands::vault::{AppState, VaultState};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tracing::warn;

// ---------------------------------------------------------------------------
// DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrayStatusDto {
    pub enabled: bool,
    pub close_to_tray: bool,
    pub shortcut: String,
    pub capture_target: String,
    pub inbox_path: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QuickCaptureResultDto {
    pub page_path: String,
    pub block_id: String,
}

// ---------------------------------------------------------------------------
// Desktop tray + shortcut
// ---------------------------------------------------------------------------

/// Initialize the system tray and register the global Quick Capture shortcut.
///
/// Tray is defined in `tauri.conf.json` (id "main"); if it is absent this falls
/// back to building one with the default window icon. When `tray.enabled` is
/// false the tray is hidden and no shortcut is registered. Registration of a
/// bad/held shortcut logs a warning rather than failing.
#[cfg(desktop)]
pub fn init<R: Runtime>(app: &AppHandle<R>, tray: &pkm_core::TrayConfig) -> tauri::Result<()> {
    if !tray.enabled {
        if let Some(tray_icon) = app.tray_by_id("main") {
            let _ = tray_icon.set_visible(false);
        }
        return Ok(());
    }

    let tray_icon = if let Some(existing) = app.tray_by_id("main") {
        existing
    } else {
        let mut builder = tauri::tray::TrayIconBuilder::with_id("main");
        if let Some(icon) = app.default_window_icon().cloned() {
            builder = builder.icon(icon);
        }
        builder.build(app)?
    };

    let menu = build_menu(app)?;
    tray_icon.set_menu(Some(menu))?;
    tray_icon.on_menu_event(|app, event| match event.id().as_ref() {
        "open" => show_main_window(app),
        "quick_capture" => open_quick_capture(app, false),
        "voice_capture" => open_quick_capture(app, true),
        "sync_now" => spawn_sync_task(app),
        "quit" => quit(app),
        _ => {}
    });
    tray_icon.on_tray_icon_event(|tray, event| {
        if let tauri::tray::TrayIconEvent::DoubleClick { .. } = event {
            show_main_window(tray.app_handle());
        }
    });

    if let Some(shortcuts) = app.try_state::<tauri_plugin_global_shortcut::GlobalShortcut<R>>() {
        register_shortcut(shortcuts.inner(), &tray.quick_capture_shortcut);
    }
    Ok(())
}

/// Apply a tray config change at runtime, idempotently.
///
/// Hides/shows the tray and re-registers the global shortcut to the new binding.
/// Never panics on a bad shortcut string. No-ops (the tray and shortcut do not
/// exist) when called from a context without the tray/plugin, e.g. mock tests.
pub fn apply_tray_state<R: Runtime>(app: &AppHandle<R>, tray: &pkm_core::TrayConfig) {
    #[cfg(desktop)]
    {
        if let Some(tray_icon) = app.tray_by_id("main") {
            let _ = tray_icon.set_visible(tray.enabled);
        }
        if let Some(shortcuts) = app.try_state::<tauri_plugin_global_shortcut::GlobalShortcut<R>>()
        {
            let _ = shortcuts.unregister_all();
            if tray.enabled {
                register_shortcut(shortcuts.inner(), &tray.quick_capture_shortcut);
            }
        }
    }
    #[cfg(not(desktop))]
    {
        let _ = (app, tray);
    }
}

/// Show, unminimize and focus the main window.
pub fn show_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

/// Show the main window and emit a `quick-capture:open` event with `{ voice }`.
pub fn open_quick_capture<R: Runtime>(app: &AppHandle<R>, voice: bool) {
    show_main_window(app);
    let _ = app.emit("quick-capture:open", serde_json::json!({ "voice": voice }));
}

/// Desktop tray + close-to-tray effective value. Always false on mobile.
pub fn get_effective_close_to_tray(config: &pkm_core::TrayConfig, desktop: bool) -> bool {
    desktop && config.enabled && config.close_to_tray
}

#[cfg(desktop)]
fn build_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<tauri::menu::Menu<R>> {
    use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
    let open = MenuItem::with_id(app, "open", "Open Stratum", true, None::<&str>)?;
    let quick_capture =
        MenuItem::with_id(app, "quick_capture", "Quick Capture", true, None::<&str>)?;
    let voice_capture =
        MenuItem::with_id(app, "voice_capture", "Voice Capture", true, None::<&str>)?;
    let sync_now = MenuItem::with_id(app, "sync_now", "Sync Now", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    Menu::with_items(
        app,
        &[
            &open,
            &quick_capture,
            &voice_capture,
            &sync_now,
            &separator,
            &quit,
        ],
    )
}

#[cfg(desktop)]
fn register_shortcut<R: Runtime>(
    shortcuts: &tauri_plugin_global_shortcut::GlobalShortcut<R>,
    shortcut: &str,
) {
    use tauri_plugin_global_shortcut::ShortcutState;
    let result = shortcuts.on_shortcut(shortcut, |app, _sc, event| {
        if event.state() == ShortcutState::Pressed {
            open_quick_capture(app, false);
        }
    });
    if let Err(e) = result {
        warn!(
            "[stratum] global shortcut '{}' could not be registered: {}",
            shortcut, e
        );
    }
}

/// Run the same sync flow as the Settings Sync tab, from a tray menu item.
fn spawn_sync_task<R: Runtime>(app: &AppHandle<R>) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let (vault_path, passphrase) = {
            let app_state = app.state::<AppState>();
            let lock = app_state.lock();
            match lock {
                Ok(s) => (s.vault_path.clone(), s.passphrase.clone()),
                Err(_) => return,
            }
        };
        let _ = sync_vault_core(&vault_path, passphrase.as_deref());
        let _ = app.emit("sync-done", ());
    });
}

/// Quit: let the frontend save its library, fall back to a hard exit after 2s.
#[cfg(desktop)]
fn quit<R: Runtime>(app: &AppHandle<R>) {
    let _ = app.emit("app:quit", ());
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        app.exit(0);
    });
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/// Read the current tray/quick-capture status for the frontend.
///
/// Returns desktop defaults when the vault has no config. On mobile the tray is
/// reported disabled but the configured shortcut/capture values are returned.
#[tauri::command]
pub fn get_tray_status(state: tauri::State<'_, AppState>) -> Result<TrayStatusDto, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let config = load_config(&state)?;
    let desktop = !(cfg!(target_os = "android") || cfg!(target_os = "ios"));
    Ok(TrayStatusDto {
        enabled: if desktop { config.tray.enabled } else { false },
        close_to_tray: get_effective_close_to_tray(&config.tray, desktop),
        shortcut: config.tray.quick_capture_shortcut.clone(),
        capture_target: match config.tray.capture_target {
            pkm_core::CaptureTarget::Inbox => "inbox".into(),
            pkm_core::CaptureTarget::Journal => "journal".into(),
        },
        inbox_path: config.tray.inbox_path.clone(),
    })
}

/// Capture a note as a block into today's journal (or the configured inbox) and
/// persist it to disk + search index.
///
/// The command does its own persistence (store insert, `.md` write and Tantivy
/// index update) so the capture is durable even if the file watcher is off.
#[tauri::command]
pub async fn quick_capture_save(
    text: String,
    state: tauri::State<'_, AppState>,
) -> Result<QuickCaptureResultDto, String> {
    let mut state = state.lock().map_err(|e| e.to_string())?;
    let config = load_config(&state)?;
    let vault = state.vault_path.clone();
    let store = state.get_store().map_err(|e| e.to_string())?;

    let page_path = match config.tray.capture_target {
        pkm_core::CaptureTarget::Inbox => {
            let mut p = config.tray.inbox_path.clone();
            if !p.ends_with(".md") {
                p.push_str(".md");
            }
            p
        }
        pkm_core::CaptureTarget::Journal => {
            let today = chrono::Local::now().format("%Y-%m-%d").to_string();
            format!("journals/{}.md", today)
        }
    };

    ensure_target_page(&store, &vault, &page_path, &config)?;

    let id = uuid::Uuid::new_v4();
    let block = pkm_block::Block::new(id, text.clone());
    store
        .insert_block(&block, &page_path)
        .map_err(|e| e.to_string())?;

    persist_blocks_to_disk(&store, &vault, &page_path)?;
    state.watcher_last_save = std::time::SystemTime::now();
    state.record_change(&page_path);

    let block_index = state.ensure_block_index()?;
    block_index
        .index_block(&block, &page_path)
        .map_err(|e| e.to_string())?;
    block_index.flush().map_err(|e| e.to_string())?;
    // Drop the Tantivy writer so the file watcher can acquire the directory lock.
    drop(state.block_index.take());

    Ok(QuickCaptureResultDto {
        page_path,
        block_id: id.to_string(),
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Load the vault config, falling back to defaults (with the vault path set)
/// when no config file exists.
fn load_config(state: &VaultState) -> Result<pkm_core::Config, String> {
    let config_path = state.vault_path.join(".pkm").join("config.toml");
    if config_path.exists() {
        pkm_core::Config::load(&config_path).map_err(|e| e.to_string())
    } else {
        Ok(pkm_core::Config {
            vault_path: state.vault_path.clone(),
            ..Default::default()
        })
    }
}

/// Ensure the capture target page exists on disk and is registered in SQLite.
///
/// Journal pages use the existing `ensure_today_journal_core` registration; an
/// inbox (or any custom) page is created with a title frontmatter if missing,
/// then synced from disk so it becomes queryable. Idempotent.
fn ensure_target_page(
    store: &pkm_block::BlockStore,
    vault: &Path,
    page_path: &str,
    config: &pkm_core::Config,
) -> Result<(), String> {
    let full = crate::commands::page::resolve_safe_write_path(vault, page_path)?;

    if !full.exists() {
        let title = match config.tray.capture_target {
            pkm_core::CaptureTarget::Journal => chrono::Local::now().format("%Y-%m-%d").to_string(),
            pkm_core::CaptureTarget::Inbox => "Inbox".to_string(),
        };
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let content = format!("---\ntitle: {}\n---\n", title);
        std::fs::write(&full, &content).map_err(|e| e.to_string())?;
    }

    if config.tray.capture_target == pkm_core::CaptureTarget::Journal {
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        crate::commands::page::ensure_today_journal_core(store, vault, &today).map(|_| ())
    } else {
        crate::commands::page::sync_page_from_disk(store, page_path, vault, None).map(|_| ())
    }
}

/// Re-serialize all blocks for a page to disk, preserving every frontmatter field
/// from the existing file (same approach as `save_blocks`).
fn persist_blocks_to_disk(
    store: &pkm_block::BlockStore,
    vault: &Path,
    page_path: &str,
) -> Result<(), String> {
    let full = crate::commands::page::resolve_safe_write_path(vault, page_path)?;
    let blocks = store
        .get_blocks_by_page(page_path)
        .map_err(|e| e.to_string())?;
    let body = pkm_markdown::block_parser::serialize_blocks(&blocks);
    let existing = std::fs::read_to_string(&full).unwrap_or_default();
    let markdown = pkm_markdown::block_parser::assemble_blocks_markdown(&existing, &body, None);

    if let Some(parent) = full.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let temp = full.with_extension("md.tmp");
    std::fs::write(&temp, &markdown).map_err(|e| e.to_string())?;
    std::fs::rename(&temp, &full).map_err(|e| e.to_string())?;
    Ok(())
}
