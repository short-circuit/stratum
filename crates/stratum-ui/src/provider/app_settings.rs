//! [`SettingsData`] implementation for the service-backed provider.
//!
//! Every method delegates to [`StratumService`] (or the free backup helper) on
//! the provider's Tokio runtime and maps the service DTOs into the settings
//! panel contract. Keeping the mapping here leaves `provider/mod.rs` under the
//! repository file-length limit.

use stratum_app::backup::test_webdav;
use stratum_app::import::{ImportOptions, ImportPreview, ImportReport};
use stratum_app::settings::SettingsDto;
use stratum_app::sync::{CommitLogEntry, SyncStatusDto};

use super::settings::{
    BackupStatusDto, DiscoveredEndpoint, ExportOutcome, ReindexResult, SearchPingResult,
    SettingsData, SttTestOutcome, TtsTestOutcome,
};
use super::AppProvider;

impl SettingsData for AppProvider {
    fn settings(&self) -> Result<SettingsDto, String> {
        self.runtime.block_on(self.service.get_settings())
    }

    fn save_settings(&self, settings: SettingsDto) -> Result<SettingsDto, String> {
        self.runtime
            .block_on(self.service.save_settings(settings))?;
        // Reload so masked keys and normalised enum values reflect what was
        // actually written to disk.
        self.runtime.block_on(self.service.get_settings())
    }

    fn set_vault_path(&self, path: &str) -> Result<(), String> {
        self.runtime
            .block_on(self.service.set_vault_path(path.to_string()))
    }

    fn fetch_models(&self) -> Result<Vec<String>, String> {
        self.runtime.block_on(self.service.fetch_models())
    }

    fn discover_local_ai(&self) -> Result<Vec<DiscoveredEndpoint>, String> {
        self.runtime.block_on(self.service.discover_local_ai())
    }

    fn reindex_vault(&self) -> Result<ReindexResult, String> {
        self.runtime
            .block_on(self.service.reindex_vault(&stratum_app::NoopSink))
    }

    fn repair_db_from_disk(&self) -> Result<ReindexResult, String> {
        self.runtime
            .block_on(self.service.repair_db_from_disk(&stratum_app::NoopSink))
    }

    fn normalize_all_files(&self) -> Result<usize, String> {
        self.runtime
            .block_on(self.service.normalize_all_files(&stratum_app::NoopSink))
    }

    fn sync_status(&self) -> Result<SyncStatusDto, String> {
        self.runtime.block_on(self.service.get_sync_status())
    }

    fn sync_now(&self) -> Result<SyncStatusDto, String> {
        self.runtime.block_on(self.service.sync_vault())
    }

    fn sync_with_passphrase(&self, passphrase: &str) -> Result<SyncStatusDto, String> {
        self.runtime.block_on(
            self.service
                .sync_vault_with_passphrase(passphrase.to_string()),
        )
    }

    fn resolve_conflict(&self, path: &str) -> Result<(), String> {
        self.runtime
            .block_on(self.service.resolve_conflict_file(path.to_string()))
    }

    fn abort_merge(&self) -> Result<(), String> {
        self.runtime.block_on(self.service.abort_merge())
    }

    fn start_sync_scheduler(&self) -> Result<(), String> {
        self.runtime.block_on(self.service.start_sync_scheduler())
    }

    fn commit_log(&self) -> Result<Vec<CommitLogEntry>, String> {
        self.runtime.block_on(self.service.get_commit_log())
    }

    fn backup_status(&self) -> Result<BackupStatusDto, String> {
        self.runtime.block_on(self.service.get_backup_status())
    }

    fn run_backup(&self) -> Result<BackupStatusDto, String> {
        self.runtime.block_on(self.service.run_backup())
    }

    fn test_webdav(
        &self,
        url: &str,
        username: Option<&str>,
        password: Option<&str>,
    ) -> Result<String, String> {
        self.runtime.block_on(test_webdav(
            url.to_string(),
            username.map(str::to_string),
            password.map(str::to_string),
        ))
    }

    fn stt_test(&self) -> Result<SttTestOutcome, String> {
        let result = self.runtime.block_on(self.service.stt_test_connection())?;
        Ok(SttTestOutcome {
            ok: result.ok,
            models: result.models,
            latency_ms: result.latency_ms,
            error: result.error,
        })
    }

    fn tts_test(&self, text: &str) -> Result<TtsTestOutcome, String> {
        let result = self
            .runtime
            .block_on(self.service.tts_synthesize(text.to_string()))?;
        use base64::Engine;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(result.audio_b64.as_bytes())
            .map_err(|error| format!("TTS audio decode failed: {error}"))?;
        super::audio::play_tts_bytes(bytes);
        Ok(TtsTestOutcome {
            byte_len: result.byte_len,
            mime: result.mime,
        })
    }

    fn ping_search_instance(&self, url: &str) -> Result<SearchPingResult, String> {
        self.runtime
            .block_on(self.service.ping_search_instance(url.to_string()))
    }

    fn set_onboarding_pending(&self, pending: bool) -> Result<(), String> {
        self.runtime
            .block_on(self.service.set_onboarding_pending(pending))
    }

    fn onboarding_pending(&self) -> bool {
        self.runtime
            .block_on(self.service.get_onboarding_state())
            .unwrap_or(false)
    }

    fn mark_onboarding_done(&self) -> Result<(), String> {
        self.runtime.block_on(self.service.mark_onboarding_done())
    }

    fn vault_path(&self) -> String {
        self.runtime
            .block_on(self.service.get_vault_info())
            .map(|info| info.path)
            .unwrap_or_default()
    }

    fn install_starter_sample(&self) -> Result<Vec<String>, String> {
        self.runtime.block_on(self.service.install_starter_sample())
    }

    fn preview_import(&self, path: &str, options: ImportOptions) -> Result<ImportPreview, String> {
        self.runtime
            .block_on(self.service.preview_import(path.to_string(), options))
    }

    fn run_import(&self, path: &str, options: ImportOptions) -> Result<ImportReport, String> {
        self.runtime
            .block_on(self.service.import_vault(path.to_string(), options))
    }

    fn export_html_to(&self, output_dir: &str) -> Result<ExportOutcome, String> {
        let result = self
            .runtime
            .block_on(self.service.export_html(output_dir.to_string()))?;
        Ok(ExportOutcome {
            output_path: result.output_dir,
            count: result.pages_exported,
            pages: result.pages_exported,
        })
    }

    fn export_json_to(&self, output_dir: &str) -> Result<ExportOutcome, String> {
        let result = self
            .runtime
            .block_on(self.service.export_json(output_dir.to_string()))?;
        Ok(ExportOutcome {
            output_path: result.output_dir,
            count: result.pages_exported,
            pages: result.pages_exported,
        })
    }

    fn export_pdf_to(&self, page_path: &str, output_path: &str) -> Result<ExportOutcome, String> {
        let result = self.runtime.block_on(
            self.service
                .export_pdf(page_path.to_string(), Some(output_path.to_string())),
        )?;
        Ok(ExportOutcome {
            output_path: result.output_path,
            count: 1,
            pages: 1,
        })
    }

    fn export_zip_to(&self, output_path: &str) -> Result<ExportOutcome, String> {
        let result = self
            .runtime
            .block_on(self.service.export_zip(Some(output_path.to_string())))?;
        Ok(ExportOutcome {
            output_path: result.output_path,
            count: result.files_packed,
            pages: result.page_count,
        })
    }
}
