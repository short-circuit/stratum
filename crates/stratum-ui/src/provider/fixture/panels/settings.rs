//! [`SettingsData`] implementation and seed data for the fixture provider.
//!
//! The fixture persists the full [`SettingsDto`] in [`super::PanelState`] so the
//! settings save/reload round-trip can be exercised without a vault. Mutating
//! calls return deterministic results documented in the fixture panel module.

use stratum_app::backup::BackupStatusDto;
use stratum_app::import::{
    ImportFormat, ImportOptions, ImportPreview, ImportPreviewItem, ImportReport,
};
use stratum_app::settings::{
    AiModelDto, AiSettingsDto, BackupSettingsDto, GraphSettingsDto, LayoutSettingsDto,
    ReminderSettingsDto, ResearchSettingsDto, SettingsDto, SttSettingsDto, SyncSettingsDto,
    ThemeSettingsDto, TraySettingsDto, TtsSettingsDto, VoiceSettingsDto,
};
use stratum_app::sync::{CommitLogEntry, SyncStatusDto};

use super::{panel_state, FixtureProvider};
use crate::provider::settings::{
    DiscoveredEndpoint, ExportOutcome, ReindexResult, SearchPingResult, SettingsData,
    SttTestOutcome, TtsTestOutcome,
};

/// The deterministic settings seed.
pub(super) fn fixture_settings() -> SettingsDto {
    SettingsDto {
        vault_path: "/vaults/fixture".to_string(),
        locale: "en".to_string(),
        layout: LayoutSettingsDto {
            pages_view: "flat".to_string(),
        },
        theme: ThemeSettingsDto {
            dark_mode: true,
            primary_color: "#f97316".to_string(),
            secondary_color: "#6b7280".to_string(),
            font_size: 16,
        },
        ai: AiSettingsDto {
            provider: "ollama".to_string(),
            endpoint: Some("http://localhost:11434".to_string()),
            api_key: None,
            api_key_from_env: false,
            model: "llama3".to_string(),
            models: vec![
                AiModelDto {
                    name: "llama3".to_string(),
                    capabilities: vec!["chat".to_string()],
                },
                AiModelDto {
                    name: "nomic-embed-text".to_string(),
                    capabilities: vec!["embedding".to_string()],
                },
            ],
            rag_enabled: true,
            rag_chunk_count: 4,
            embedding_dimensions: 768,
            use_llm_gateway_and_auth: false,
            extra_local_hosts: vec!["http://127.0.0.1:1234".to_string()],
            last_discovered: Vec::new(),
        },
        research: ResearchSettingsDto {
            searxng_endpoint: "http://localhost:8888".to_string(),
            max_results: 3,
            max_depth: 2,
        },
        graph: GraphSettingsDto {
            show_connected: true,
            show_orphaned: false,
            show_tags: false,
            charge_strength: -30.0,
            link_distance: 30.0,
            alpha_decay: 0.1,
            velocity_decay: 0.5,
            link_curvature: 0.0,
            use_3d: true,
        },
        sync: SyncSettingsDto {
            mode: "manual".to_string(),
            remote_url: Some("git@github.com:example/vault.git".to_string()),
            branch: "main".to_string(),
            auto_commit_interval_secs: 300,
            auto_sync_interval_secs: 1800,
            ssh_key_path: None,
            commit_template:
                "stratum({datetime}): {editedfiles} edited, {newfiles} added, {deletedfiles} deleted"
                    .to_string(),
            backup: BackupSettingsDto::default(),
        },
        stt: SttSettingsDto {
            endpoint: "http://localhost:8081".to_string(),
            api_key: None,
            model: "whisper-1".to_string(),
            diarize_model: "pyannote".to_string(),
            language: None,
            diarize: true,
            auto_summarize: true,
            auto_identify: false,
            use_llm_gateway_and_auth: false,
        },
        tts: TtsSettingsDto::default(),
        tray: TraySettingsDto::default(),
        voice: VoiceSettingsDto::default(),
        reminders: ReminderSettingsDto::default(),
    }
}

impl SettingsData for FixtureProvider {
    fn settings(&self) -> Result<SettingsDto, String> {
        Ok(panel_state(self)?.settings.clone())
    }

    fn save_settings(&self, settings: SettingsDto) -> Result<SettingsDto, String> {
        let mut state = panel_state(self)?;
        state.settings = settings;
        Ok(state.settings.clone())
    }

    fn set_vault_path(&self, path: &str) -> Result<(), String> {
        panel_state(self)?.settings.vault_path = path.to_string();
        Ok(())
    }

    fn fetch_models(&self) -> Result<Vec<String>, String> {
        Ok(vec!["llama3".to_string(), "nomic-embed-text".to_string()])
    }

    fn discover_local_ai(&self) -> Result<Vec<DiscoveredEndpoint>, String> {
        Ok(vec![
            DiscoveredEndpoint {
                host: "http://localhost:11434".to_string(),
                name: "Ollama".to_string(),
                kind: "ollama".to_string(),
                models: vec!["llama3".to_string(), "nomic-embed-text".to_string()],
                status: "available".to_string(),
                latency_ms: 8,
                error: None,
            },
            DiscoveredEndpoint {
                host: "http://127.0.0.1:1234/v1".to_string(),
                name: "LM Studio".to_string(),
                kind: "openai_compatible".to_string(),
                models: vec!["local-model".to_string()],
                status: "available".to_string(),
                latency_ms: 12,
                error: None,
            },
        ])
    }

    fn reindex_vault(&self) -> Result<ReindexResult, String> {
        Ok(ReindexResult {
            processed: 4,
            succeeded: 4,
            failed: 0,
            errors: Vec::new(),
        })
    }

    fn repair_db_from_disk(&self) -> Result<ReindexResult, String> {
        Ok(ReindexResult {
            processed: 4,
            succeeded: 4,
            failed: 0,
            errors: Vec::new(),
        })
    }

    fn normalize_all_files(&self) -> Result<usize, String> {
        Ok(4)
    }

    fn sync_status(&self) -> Result<SyncStatusDto, String> {
        let conflicts = panel_state(self)?.conflicts.clone();
        if conflicts.is_empty() {
            Ok(SyncStatusDto {
                status: "ok".to_string(),
                branch: Some("main".to_string()),
                ahead: 1,
                behind: 0,
                conflicts,
                last_sync_time: Some("2026-11-10T08:00:00Z".to_string()),
                last_sync_success: Some(true),
                pending_commits: 0,
            })
        } else {
            Ok(SyncStatusDto {
                status: "conflicts".to_string(),
                branch: Some("main".to_string()),
                ahead: 1,
                behind: 2,
                conflicts,
                last_sync_time: Some("2026-11-10T08:00:00Z".to_string()),
                last_sync_success: Some(false),
                pending_commits: 0,
            })
        }
    }

    fn sync_now(&self) -> Result<SyncStatusDto, String> {
        self.sync_status()
    }

    fn sync_with_passphrase(&self, _passphrase: &str) -> Result<SyncStatusDto, String> {
        self.sync_status()
    }

    fn resolve_conflict(&self, path: &str) -> Result<(), String> {
        panel_state(self)?
            .conflicts
            .retain(|candidate| candidate != path);
        Ok(())
    }

    fn abort_merge(&self) -> Result<(), String> {
        panel_state(self)?.conflicts.clear();
        Ok(())
    }

    fn start_sync_scheduler(&self) -> Result<(), String> {
        Ok(())
    }

    fn commit_log(&self) -> Result<Vec<CommitLogEntry>, String> {
        Ok(vec![
            CommitLogEntry {
                hash: "a1b2c3d4e5f6a7b8".to_string(),
                author: "Stratum".to_string(),
                message: "stratum(2026-11-10 08:00:00): 3 edited, 1 added, 0 deleted".to_string(),
                timestamp: "2026-11-10T08:00:00Z".to_string(),
            },
            CommitLogEntry {
                hash: "b2c3d4e5f6a7b8c9".to_string(),
                author: "Stratum".to_string(),
                message: "stratum(2026-11-09 18:30:00): 1 edited, 0 added, 0 deleted".to_string(),
                timestamp: "2026-11-09T18:30:00Z".to_string(),
            },
        ])
    }

    fn backup_status(&self) -> Result<BackupStatusDto, String> {
        let runs = panel_state(self)?.backup_runs;
        Ok(BackupStatusDto {
            enabled: true,
            mode: "folder".to_string(),
            last_run: Some("2026-11-10T08:00:00Z".to_string()),
            last_success: Some(true),
            last_error: None,
            files_copied: Some(12 + runs),
            files_deleted: Some(0),
        })
    }

    fn run_backup(&self) -> Result<BackupStatusDto, String> {
        panel_state(self)?.backup_runs += 1;
        self.backup_status()
    }

    fn test_webdav(
        &self,
        _url: &str,
        _username: Option<&str>,
        _password: Option<&str>,
    ) -> Result<String, String> {
        Ok("ok".to_string())
    }

    fn stt_test(&self) -> Result<SttTestOutcome, String> {
        Ok(SttTestOutcome {
            ok: true,
            models: vec!["whisper-1".to_string()],
            latency_ms: 30,
            error: None,
        })
    }

    fn tts_test(&self, _text: &str) -> Result<TtsTestOutcome, String> {
        Ok(TtsTestOutcome {
            byte_len: 2048,
            mime: "audio/mpeg".to_string(),
        })
    }

    fn ping_search_instance(&self, _url: &str) -> Result<SearchPingResult, String> {
        Ok(SearchPingResult {
            ok: true,
            latency_ms: 42,
            error: None,
        })
    }

    fn set_onboarding_pending(&self, _pending: bool) -> Result<(), String> {
        Ok(())
    }

    fn preview_import(&self, path: &str, _options: ImportOptions) -> Result<ImportPreview, String> {
        Ok(ImportPreview {
            source_path: path.to_string(),
            source_name: "fixture-vault".to_string(),
            format: ImportFormat::Obsidian,
            pages: vec![
                ImportPreviewItem {
                    source_rel: "Welcome.md".to_string(),
                    rel_path: "imported/fixture-vault/Welcome.md".to_string(),
                    title: "Welcome".to_string(),
                    status: "new".to_string(),
                    size_bytes: 512,
                },
                ImportPreviewItem {
                    source_rel: "Projects/Plan.md".to_string(),
                    rel_path: "imported/fixture-vault/Projects/Plan.md".to_string(),
                    title: "Plan".to_string(),
                    status: "new".to_string(),
                    size_bytes: 384,
                },
            ],
            assets: Vec::new(),
            total_pages: 2,
            total_assets: 0,
            duplicates: Vec::new(),
            conflicts: Vec::new(),
        })
    }

    fn run_import(&self, path: &str, options: ImportOptions) -> Result<ImportReport, String> {
        Ok(ImportReport {
            source_path: path.to_string(),
            source_name: "fixture-vault".to_string(),
            format: ImportFormat::Obsidian,
            pages_imported: if options.dry_run { 0 } else { 2 },
            assets_imported: 0,
            duplicates_skipped: 0,
            conflicts_skipped: 0,
            dry_run: options.dry_run,
            errors: Vec::new(),
            imported_paths: vec![
                "imported/fixture-vault/Welcome.md".to_string(),
                "imported/fixture-vault/Projects/Plan.md".to_string(),
            ],
        })
    }

    fn export_html_to(&self, output_dir: &str) -> Result<ExportOutcome, String> {
        Ok(ExportOutcome {
            output_path: output_dir.to_string(),
            count: 4,
            pages: 4,
        })
    }

    fn export_json_to(&self, output_dir: &str) -> Result<ExportOutcome, String> {
        Ok(ExportOutcome {
            output_path: output_dir.to_string(),
            count: 4,
            pages: 4,
        })
    }

    fn export_pdf_to(&self, _page_path: &str, output_path: &str) -> Result<ExportOutcome, String> {
        Ok(ExportOutcome {
            output_path: output_path.to_string(),
            count: 1,
            pages: 1,
        })
    }

    fn export_zip_to(&self, output_path: &str) -> Result<ExportOutcome, String> {
        Ok(ExportOutcome {
            output_path: output_path.to_string(),
            count: 12,
            pages: 4,
        })
    }
}
