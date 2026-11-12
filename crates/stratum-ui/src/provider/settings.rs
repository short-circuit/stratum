//! Settings-panel provider seam.
//!
//! [`SettingsData`] is the data contract behind the native Settings surface.
//! Every method has a safe default so existing providers and window-free tests
//! keep compiling; the service-backed implementation lives in
//! [`super::app_settings`] and the deterministic fixture in
//! [`super::fixture::panels::settings`].
//!
//! The settings DTOs and the import/sync/backup/export result types are
//! re-exported from `stratum-app` so hosts never maintain a second copy of the
//! schema.

pub use stratum_app::backup::BackupStatusDto;
pub use stratum_app::events::ReindexResult;
pub use stratum_app::import::{
    ImportAsset, ImportFormat, ImportOptions, ImportPreview, ImportPreviewItem, ImportReport,
};
pub use stratum_app::settings::{
    AiModelDto, AiSettingsDto, BackupSettingsDto, GraphSettingsDto, LayoutSettingsDto,
    ReminderSettingsDto, ResearchSettingsDto, SettingsDto, SttSettingsDto, SyncSettingsDto,
    ThemeSettingsDto, TraySettingsDto, TtsSettingsDto, VoiceSettingsDto,
};
pub use stratum_app::sync::{CommitLogEntry, SyncStatusDto};

pub use pkm_core::{DiscoveredEndpoint, SearchPingResult};

/// Stable error for providers that cannot answer a settings request.
pub const SETTINGS_DATA_NOT_AVAILABLE: &str = "settings_data_not_available";

/// Result of a speech-to-text connectivity probe.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SttTestOutcome {
    /// Whether the endpoint answered.
    pub ok: bool,
    /// Models reported by the endpoint.
    pub models: Vec<String>,
    /// Probe latency in milliseconds.
    pub latency_ms: u64,
    /// Human-readable error when the probe failed.
    pub error: Option<String>,
}

/// Result of a text-to-speech synthesis probe.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TtsTestOutcome {
    /// Decoded audio size in bytes.
    pub byte_len: usize,
    /// Audio MIME type.
    pub mime: String,
}

/// Result of a settings-page export action.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportOutcome {
    /// Vault-relative or absolute output path.
    pub output_path: String,
    /// Pages written (HTML/JSON/PDF) or files packed (ZIP).
    pub count: usize,
    /// Pages represented by the archive (ZIP only; mirrors `count` otherwise).
    pub pages: usize,
}

/// Settings operations used by the native Settings panel.
///
/// Defaults keep the trait non-breaking: a provider that only implements the
/// shell surface still compiles. Methods whose empty result would mislead the
/// UI return [`SETTINGS_DATA_NOT_AVAILABLE`] instead.
pub trait SettingsData: Send + Sync {
    /// Read the effective settings for the current vault.
    fn settings(&self) -> Result<SettingsDto, String> {
        Err(SETTINGS_DATA_NOT_AVAILABLE.to_string())
    }

    /// Persist settings and return the stored (reloaded) settings.
    fn save_settings(&self, _settings: SettingsDto) -> Result<SettingsDto, String> {
        Err(SETTINGS_DATA_NOT_AVAILABLE.to_string())
    }

    /// Fetch the models available from the configured AI endpoint.
    fn fetch_models(&self) -> Result<Vec<String>, String> {
        Ok(Vec::new())
    }

    /// Discover local AI endpoints.
    fn discover_local_ai(&self) -> Result<Vec<DiscoveredEndpoint>, String> {
        Ok(Vec::new())
    }

    /// Rebuild the vault index from disk.
    fn reindex_vault(&self) -> Result<ReindexResult, String> {
        Err(SETTINGS_DATA_NOT_AVAILABLE.to_string())
    }

    /// Reconcile the database with the files on disk.
    fn repair_db_from_disk(&self) -> Result<ReindexResult, String> {
        Err(SETTINGS_DATA_NOT_AVAILABLE.to_string())
    }

    /// Normalize every page through the block parser.
    fn normalize_all_files(&self) -> Result<usize, String> {
        Ok(0)
    }

    /// Read the current git sync status.
    fn sync_status(&self) -> Result<SyncStatusDto, String> {
        Err(SETTINGS_DATA_NOT_AVAILABLE.to_string())
    }

    /// Run a manual sync.
    fn sync_now(&self) -> Result<SyncStatusDto, String> {
        Err(SETTINGS_DATA_NOT_AVAILABLE.to_string())
    }

    /// Run a manual sync with an SSH key passphrase.
    fn sync_with_passphrase(&self, _passphrase: &str) -> Result<SyncStatusDto, String> {
        Err(SETTINGS_DATA_NOT_AVAILABLE.to_string())
    }

    /// Start the periodic sync scheduler.
    fn start_sync_scheduler(&self) -> Result<(), String> {
        Ok(())
    }

    /// Read the recent commit log.
    fn commit_log(&self) -> Result<Vec<CommitLogEntry>, String> {
        Ok(Vec::new())
    }

    /// Read the cloud/folder backup status.
    fn backup_status(&self) -> Result<BackupStatusDto, String> {
        Err(SETTINGS_DATA_NOT_AVAILABLE.to_string())
    }

    /// Run a cloud/folder backup now.
    fn run_backup(&self) -> Result<BackupStatusDto, String> {
        Err(SETTINGS_DATA_NOT_AVAILABLE.to_string())
    }

    /// Test a WebDAV endpoint, returning `"ok"` or an error message.
    fn test_webdav(
        &self,
        _url: &str,
        _username: Option<&str>,
        _password: Option<&str>,
    ) -> Result<String, String> {
        Ok("ok".to_string())
    }

    /// Probe the configured speech-to-text endpoint.
    fn stt_test(&self) -> Result<SttTestOutcome, String> {
        Ok(SttTestOutcome {
            ok: false,
            models: Vec::new(),
            latency_ms: 0,
            error: Some(SETTINGS_DATA_NOT_AVAILABLE.to_string()),
        })
    }

    /// Synthesize a short text-to-speech sample.
    fn tts_test(&self, _text: &str) -> Result<TtsTestOutcome, String> {
        Err(SETTINGS_DATA_NOT_AVAILABLE.to_string())
    }

    /// Ping a SearXNG instance.
    fn ping_search_instance(&self, _url: &str) -> Result<SearchPingResult, String> {
        Ok(SearchPingResult {
            ok: false,
            latency_ms: 0,
            error: Some(SETTINGS_DATA_NOT_AVAILABLE.to_string()),
        })
    }

    /// Mark the first-run onboarding wizard as pending.
    fn set_onboarding_pending(&self, _pending: bool) -> Result<(), String> {
        Ok(())
    }

    /// Preview a vault import without writing anything.
    fn preview_import(
        &self,
        _path: &str,
        _options: ImportOptions,
    ) -> Result<ImportPreview, String> {
        Err(SETTINGS_DATA_NOT_AVAILABLE.to_string())
    }

    /// Run a vault import.
    fn run_import(&self, _path: &str, _options: ImportOptions) -> Result<ImportReport, String> {
        Err(SETTINGS_DATA_NOT_AVAILABLE.to_string())
    }

    /// Export the vault as static HTML to `output_dir`.
    fn export_html_to(&self, _output_dir: &str) -> Result<ExportOutcome, String> {
        Err(SETTINGS_DATA_NOT_AVAILABLE.to_string())
    }

    /// Export the vault as JSON to `output_dir`.
    fn export_json_to(&self, _output_dir: &str) -> Result<ExportOutcome, String> {
        Err(SETTINGS_DATA_NOT_AVAILABLE.to_string())
    }

    /// Export one page as PDF.
    fn export_pdf_to(&self, _page_path: &str, _output_path: &str) -> Result<ExportOutcome, String> {
        Err(SETTINGS_DATA_NOT_AVAILABLE.to_string())
    }

    /// Export the vault as a portable ZIP archive.
    fn export_zip_to(&self, _output_path: &str) -> Result<ExportOutcome, String> {
        Err(SETTINGS_DATA_NOT_AVAILABLE.to_string())
    }
}
