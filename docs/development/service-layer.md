# Headless service layer (`stratum-app`)

`crates/stratum-app` owns the business logic behind every shipped command.
It is a plain Rust library with **no** dependency on any UI toolkit, so the same
logic runs in the desktop shell, on the Android host, in integration tests and in
tooling.

The shell (`crates/stratum-ui`) is a thin layer over it:

```
panel entity ──► ShellData / PanelData  (provider seam)
                        │  state: StratumService
                        ▼
                 crates/stratum-app/src/*   (business logic)
                        │
                        ▼
   pkm-core / pkm-block / pkm-index / pkm-sync / …
```

The split is intentional: command **names, argument names, DTO shapes, event
names/payloads, error strings, and on-disk formats are unchanged**. The React
IPC adapters were deleted at the E12 cutover, but the service surface they
called is still the contract every panel uses.

## Entry point

All commands are exposed as associated functions on
[`stratum_app::StratumService`](../../crates/stratum-app/src/state.rs), a
cloneable handle around the shared vault state (`crates/stratum-app/src/state.rs`).
The method name matches the historical command name, and the argument list
matches those arguments **minus** the host handles.

```rust
use stratum_app::{EventSink, StratumService};

let service = StratumService::new(vault_path);
let info = service.get_vault_info().await?;
```

Operations that emit progress take an
[`EventSink`](../../crates/stratum-app/src/events.rs). Hosts implement that trait
over their own event bus; tests can record emitted events; and `NoopSink`
discards everything. Event names and payload shapes are unchanged (for example
`reindex-progress`, `dictation:stage`, `dictation:done`,
`mic-permission-request`).

## Host rules

Every host (the desktop shell, the Android host) must:

1. Hold a `StratumService` and forward arguments verbatim.
2. Call exactly one service method per user action. Do not add business logic to
   the panel.
3. Forward events through an `EventSink` implementation backed by the shell's
   event bus.
4. Keep platform-only work (folder pickers, tray icons, window management,
   OS notifications, global shortcuts, Android SAF) in `stratum-platform` /
   `stratum-mobile`, calling the service only for the vault/data mutation.
5. Surface the service result unchanged — no error-string rewriting.

## Command → service method map

The table below maps every command in the shipped v1.0.0 surface to the service
method that implements it. The third column records the adapter module the
command was registered from before the cutover; it is kept as provenance and is
**not** a live path. Methods live under `crates/stratum-app/src/`.

`crates/stratum-ui/tests/command_parity.rs` parses this table and fails when a
listed method no longer exists in `stratum-app`, so the inventory cannot drift.

### Vault

| IPC command | Service method | Adapter |
|---|---|---|
| `get_vault_info` | `vault::get_vault_info` | `commands/vault/mod.rs` |
| `set_vault_path` | `vault::set_vault_path` | `commands/vault/mod.rs` |
| `init_vault` | `vault::init_vault` | `commands/vault/mod.rs` |
| `init_default_vault` | `vault::init_vault_at` (path resolved by adapter) | `commands/vault/mod.rs` |
| `pick_vault_directory` (desktop) | `vault::init_vault_at` (path from native dialog) | `commands/vault/mod.rs` |
| `pick_android_directory` (Android) | `vault::init_vault_at` (path from SAF picker) | `commands/vault/mod.rs` |

### Pages

| IPC command | Service method | Adapter |
|---|---|---|
| `list_pages` | `page::list_pages` | `commands/page/mod.rs` |
| `open_page` | `page::open_page` | `commands/page/mod.rs` |
| `save_page` | `page::save_page` | `commands/page/mod.rs` |
| `create_page` | `page::create_page` | `commands/page/mod.rs` |
| `ensure_today_journal` | `page::journal::ensure_today_journal` | `commands/page/journal.rs` |
| `delete_page` | `page::delete_page` | `commands/page/mod.rs` |
| `move_page` | `page::move_page` | `commands/page/move.rs` |
| `reindex_vault` | `page::reindex::reindex_vault` | `commands/page/reindex.rs` |
| `reindex_page` | `page::reindex::reindex_page` | `commands/page/reindex.rs` |
| `repair_db_from_disk` | `page::repair::repair_db_from_disk` | `commands/page/repair.rs` |
| `normalize_file` | `page::normalize::normalize_file` | `commands/page/normalize.rs` |
| `normalize_all_files` | `page::normalize::normalize_all_files` | `commands/page/normalize.rs` |

### Importers

| IPC command | Service method | Adapter |
|---|---|---|
| `preview_import` | `import::preview_import` | `commands/import.rs` |
| `import_vault` | `import::import_vault` | `commands/import.rs` |

### Assets

| IPC command | Service method | Adapter |
|---|---|---|
| `import_asset_path` | `asset::import_asset_path` | `commands/asset.rs` |
| `import_asset_bytes` | `asset::import_asset_bytes` | `commands/asset.rs` |
| `read_asset` | `asset::read_asset` | `commands/asset.rs` |
| `delete_asset` | `asset::delete_asset` | `commands/asset.rs` |
| `list_asset_refs` | `asset::list_asset_refs` | `commands/asset.rs` |
| `get_page_asset_refs` | `asset::get_page_asset_refs` | `commands/asset.rs` |

### Blocks

| IPC command | Service method | Adapter |
|---|---|---|
| `get_blocks` | `block::get_blocks` | `commands/block/mod.rs` |
| `build_markdown` | `block::build_markdown` | `commands/block/mod.rs` |
| `save_blocks` | `block::save_blocks` | `commands/block/mod.rs` |
| `update_block` | `block::update_block` | `commands/block/mod.rs` |
| `delete_block` | `block::delete_block` | `commands/block/mod.rs` |
| `insert_block` | `block::insert_block` | `commands/block/mod.rs` |
| `toggle_block_marker` | `block::marker::toggle_block_marker` | `commands/block/mod.rs` |
| `clear_block_marker` | `block::marker::clear_block_marker` | `commands/block/mod.rs` |

### Search

| IPC command | Service method | Adapter |
|---|---|---|
| `search_blocks` | `search::search_blocks` | `commands/search/mod.rs` |
| `search_by_tag` | `search::autocomplete::search_by_tag` | `commands/search/mod.rs` |
| `rebuild_search_index` | `search::backlinks::rebuild_search_index` | `commands/search/mod.rs` |
| `get_backlinks` | `search::backlinks::get_backlinks` | `commands/search/mod.rs` |
| `get_page_backlinks` | `search::backlinks::get_page_backlinks` | `commands/search/mod.rs` |
| `get_backlink_snippet` | `search::backlinks::get_backlink_snippet` | `commands/search/mod.rs` |
| `autocomplete` | `search::autocomplete::autocomplete` | `commands/search/mod.rs` |
| `suggest_connections` | `search::suggestions::suggest_connections` | `commands/search/mod.rs` |
| `get_backlink_context` | `search::backlinks::get_backlink_context` | `commands/search/mod.rs` |

### Graph

| IPC command | Service method | Adapter |
|---|---|---|
| `get_graph_data` | `graph::get_graph_data` | `commands/graph/mod.rs` |
| `get_connected_components` | `graph::get_connected_components` | `commands/graph/mod.rs` |
| `get_orphaned_notes` | `graph::get_orphaned_notes` | `commands/graph/mod.rs` |
| `get_graph_panel_data` | `graph::get_graph_panel_data` | `commands/graph/mod.rs` |
| `resolve_link_target` | `graph::resolve_link_target` | `commands/graph/mod.rs` |

### Query

| IPC command | Service method | Adapter |
|---|---|---|
| `run_query` | `query::run_query` | `commands/query.rs` |
| `list_saved_queries` | `query::list_saved_queries` | `commands/query.rs` |
| `save_saved_query` | `query::save_saved_query` | `commands/query.rs` |
| `rename_saved_query` | `query::rename_saved_query` | `commands/query.rs` |
| `delete_saved_query` | `query::delete_saved_query` | `commands/query.rs` |

### Sync

| IPC command | Service method | Adapter |
|---|---|---|
| `get_sync_status` | `sync::get_sync_status` | `commands/sync.rs` |
| `sync_vault` | `sync::sync_vault` | `commands/sync.rs` |
| `sync_vault_with_passphrase` | `sync::sync_vault_with_passphrase` | `commands/sync.rs` |
| `start_sync_scheduler` | `sync::start_sync_scheduler` | `commands/sync.rs` |
| `stop_sync_scheduler` | `sync::stop_sync_scheduler` | `commands/sync.rs` |
| `get_commit_log` | `sync::get_commit_log` | `commands/sync.rs` |
| `resolve_conflict_file` | `sync::resolve_conflict_file` | `commands/sync.rs` |
| `abort_merge` | `sync::abort_merge` | `commands/sync.rs` |

### Backup

| IPC command | Service method | Adapter |
|---|---|---|
| `run_backup` | `backup::run_backup` | `commands/backup.rs` |
| `get_backup_status` | `backup::get_backup_status` | `commands/backup.rs` |
| `test_webdav` | `backup::test_webdav` | `commands/backup.rs` |

### Templates

| IPC command | Service method | Adapter |
|---|---|---|
| `list_templates` | `template::list_templates` | `commands/template.rs` |
| `save_template` | `template::save_template` | `commands/template.rs` |
| `apply_template` | `template::apply_template` | `commands/template.rs` |
| `install_starter_templates` | `template::install_starter_templates` | `commands/template.rs` |
| `install_starter_sample` | `template::install_starter_sample` | `commands/template.rs` |

### Export

| IPC command | Service method | Adapter |
|---|---|---|
| `export_html` | `export::export_html` | `commands/export.rs` |
| `export_json` | `export::export_json` | `commands/export.rs` |
| `export_pdf` | `export::export_pdf` | `commands/export/pdf.rs` |
| `export_zip` | `export::export_zip` | `commands/export/zip.rs` |

### Flashcards, kanban, reminders

| IPC command | Service method | Adapter |
|---|---|---|
| `generate_flashcards` | `flashcards::generate_flashcards` | `commands/flashcards.rs` |
| `generate_cards_from_page` | `flashcards::generate_cards_from_page` | `commands/flashcards.rs` |
| `review_card` | `flashcards::review_card` | `commands/flashcards.rs` |
| `get_kanban_blocks` | `kanban::get_kanban_blocks` | `commands/kanban.rs` |
| `create_kanban_block` | `kanban::create_kanban_block` | `commands/kanban.rs` |
| `list_reminders` | `reminders::list_reminders` | `commands/reminders.rs` |
| `snooze_reminder` | `reminders::snooze_reminder` | `commands/reminders.rs` |
| `clear_reminder` | `reminders::clear_reminder` | `commands/reminders.rs` |

### Whiteboards

| IPC command | Service method | Adapter |
|---|---|---|
| `list_whiteboards` | `whiteboard::list_whiteboards` | `commands/whiteboard.rs` |
| `save_whiteboard` | `whiteboard::save_whiteboard` | `commands/whiteboard.rs` |
| `load_whiteboard` | `whiteboard::load_whiteboard` | `commands/whiteboard.rs` |
| `rename_whiteboard` | `whiteboard::rename_whiteboard` | `commands/whiteboard.rs` |
| `delete_whiteboard` | `whiteboard::delete_whiteboard` | `commands/whiteboard.rs` |
| `save_library` | `whiteboard::save_library` | `commands/whiteboard.rs` |
| `load_library` | `whiteboard::load_library` | `commands/whiteboard.rs` |
| `load_extra_libraries` | `whiteboard::load_extra_libraries` | `commands/whiteboard.rs` |

### AI

| IPC command | Service method | Adapter |
|---|---|---|
| `ai_transform_block` | `ai::ai_transform_block` | `commands/ai.rs` |
| `ai_research` | `ai::ai_research` | `commands/ai.rs` |
| `ai_interlink_notes` | `ai::ai_interlink_notes` | `commands/ai.rs` |
| `generate_mermaid` | `ai::generate_mermaid` | `commands/ai.rs` |
| `ai_rag_query` | `ai::ai_rag_query` | `commands/ai.rs` |
| `discover_local_ai` | `ai::discover_local_ai` | `commands/ai.rs` |
| `ping_search_instance` | `ai::ping_search_instance` | `commands/ai.rs` |

### Dictation

| IPC command | Service method | Adapter |
|---|---|---|
| `dictation_start` | `dictation::dictation_start` | `commands/dictation/mod.rs` |
| `dictation_stop` | `dictation::dictation_stop` | `commands/dictation/mod.rs` |
| `dictation_cancel` | `dictation::dictation_cancel` | `commands/dictation/mod.rs` |
| `dictation_transcribe` | `dictation::transcribe::dictation_transcribe` | `commands/dictation/mod.rs` |
| `dictation_insert_clip_ref` | `dictation::memos::dictation_insert_clip_ref` | `commands/dictation/mod.rs` |
| `mic_permission_status` | `dictation::mic::mic_permission_status` | `commands/dictation/mod.rs` |
| `request_mic_permission` | `dictation::mic::request_mic_permission` | `commands/dictation/mod.rs` |
| `speaker_list` | `dictation::speakers::speaker_list` | `commands/dictation/mod.rs` |
| `speaker_assign` | `dictation::speakers::speaker_assign` | `commands/dictation/mod.rs` |
| `speaker_delete` | `dictation::speakers::speaker_delete` | `commands/dictation/mod.rs` |
| `stt_test_connection` | `dictation::stt_test_connection` | `commands/dictation/mod.rs` |

`request_mic_permission` is the one command that still emits from the adapter:
Android-only runtime permission requests need the `AppHandle`. The adapter emits
`mic-permission-request` with `null` and calls the service for the status
update.

### TTS

| IPC command | Service method | Adapter |
|---|---|---|
| `tts_synthesize` | `tts::tts_synthesize` | `commands/tts.rs` |
| `tts_speak` | `tts::tts_speak` | `commands/tts.rs` |

### Settings and onboarding

| IPC command | Service method | Adapter |
|---|---|---|
| `get_settings` | `settings::get_settings` | `commands/settings.rs` |
| `save_settings` | `settings::save_settings` (adapter applies tray state on desktop) | `commands/settings.rs` |
| `save_graph_settings` | `settings::graph::save_graph_settings` | `commands/settings.rs` |
| `fetch_models` | `settings::models::fetch_models` | `commands/settings.rs` |
| `get_onboarding_state` | `settings::onboarding::get_onboarding_state` | `commands/settings.rs` |
| `mark_onboarding_done` | `settings::onboarding::mark_onboarding_done` | `commands/settings.rs` |
| `set_onboarding_pending` | `settings::onboarding::set_onboarding_pending` | `commands/settings.rs` |

### Tray and quick capture

| IPC command | Service method | Adapter |
|---|---|---|
| `get_tray_status` (desktop) | `tray::get_tray_status` | `commands/tray.rs` |
| `quick_capture_save` (desktop) | `tray::quick_capture_save` | `commands/tray.rs` |

`tray::get_effective_close_to_tray` is a pure helper used by the setup code
when creating the tray menu; it has no IPC command.

### Plugins

| IPC command | Service method | Adapter |
|---|---|---|
| `plugins_list` | `plugins::plugins_list` | `commands/plugins.rs` |
| `plugins_enable` | `plugins::plugins_enable` | `commands/plugins.rs` |
| `plugins_disable` | `plugins::plugins_disable` | `commands/plugins.rs` |
| `plugins_reload` | `plugins::plugins_reload` | `commands/plugins.rs` |
| `plugins_status` | `plugins::plugins_status` | `commands/plugins.rs` |
| `plugins_install` | `plugins::plugins_install` | `commands/plugins.rs` |
| `plugins_install_url` | `plugins::plugins_install_url` | `commands/plugins.rs` |
| `plugins_uninstall` | `plugins::plugins_uninstall` | `commands/plugins.rs` |
| `plugin_note_read` | `plugins::plugin_note_read` | `commands/plugins.rs` |
| `plugin_http_request` | `plugins::plugin_http_request` | `commands/plugins.rs` |

Hook dispatch helpers (`dispatch_on_open`, `dispatch_on_link`,
`dispatch_on_search`, …) live in `crates/stratum-app/src/plugins/hooks.rs` and
are re-exported from `stratum_app::plugins` for the shell and tests.

## Testing

Headless tests live in `crates/stratum-app/tests/`, one file per domain
(`vault_service.rs`, `page_service.rs`, `block_service.rs`, …). They drive
`StratumService` directly with a real temp vault and a `RecordingSink` that
captures events — no UI runtime and no serialization. This is the preferred
place for new command coverage; `crates/stratum-ui/tests/` holds the shell
contracts (menu order, validation, catalog gates, parity) and
`crates/pkm-tests/tests/` the cross-crate integration suites.

Tests that need an external service use `wiremock` (AI, STT, TTS, WebDAV).
Tests that need a microphone or a Chromium binary skip cleanly when the
resource is unavailable.

## Adding a command

1. Implement the logic as a `StratumService` method in the matching
   `crates/stratum-app/src/` module. Keep the command name, argument names,
   DTOs, and error strings.
2. Expose it through the provider seam (`ShellData`/`PanelData`), implement it on
   both the service-backed and fixture providers, and add a typed accessor for
   the panel.
3. Add a headless test in `crates/stratum-app/tests/` that calls the service
   method directly, plus a contract test when the panel has a gate list.
4. Update the table above, the inventory gate's `SHIPPED_COMMANDS` constant in
   `crates/stratum-ui/tests/command_parity.rs`, and any user-facing guide that
   documents the feature.
