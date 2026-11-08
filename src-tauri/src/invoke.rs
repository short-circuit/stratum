//! The IPC command registry.
//!
//! Every command is a thin adapter in [`crate::commands`] that delegates to
//! [`stratum_app::StratumService`]. Keeping the list here leaves `lib.rs` as a
//! small bootstrap file.
//!
//! The registry is a macro so `tauri::generate_handler!` expands at the call
//! site (where the concrete runtime is known) while the long command list stays
//! out of `lib.rs`.

macro_rules! invoke_handler {
    () => {
        tauri::generate_handler![
            // Vault
            crate::commands::vault::get_vault_info,
            crate::commands::vault::set_vault_path,
            crate::commands::vault::init_vault,
            crate::commands::vault::init_default_vault,
            #[cfg(desktop)]
            crate::commands::vault::pick_vault_directory,
            #[cfg(target_os = "android")]
            crate::commands::vault::pick_android_directory,
            // Pages
            crate::commands::page::list_pages,
            crate::commands::page::open_page,
            crate::commands::page::save_page,
            crate::commands::page::create_page,
            crate::commands::page::ensure_today_journal,
            crate::commands::page::delete_page,
            crate::commands::page::move_page,
            crate::commands::page::reindex_vault,
            crate::commands::page::reindex_page,
            crate::commands::page::repair_db_from_disk,
            crate::commands::page::normalize_file,
            crate::commands::page::normalize_all_files,
            // Importers
            crate::commands::import::preview_import,
            crate::commands::import::import_vault,
            // Assets
            crate::commands::asset::import_asset_path,
            crate::commands::asset::import_asset_bytes,
            crate::commands::asset::read_asset,
            crate::commands::asset::delete_asset,
            crate::commands::asset::list_asset_refs,
            crate::commands::asset::get_page_asset_refs,
            // Blocks
            crate::commands::block::get_blocks,
            crate::commands::block::build_markdown,
            crate::commands::block::save_blocks,
            crate::commands::block::update_block,
            crate::commands::block::delete_block,
            crate::commands::block::insert_block,
            crate::commands::block::toggle_block_marker,
            crate::commands::block::clear_block_marker,
            // Search
            crate::commands::search::search_blocks,
            crate::commands::search::search_by_tag,
            crate::commands::search::rebuild_search_index,
            crate::commands::search::get_backlinks,
            crate::commands::search::get_page_backlinks,
            crate::commands::search::get_backlink_snippet,
            crate::commands::search::autocomplete,
            crate::commands::search::suggest_connections,
            crate::commands::search::get_backlink_context,
            // Graph
            crate::commands::graph::get_graph_data,
            crate::commands::graph::get_connected_components,
            crate::commands::graph::get_orphaned_notes,
            crate::commands::graph::get_graph_panel_data,
            crate::commands::graph::resolve_link_target,
            // Query
            crate::commands::query::run_query,
            crate::commands::query::list_saved_queries,
            crate::commands::query::save_saved_query,
            crate::commands::query::rename_saved_query,
            crate::commands::query::delete_saved_query,
            // Sync
            crate::commands::sync::get_sync_status,
            crate::commands::sync::sync_vault,
            crate::commands::sync::sync_vault_with_passphrase,
            crate::commands::sync::start_sync_scheduler,
            crate::commands::sync::stop_sync_scheduler,
            crate::commands::sync::get_commit_log,
            crate::commands::sync::resolve_conflict_file,
            crate::commands::sync::abort_merge,
            // Backup (E10.F3)
            crate::commands::backup::run_backup,
            crate::commands::backup::get_backup_status,
            crate::commands::backup::test_webdav,
            // Templates
            crate::commands::template::list_templates,
            crate::commands::template::save_template,
            crate::commands::template::apply_template,
            crate::commands::template::install_starter_templates,
            crate::commands::template::install_starter_sample,
            // Export
            crate::commands::export::export_html,
            crate::commands::export::export_json,
            crate::commands::export::pdf::export_pdf,
            crate::commands::export::zip::export_zip,
            // Flashcards
            crate::commands::flashcards::generate_flashcards,
            crate::commands::flashcards::generate_cards_from_page,
            crate::commands::flashcards::review_card,
            // Kanban
            crate::commands::kanban::get_kanban_blocks,
            crate::commands::kanban::create_kanban_block,
            // Reminders
            crate::commands::reminders::list_reminders,
            crate::commands::reminders::snooze_reminder,
            crate::commands::reminders::clear_reminder,
            // Whiteboards
            crate::commands::whiteboard::list_whiteboards,
            crate::commands::whiteboard::save_whiteboard,
            crate::commands::whiteboard::load_whiteboard,
            crate::commands::whiteboard::rename_whiteboard,
            crate::commands::whiteboard::delete_whiteboard,
            crate::commands::whiteboard::save_library,
            crate::commands::whiteboard::load_library,
            crate::commands::whiteboard::load_extra_libraries,
            // AI
            crate::commands::ai::ai_transform_block,
            crate::commands::ai::ai_research,
            crate::commands::ai::ai_interlink_notes,
            crate::commands::ai::generate_mermaid,
            crate::commands::ai::ai_rag_query,
            crate::commands::ai::discover_local_ai,
            crate::commands::ai::ping_search_instance,
            // Dictation
            crate::commands::dictation::dictation_start,
            crate::commands::dictation::dictation_stop,
            crate::commands::dictation::dictation_cancel,
            crate::commands::dictation::dictation_transcribe,
            crate::commands::dictation::dictation_insert_clip_ref,
            crate::commands::dictation::mic_permission_status,
            crate::commands::dictation::request_mic_permission,
            crate::commands::dictation::speaker_list,
            crate::commands::dictation::speaker_assign,
            crate::commands::dictation::speaker_delete,
            crate::commands::dictation::stt_test_connection,
            // TTS
            crate::commands::tts::tts_synthesize,
            crate::commands::tts::tts_speak,
            // Settings
            crate::commands::settings::get_settings,
            crate::commands::settings::save_settings,
            crate::commands::settings::save_graph_settings,
            crate::commands::settings::fetch_models,
            // Tray / quick capture
            #[cfg(desktop)]
            crate::commands::tray::get_tray_status,
            #[cfg(desktop)]
            crate::commands::tray::quick_capture_save,
            // Onboarding
            crate::commands::settings::get_onboarding_state,
            crate::commands::settings::mark_onboarding_done,
            crate::commands::settings::set_onboarding_pending,
            // Plugins
            crate::commands::plugins::plugins_list,
            crate::commands::plugins::plugins_enable,
            crate::commands::plugins::plugins_disable,
            crate::commands::plugins::plugins_reload,
            crate::commands::plugins::plugins_status,
            crate::commands::plugins::plugins_install,
            crate::commands::plugins::plugins_install_url,
            crate::commands::plugins::plugins_uninstall,
            crate::commands::plugins::plugin_note_read,
            crate::commands::plugins::plugin_http_request,
        ]
    };
}

pub(crate) use invoke_handler;
