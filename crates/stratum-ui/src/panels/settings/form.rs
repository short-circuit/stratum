//! Editable settings form state.
//!
//! The native Settings surface edits the shipped [`SettingsDto`] verbatim, so
//! saving round-trips every field the frozen React page exposes. Numeric inputs
//! keep their raw text here: parsing happens only when a save is attempted, which
//! lets validation reject bad text instead of silently replacing it with a
//! default.
//!
//! Export paths and the import source are shell-only form state; they are not
//! part of [`SettingsDto`] because the shipped page keeps them in local React
//! state as well.

use std::collections::BTreeMap;

use stratum_app::import::ImportFormat;
use stratum_app::settings::{
    AiSettingsDto, BackupSettingsDto, GraphSettingsDto, LayoutSettingsDto, ReminderSettingsDto,
    ResearchSettingsDto, SettingsDto, SttSettingsDto, SyncSettingsDto, ThemeSettingsDto,
    TraySettingsDto, TtsSettingsDto, VoiceSettingsDto,
};

use crate::theme::ThemeConfig;

use super::raw::{non_empty, parse_f32, parse_u32, parse_u64, parse_usize, static_key};

/// Raw-text key for the theme font size.
pub const KEY_THEME_FONT_SIZE: &str = "theme_font_size";
/// Raw-text key for the RAG chunk count.
pub const KEY_AI_RAG_CHUNKS: &str = "ai_rag_chunk_count";
/// Raw-text key for the embedding dimensions.
pub const KEY_AI_EMBEDDING_DIMENSIONS: &str = "ai_embedding_dimensions";
/// Raw-text key for the extra local hosts.
pub const KEY_AI_EXTRA_HOSTS: &str = "ai_extra_local_hosts";
/// Raw-text key for the research result cap.
pub const KEY_RESEARCH_MAX_RESULTS: &str = "research_max_results";
/// Raw-text key for the research depth.
pub const KEY_RESEARCH_MAX_DEPTH: &str = "research_max_depth";
/// Raw-text key for the auto-commit interval.
pub const KEY_SYNC_COMMIT_INTERVAL: &str = "sync_commit_interval";
/// Raw-text key for the auto-sync interval.
pub const KEY_SYNC_SYNC_INTERVAL: &str = "sync_sync_interval";
/// Raw-text key for the backup interval.
pub const KEY_BACKUP_INTERVAL: &str = "backup_interval";
/// Raw-text key for the backup exclude list.
pub const KEY_BACKUP_EXCLUDE: &str = "backup_exclude";
/// Raw-text key for the TTS playback speed.
pub const KEY_TTS_SPEED: &str = "tts_speed";

/// The four export rows in the shipped Export section.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExportKind {
    /// Static HTML export.
    Html,
    /// JSON export.
    Json,
    /// Per-page PDF export.
    Pdf,
    /// Portable vault ZIP export.
    Zip,
}

impl ExportKind {
    /// Every export row, in shipped order.
    pub const ALL: [Self; 4] = [Self::Html, Self::Json, Self::Pdf, Self::Zip];

    /// Catalog key for the export action label.
    pub fn label_key(self) -> &'static str {
        match self {
            Self::Html => "export.html",
            Self::Json => "export.json",
            Self::Pdf => "export.pdf",
            Self::Zip => "export.zip",
        }
    }

    /// Input-registry key for the output path.
    pub fn input_key(self) -> &'static str {
        match self {
            Self::Html => "export_html",
            Self::Json => "export_json",
            Self::Pdf => "export_pdf",
            Self::Zip => "export_zip",
        }
    }

    /// Whether the shipped row browses for a directory instead of a file.
    pub fn is_folder(self) -> bool {
        matches!(self, Self::Html | Self::Json)
    }
}

/// Editable state behind the native Settings panel.
pub struct SettingsForm {
    /// The DTO being edited.
    pub settings: SettingsDto,
    /// Raw text for numeric and list fields.
    pub raw: BTreeMap<&'static str, String>,
    /// Export output paths keyed by row.
    pub exports: BTreeMap<ExportKind, String>,
    /// Import source path.
    pub import_path: String,
    /// Import format selector.
    pub import_format: ImportFormat,
    /// Whether the import is a preview-only dry run.
    pub import_dry_run: bool,
    /// Ephemeral SSH passphrase for the inline sync action.
    pub passphrase: String,
    /// Field → validation-key map from the last failed save.
    pub errors: BTreeMap<&'static str, &'static str>,
    /// The last theme configuration that passed validation.
    pub last_valid_theme: ThemeConfig,
}

impl SettingsForm {
    /// Build a form from persisted settings.
    pub fn from_settings(settings: SettingsDto) -> Self {
        let vault = settings.vault_path.clone();
        let raw = BTreeMap::from([
            (KEY_THEME_FONT_SIZE, settings.theme.font_size.to_string()),
            (KEY_AI_RAG_CHUNKS, settings.ai.rag_chunk_count.to_string()),
            (
                KEY_AI_EMBEDDING_DIMENSIONS,
                settings.ai.embedding_dimensions.to_string(),
            ),
            (KEY_AI_EXTRA_HOSTS, settings.ai.extra_local_hosts.join("\n")),
            (
                KEY_RESEARCH_MAX_RESULTS,
                settings.research.max_results.to_string(),
            ),
            (
                KEY_RESEARCH_MAX_DEPTH,
                settings.research.max_depth.to_string(),
            ),
            (
                KEY_SYNC_COMMIT_INTERVAL,
                settings.sync.auto_commit_interval_secs.to_string(),
            ),
            (
                KEY_SYNC_SYNC_INTERVAL,
                settings.sync.auto_sync_interval_secs.to_string(),
            ),
            (
                KEY_BACKUP_INTERVAL,
                settings.sync.backup.interval_minutes.to_string(),
            ),
            (KEY_BACKUP_EXCLUDE, settings.sync.backup.exclude.join(", ")),
            (KEY_TTS_SPEED, settings.tts.speed.to_string()),
        ]);
        let exports = BTreeMap::from([
            (ExportKind::Html, format!("{vault}/export-html")),
            (ExportKind::Json, format!("{vault}/export-json")),
            (ExportKind::Pdf, format!("{vault}/export-page.pdf")),
            (ExportKind::Zip, format!("{vault}/export.zip")),
        ]);
        let last_valid_theme = ThemeConfig::from_settings(&settings.theme);
        Self {
            settings,
            raw,
            exports,
            import_path: String::new(),
            import_format: ImportFormat::Auto,
            import_dry_run: true,
            passphrase: String::new(),
            errors: BTreeMap::new(),
            last_valid_theme,
        }
    }

    /// Build the fallback form used when the provider cannot load settings.
    ///
    /// Mirrors `settingsDefaults.ts` so the surface still renders and saves a
    /// complete DTO.
    pub fn fallback(vault_path: impl Into<String>) -> Self {
        let vault_path = vault_path.into();
        let settings = SettingsDto {
            vault_path: vault_path.clone(),
            locale: "en".to_string(),
            layout: LayoutSettingsDto::default(),
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
                model: String::new(),
                models: Vec::new(),
                rag_enabled: false,
                rag_chunk_count: 5,
                embedding_dimensions: 0,
                use_llm_gateway_and_auth: false,
                extra_local_hosts: Vec::new(),
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
                remote_url: None,
                branch: "main".to_string(),
                auto_commit_interval_secs: 300,
                auto_sync_interval_secs: 1800,
                ssh_key_path: None,
                commit_template:
                    "stratum({datetime}): {editedfiles} edited, {newfiles} added, {deletedfiles} deleted"
                        .to_string(),
                backup: BackupSettingsDto::default(),
            },
            stt: SttSettingsDto::default(),
            tts: TtsSettingsDto::default(),
            tray: TraySettingsDto::default(),
            voice: VoiceSettingsDto::default(),
            reminders: ReminderSettingsDto::default(),
        };
        Self::from_settings(settings)
    }

    /// Apply one input-registry value to the form.
    ///
    /// Direct string fields are written to the DTO; numeric/list/export fields
    /// stay raw until [`Self::candidate`] parses them.
    pub fn apply_input(&mut self, key: &str, value: String) {
        self.errors.clear();
        match key {
            "vault_path" => self.settings.vault_path = value,
            "theme_primary" => self.settings.theme.primary_color = value,
            "theme_secondary" => self.settings.theme.secondary_color = value,
            KEY_THEME_FONT_SIZE
            | KEY_AI_RAG_CHUNKS
            | KEY_AI_EMBEDDING_DIMENSIONS
            | KEY_AI_EXTRA_HOSTS
            | KEY_RESEARCH_MAX_RESULTS
            | KEY_RESEARCH_MAX_DEPTH
            | KEY_SYNC_COMMIT_INTERVAL
            | KEY_SYNC_SYNC_INTERVAL
            | KEY_BACKUP_INTERVAL
            | KEY_BACKUP_EXCLUDE
            | KEY_TTS_SPEED => {
                self.raw.insert(static_key(key), value);
            }
            "ai_endpoint" => self.settings.ai.endpoint = non_empty(value),
            "ai_api_key" => self.settings.ai.api_key = non_empty(value),
            "ai_model" => self.settings.ai.model = value,
            "research_endpoint" => self.settings.research.searxng_endpoint = value,
            "sync_remote_url" => self.settings.sync.remote_url = non_empty(value),
            "sync_branch" => self.settings.sync.branch = value,
            "sync_ssh_key_path" => self.settings.sync.ssh_key_path = non_empty(value),
            "sync_commit_template" => self.settings.sync.commit_template = value,
            "backup_folder_path" => self.settings.sync.backup.folder_path = non_empty(value),
            "backup_webdav_url" => self.settings.sync.backup.webdav_url = non_empty(value),
            "backup_webdav_username" => {
                self.settings.sync.backup.webdav_username = non_empty(value);
            }
            "backup_webdav_password" => {
                self.settings.sync.backup.webdav_password = non_empty(value);
            }
            "stt_endpoint" => self.settings.stt.endpoint = value,
            "stt_api_key" => self.settings.stt.api_key = non_empty(value),
            "stt_model" => self.settings.stt.model = value,
            "stt_diarize_model" => self.settings.stt.diarize_model = value,
            "stt_language" => self.settings.stt.language = non_empty(value),
            "tts_endpoint" => self.settings.tts.endpoint = value,
            "tts_api_key" => self.settings.tts.api_key = non_empty(value),
            "tts_voice" => self.settings.tts.voice = value,
            "tts_format" => self.settings.tts.format = value,
            "tray_shortcut" => self.settings.tray.quick_capture_shortcut = value,
            "tray_inbox_path" => self.settings.tray.inbox_path = value,
            "voice_target" => self.settings.voice.target = value,
            "import_path" => self.import_path = value,
            "passphrase" => self.passphrase = value,
            "export_html" | "export_json" | "export_pdf" | "export_zip" => {
                if let Some(kind) = ExportKind::ALL
                    .into_iter()
                    .find(|kind| kind.input_key() == key)
                {
                    self.exports.insert(kind, value);
                }
            }
            _ => {}
        }
    }

    /// Build the candidate DTO and collect parse errors.
    ///
    /// Parsing failures keep the previous value and record the matching
    /// validation key, so a save with invalid text is blocked rather than
    /// silently saving a default.
    pub fn candidate(&self) -> Result<SettingsDto, BTreeMap<&'static str, &'static str>> {
        let mut errors = BTreeMap::new();
        let mut out = self.settings.clone();

        out.theme.font_size = parse_u32(
            self.raw_text(KEY_THEME_FONT_SIZE),
            "theme_font_size",
            "settings.validation.range",
            &mut errors,
        )
        .unwrap_or(out.theme.font_size);
        out.ai.rag_chunk_count = parse_usize(
            self.raw_text(KEY_AI_RAG_CHUNKS),
            "ai_rag_chunk_count",
            "settings.validation.range",
            &mut errors,
        )
        .unwrap_or(out.ai.rag_chunk_count);
        out.ai.embedding_dimensions = parse_usize(
            self.raw_text(KEY_AI_EMBEDDING_DIMENSIONS),
            "ai_embedding_dimensions",
            "settings.validation.range",
            &mut errors,
        )
        .unwrap_or(out.ai.embedding_dimensions);
        out.research.max_results = parse_usize(
            self.raw_text(KEY_RESEARCH_MAX_RESULTS),
            "research_max_results",
            "settings.validation.range",
            &mut errors,
        )
        .unwrap_or(out.research.max_results);
        out.research.max_depth = parse_u32(
            self.raw_text(KEY_RESEARCH_MAX_DEPTH),
            "research_max_depth",
            "settings.validation.range",
            &mut errors,
        )
        .unwrap_or(out.research.max_depth);
        out.sync.auto_commit_interval_secs = parse_u64(
            self.raw_text(KEY_SYNC_COMMIT_INTERVAL),
            "sync_commit_interval",
            "settings.validation.commit_interval",
            &mut errors,
        )
        .unwrap_or(out.sync.auto_commit_interval_secs);
        out.sync.auto_sync_interval_secs = parse_u64(
            self.raw_text(KEY_SYNC_SYNC_INTERVAL),
            "sync_sync_interval",
            "settings.validation.sync_interval",
            &mut errors,
        )
        .unwrap_or(out.sync.auto_sync_interval_secs);
        out.sync.backup.interval_minutes = parse_u64(
            self.raw_text(KEY_BACKUP_INTERVAL),
            "backup_interval",
            "settings.validation.positive",
            &mut errors,
        )
        .unwrap_or(out.sync.backup.interval_minutes);
        out.tts.speed = parse_f32(
            self.raw_text(KEY_TTS_SPEED),
            "tts_speed",
            "settings.validation.range",
            &mut errors,
        )
        .unwrap_or(out.tts.speed);
        out.ai.extra_local_hosts = self
            .raw_text(KEY_AI_EXTRA_HOSTS)
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_string)
            .collect();
        out.sync.backup.exclude = self
            .raw_text(KEY_BACKUP_EXCLUDE)
            .split(',')
            .map(str::trim)
            .filter(|entry| !entry.is_empty())
            .map(str::to_string)
            .collect();

        if errors.is_empty() {
            Ok(out)
        } else {
            Err(errors)
        }
    }

    /// Build the live theme candidate from the current edits.
    ///
    /// Returns `None` while either colour is not a valid hex value so a
    /// half-typed value never repaints the shell with a fallback palette. The
    /// font size falls back to the last persisted value when the raw field is
    /// invalid.
    pub fn theme_candidate(&self) -> Option<ThemeConfig> {
        use crate::theme::is_hex_color;

        let primary = &self.settings.theme.primary_color;
        let secondary = &self.settings.theme.secondary_color;
        if !is_hex_color(primary) || !is_hex_color(secondary) {
            return None;
        }
        let font_size = self
            .raw_text(KEY_THEME_FONT_SIZE)
            .trim()
            .parse::<u32>()
            .ok()
            .filter(|size| (8..=48).contains(size))
            .or_else(|| {
                (self.settings.theme.font_size > 0).then_some(self.settings.theme.font_size)
            });
        Some(ThemeConfig {
            primary_hex: primary.clone(),
            secondary_hex: secondary.clone(),
            dark: self.settings.theme.dark_mode,
            font_size: font_size.map(|size| size as f32),
        })
    }

    /// Collect export-path validation errors.
    pub fn export_errors(&self) -> BTreeMap<&'static str, &'static str> {
        let mut errors = BTreeMap::new();
        for kind in ExportKind::ALL {
            if self
                .exports
                .get(&kind)
                .map(|path| path.trim().is_empty())
                .unwrap_or(true)
            {
                errors.insert(kind.input_key(), "settings.validation.export_path");
            }
        }
        errors
    }

    /// The raw text for a key, or an empty string.
    fn raw_text(&self, key: &'static str) -> &str {
        self.raw.get(key).map(String::as_str).unwrap_or("")
    }
}
