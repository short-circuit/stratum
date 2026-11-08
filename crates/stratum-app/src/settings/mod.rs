//! Settings commands: read and persist the vault configuration.
//!
//! DTOs live in [`dto`], graph-only persistence in [`graph`], model discovery in
//! [`models`] and the onboarding flags in [`onboarding`].

pub mod dto;
pub mod graph;
pub mod models;
pub mod onboarding;

pub use dto::{
    AiModelDto, AiSettingsDto, BackupSettingsDto, GraphSettingsDto, LayoutSettingsDto,
    ReminderSettingsDto, ResearchSettingsDto, SettingsDto, SttSettingsDto, SyncSettingsDto,
    ThemeSettingsDto, TraySettingsDto, TtsSettingsDto, VoiceSettingsDto,
};

use crate::state::StratumService;
use dto::mask_api_key;

impl StratumService {
    /// Read the effective settings for the current vault.
    pub async fn get_settings(&self) -> Result<SettingsDto, String> {
        let state = self.lock()?;
        let config_path = state.vault_path.join(".pkm").join("config.toml");

        let config = if config_path.exists() {
            pkm_core::Config::load(&config_path).map_err(|e| e.to_string())?
        } else {
            pkm_core::Config {
                vault_path: state.vault_path.clone(),
                ..Default::default()
            }
        };

        // FM-14: an env var that is present but empty is the same as unset.
        let env_non_empty = |name: &str| {
            std::env::var(name)
                .map(|v| !v.trim().is_empty())
                .unwrap_or(false)
        };
        let api_key_from_env = match config.ai.provider {
            pkm_core::AiProvider::OpenAI | pkm_core::AiProvider::CustomOpenAI => {
                env_non_empty("OPENAI_API_KEY")
            }
            pkm_core::AiProvider::Anthropic | pkm_core::AiProvider::CustomAnthropic => {
                env_non_empty("ANTHROPIC_API_KEY")
            }
            pkm_core::AiProvider::Google => env_non_empty("GOOGLE_API_KEY"),
            _ => false,
        };

        Ok(SettingsDto {
            vault_path: config.vault_path.to_string_lossy().to_string(),
            locale: config.locale.clone(),
            layout: LayoutSettingsDto {
                pages_view: config.layout.pages_view.clone(),
            },
            theme: ThemeSettingsDto {
                dark_mode: config.theme.dark_mode,
                primary_color: config.theme.primary_color.clone(),
                secondary_color: config.theme.secondary_color.clone(),
                font_size: config.theme.font_size,
            },
            ai: AiSettingsDto {
                provider: match config.ai.provider {
                    pkm_core::AiProvider::Ollama => "ollama".into(),
                    pkm_core::AiProvider::OpenAI => "openai".into(),
                    pkm_core::AiProvider::Anthropic => "anthropic".into(),
                    pkm_core::AiProvider::Google => "google".into(),
                    pkm_core::AiProvider::Zai => "zai".into(),
                    pkm_core::AiProvider::Custom => "custom".into(),
                    pkm_core::AiProvider::CustomOpenAI => "custom-openai".into(),
                    pkm_core::AiProvider::CustomAnthropic => "custom-anthropic".into(),
                },
                endpoint: config.ai.endpoint,
                api_key: mask_api_key(&config.ai.api_key),
                api_key_from_env,
                model: config.ai.model,
                models: config
                    .ai
                    .models
                    .iter()
                    .map(|m| AiModelDto {
                        name: m.name.clone(),
                        capabilities: m.capabilities.clone(),
                    })
                    .collect(),
                rag_enabled: config.ai.rag_enabled,
                rag_chunk_count: config.ai.rag_chunk_count,
                embedding_dimensions: config.ai.embedding_dimensions,
                use_llm_gateway_and_auth: config.ai.use_llm_gateway_and_auth,
                extra_local_hosts: config.ai.extra_local_hosts,
                last_discovered: config.ai.last_discovered,
            },
            research: ResearchSettingsDto {
                searxng_endpoint: config.research.searxng_endpoint.clone(),
                max_results: config.research.max_results,
                max_depth: config.research.max_depth,
            },
            graph: GraphSettingsDto {
                show_connected: config.graph.show_connected,
                show_orphaned: config.graph.show_orphaned,
                show_tags: config.graph.show_tags,
                charge_strength: config.graph.charge_strength,
                link_distance: config.graph.link_distance,
                alpha_decay: config.graph.alpha_decay,
                velocity_decay: config.graph.velocity_decay,
                link_curvature: config.graph.link_curvature,
                use_3d: config.graph.use_3d,
            },
            sync: SyncSettingsDto {
                mode: match config.sync.mode {
                    pkm_core::SyncMode::Manual => "manual".into(),
                    pkm_core::SyncMode::AutoCommit => "auto_commit".into(),
                    pkm_core::SyncMode::AutoSync => "auto_sync".into(),
                    pkm_core::SyncMode::Background => "background".into(),
                },
                remote_url: config.sync.remote_url.clone(),
                branch: config.sync.branch.clone(),
                auto_commit_interval_secs: config.sync.auto_commit_interval_secs,
                auto_sync_interval_secs: config.sync.auto_sync_interval_secs,
                ssh_key_path: config.sync.ssh_key_path.clone(),
                commit_template: config.sync.commit_template.clone(),
                backup: BackupSettingsDto {
                    enabled: config.sync.backup.enabled,
                    mode: match config.sync.backup.mode {
                        pkm_core::BackupMode::Folder => "folder".into(),
                        pkm_core::BackupMode::Webdav => "webdav".into(),
                        pkm_core::BackupMode::Both => "both".into(),
                    },
                    interval_minutes: config.sync.backup.interval_minutes,
                    exclude: config.sync.backup.exclude.clone(),
                    webdav_url: config.sync.backup.webdav.url.clone(),
                    webdav_username: config.sync.backup.webdav.username.clone(),
                    webdav_password: mask_api_key(&config.sync.backup.webdav.password),
                    folder_path: config.sync.backup.folder.path.clone(),
                },
            },
            stt: SttSettingsDto {
                endpoint: config.stt.endpoint.clone(),
                api_key: mask_api_key(&config.stt.api_key),
                model: config.stt.model.clone(),
                diarize_model: config.stt.diarize_model.clone(),
                language: config.stt.language.clone(),
                diarize: config.stt.diarize,
                auto_summarize: config.stt.auto_summarize,
                auto_identify: config.stt.auto_identify,
                use_llm_gateway_and_auth: config.stt.use_llm_gateway_and_auth,
            },
            tts: TtsSettingsDto {
                endpoint: config.tts.endpoint.clone(),
                api_key: mask_api_key(&config.tts.api_key),
                voice: config.tts.voice.clone(),
                format: config.tts.format.clone(),
                speed: config.tts.speed,
                use_llm_gateway_and_auth: config.tts.use_llm_gateway_and_auth,
            },
            voice: VoiceSettingsDto {
                enabled: config.voice.enabled,
                target: config.voice.target.clone(),
            },
            tray: TraySettingsDto {
                enabled: config.tray.enabled,
                close_to_tray: config.tray.close_to_tray,
                quick_capture_shortcut: config.tray.quick_capture_shortcut.clone(),
                capture_target: match config.tray.capture_target {
                    pkm_core::CaptureTarget::Inbox => "inbox".into(),
                    pkm_core::CaptureTarget::Journal => "journal".into(),
                },
                inbox_path: config.tray.inbox_path.clone(),
            },
            reminders: ReminderSettingsDto {
                enabled: config.reminders.enabled,
                sound: config.reminders.sound,
            },
        })
    }

    /// Persist settings to `<vault>/.pkm/config.toml` and return the saved
    /// config so hosts can apply UI-only side effects (e.g. tray state).
    pub async fn save_settings(&self, settings: SettingsDto) -> Result<pkm_core::Config, String> {
        let state = self.lock()?;
        let config_path = state.vault_path.join(".pkm").join("config.toml");

        // Check if the API key from the frontend is masked — if so, preserve the
        // existing key on disk rather than overwriting with the masked placeholder.
        let is_key_masked = settings
            .ai
            .api_key
            .as_ref()
            .is_some_and(|k| k.contains("****"));
        let api_key = if is_key_masked {
            // Load the existing key from disk to avoid saving the masked placeholder
            if config_path.exists() {
                pkm_core::Config::load(&config_path)
                    .ok()
                    .and_then(|c| c.ai.api_key)
            } else {
                None
            }
        } else {
            settings.ai.api_key
        };

        let provider = match settings.ai.provider.as_str() {
            "openai" => pkm_core::AiProvider::OpenAI,
            "anthropic" => pkm_core::AiProvider::Anthropic,
            "google" => pkm_core::AiProvider::Google,
            "zai" => pkm_core::AiProvider::Zai,
            "custom" => pkm_core::AiProvider::Custom,
            "custom-openai" => pkm_core::AiProvider::CustomOpenAI,
            "custom-anthropic" => pkm_core::AiProvider::CustomAnthropic,
            _ => pkm_core::AiProvider::Ollama,
        };

        let sync_mode = match settings.sync.mode.as_str() {
            "auto_commit" => pkm_core::SyncMode::AutoCommit,
            "auto_sync" => pkm_core::SyncMode::AutoSync,
            "background" => pkm_core::SyncMode::Background,
            _ => pkm_core::SyncMode::Manual,
        };

        let capture_target = match settings.tray.capture_target.as_str() {
            "inbox" => pkm_core::CaptureTarget::Inbox,
            _ => pkm_core::CaptureTarget::Journal,
        };

        // Preserve custom layout directories (pages_dir/journals_dir/recordings_dir)
        // that are not exposed in the DTO. When a config already exists, load its
        // `layout` and overwrite only `pages_view`; otherwise start from defaults.
        let pages_view = settings.layout.pages_view.clone();
        let layout = if config_path.exists() {
            let mut existing = pkm_core::Config::load(&config_path)
                .ok()
                .map(|c| c.layout)
                .unwrap_or_default();
            existing.pages_view = pages_view;
            existing
        } else {
            pkm_core::VaultLayout {
                pages_view,
                ..Default::default()
            }
        };

        let config = pkm_core::Config {
            vault_path: std::path::PathBuf::from(&settings.vault_path),
            locale: settings.locale.clone(),
            theme: pkm_core::ThemeConfig {
                dark_mode: settings.theme.dark_mode,
                primary_color: settings.theme.primary_color.clone(),
                secondary_color: settings.theme.secondary_color.clone(),
                font_size: settings.theme.font_size,
                ..pkm_core::ThemeConfig::default()
            },
            ai: pkm_core::AiConfig {
                provider,
                endpoint: settings.ai.endpoint,
                api_key,
                model: settings.ai.model,
                models: settings
                    .ai
                    .models
                    .iter()
                    .map(|m| pkm_core::AiModelConfig {
                        name: m.name.clone(),
                        capabilities: m.capabilities.clone(),
                    })
                    .collect(),
                rag_enabled: settings.ai.rag_enabled,
                rag_chunk_count: settings.ai.rag_chunk_count,
                embedding_dimensions: settings.ai.embedding_dimensions,
                use_llm_gateway_and_auth: settings.ai.use_llm_gateway_and_auth,
                extra_local_hosts: settings.ai.extra_local_hosts,
                last_discovered: settings.ai.last_discovered,
            },
            research: pkm_core::ResearchConfig {
                searxng_endpoint: settings.research.searxng_endpoint,
                max_results: settings.research.max_results,
                max_depth: settings.research.max_depth,
            },
            graph: pkm_core::GraphConfig {
                show_connected: settings.graph.show_connected,
                show_orphaned: settings.graph.show_orphaned,
                show_tags: settings.graph.show_tags,
                charge_strength: settings.graph.charge_strength,
                link_distance: settings.graph.link_distance,
                alpha_decay: settings.graph.alpha_decay,
                velocity_decay: settings.graph.velocity_decay,
                link_curvature: settings.graph.link_curvature,
                use_3d: settings.graph.use_3d,
            },
            sync: pkm_core::SyncConfig {
                mode: sync_mode,
                remote_url: settings.sync.remote_url,
                branch: settings.sync.branch,
                auto_commit_interval_secs: settings.sync.auto_commit_interval_secs,
                auto_sync_interval_secs: settings.sync.auto_sync_interval_secs,
                ssh_key_path: settings.sync.ssh_key_path,
                commit_template: settings.sync.commit_template,
                backup: pkm_core::BackupConfig {
                    enabled: settings.sync.backup.enabled,
                    mode: match settings.sync.backup.mode.as_str() {
                        "webdav" => pkm_core::BackupMode::Webdav,
                        "both" => pkm_core::BackupMode::Both,
                        _ => pkm_core::BackupMode::Folder,
                    },
                    interval_minutes: settings.sync.backup.interval_minutes,
                    exclude: settings.sync.backup.exclude.clone(),
                    webdav: pkm_core::WebdavBackupConfig {
                        url: settings.sync.backup.webdav_url,
                        username: settings.sync.backup.webdav_username,
                        password: {
                            // Preserve the stored password when the frontend sent a
                            // masked one (same pattern as stt/tts api_key).
                            let masked = settings
                                .sync
                                .backup
                                .webdav_password
                                .as_ref()
                                .is_some_and(|k| k.contains("****"));
                            if masked {
                                if config_path.exists() {
                                    pkm_core::Config::load(&config_path)
                                        .ok()
                                        .and_then(|c| c.sync.backup.webdav.password)
                                } else {
                                    None
                                }
                            } else {
                                settings.sync.backup.webdav_password
                            }
                        },
                    },
                    folder: pkm_core::FolderBackupConfig {
                        path: settings.sync.backup.folder_path,
                    },
                },
                ..pkm_core::SyncConfig::default()
            },
            stt: pkm_core::SttConfig {
                endpoint: settings.stt.endpoint,
                api_key: {
                    // Preserve the stored key when the frontend sent a masked one.
                    let masked = settings
                        .stt
                        .api_key
                        .as_ref()
                        .is_some_and(|k| k.contains("****"));
                    if masked {
                        if config_path.exists() {
                            pkm_core::Config::load(&config_path)
                                .ok()
                                .and_then(|c| c.stt.api_key)
                        } else {
                            None
                        }
                    } else {
                        settings.stt.api_key
                    }
                },
                model: settings.stt.model,
                diarize_model: settings.stt.diarize_model,
                language: settings.stt.language,
                diarize: settings.stt.diarize,
                auto_summarize: settings.stt.auto_summarize,
                auto_identify: settings.stt.auto_identify,
                use_llm_gateway_and_auth: settings.stt.use_llm_gateway_and_auth,
            },
            tts: pkm_core::TtsConfig {
                endpoint: settings.tts.endpoint,
                api_key: {
                    // Preserve the stored key when the frontend sent a masked one.
                    let masked = settings
                        .tts
                        .api_key
                        .as_ref()
                        .is_some_and(|k| k.contains("****"));
                    if masked {
                        if config_path.exists() {
                            pkm_core::Config::load(&config_path)
                                .ok()
                                .and_then(|c| c.tts.api_key)
                        } else {
                            None
                        }
                    } else {
                        settings.tts.api_key
                    }
                },
                voice: settings.tts.voice,
                format: settings.tts.format,
                speed: settings.tts.speed,
                use_llm_gateway_and_auth: settings.tts.use_llm_gateway_and_auth,
            },
            voice: pkm_core::VoiceCaptureConfig {
                enabled: settings.voice.enabled,
                target: settings.voice.target,
            },
            tray: pkm_core::TrayConfig {
                enabled: settings.tray.enabled,
                close_to_tray: settings.tray.close_to_tray,
                quick_capture_shortcut: settings.tray.quick_capture_shortcut,
                capture_target,
                inbox_path: settings.tray.inbox_path,
            },
            reminders: pkm_core::ReminderConfig {
                enabled: settings.reminders.enabled,
                sound: settings.reminders.sound,
            },
            layout,
            ..pkm_core::Config::default()
        };

        config.save(&config_path).map_err(|e| e.to_string())?;
        Ok(config)
    }
}
