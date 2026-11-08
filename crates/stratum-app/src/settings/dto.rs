//! Settings DTOs and shared formatting helpers.
//!
//! Field names and shapes are ABI-frozen: the frontend and every host depend on
//! the exact JSON produced by these structs.

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SettingsDto {
    pub vault_path: String,
    pub locale: String,
    #[serde(default)]
    pub layout: LayoutSettingsDto,
    pub theme: ThemeSettingsDto,
    pub ai: AiSettingsDto,
    pub research: ResearchSettingsDto,
    pub graph: GraphSettingsDto,
    pub sync: SyncSettingsDto,
    pub stt: SttSettingsDto,
    pub tts: TtsSettingsDto,
    #[serde(default)]
    pub tray: TraySettingsDto,
    #[serde(default)]
    pub voice: VoiceSettingsDto,
    #[serde(default)]
    pub reminders: ReminderSettingsDto,
}

/// Vault layout settings, mirroring the `layout` section of `pkm_core::Config`.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct LayoutSettingsDto {
    /// "flat" | "tree" — page list rendering mode for the current vault.
    pub pages_view: String,
}

impl Default for LayoutSettingsDto {
    fn default() -> Self {
        Self {
            pages_view: "flat".to_string(),
        }
    }
}

/// Block reminder settings, mirroring `pkm_core::ReminderConfig`.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReminderSettingsDto {
    /// Whether block reminders are enabled (scheduler active + notifications).
    pub enabled: bool,
    /// Whether notification sound is enabled.
    pub sound: bool,
}

impl Default for ReminderSettingsDto {
    fn default() -> Self {
        // Keep in sync with `ReminderConfig::default()` in pkm-core.
        Self {
            enabled: true,
            sound: true,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TraySettingsDto {
    pub enabled: bool,
    pub close_to_tray: bool,
    pub quick_capture_shortcut: String,
    /// "journal" | "inbox"
    pub capture_target: String,
    pub inbox_path: String,
}

impl Default for TraySettingsDto {
    fn default() -> Self {
        Self {
            enabled: true,
            close_to_tray: true,
            quick_capture_shortcut: "CmdOrCtrl+Shift+Space".to_string(),
            capture_target: "journal".to_string(),
            inbox_path: "Inbox.md".to_string(),
        }
    }
}

/// One-tap voice-capture (E9.F4) feature settings, mirroring
/// `pkm_core::VoiceCaptureConfig`.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VoiceSettingsDto {
    /// Whether the one-tap voice-capture button is shown in the Journal.
    pub enabled: bool,
    /// Insertion target, e.g. "journal:today".
    pub target: String,
}

impl Default for VoiceSettingsDto {
    fn default() -> Self {
        // Keep in sync with `VoiceCaptureConfig::default()` in pkm-core.
        Self {
            enabled: true,
            target: "journal:today".to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TtsSettingsDto {
    pub endpoint: String,
    pub api_key: Option<String>,
    pub voice: String,
    pub format: String,
    pub speed: f32,
    #[serde(default)]
    pub use_llm_gateway_and_auth: bool,
}

impl Default for TtsSettingsDto {
    fn default() -> Self {
        Self {
            endpoint: String::new(),
            api_key: None,
            voice: "alloy".to_string(),
            format: "mp3".to_string(),
            speed: 1.0,
            use_llm_gateway_and_auth: false,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct SttSettingsDto {
    pub endpoint: String,
    pub api_key: Option<String>,
    pub model: String,
    pub diarize_model: String,
    pub language: Option<String>,
    pub diarize: bool,
    pub auto_summarize: bool,
    pub auto_identify: bool,
    #[serde(default)]
    pub use_llm_gateway_and_auth: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GraphSettingsDto {
    pub show_connected: bool,
    pub show_orphaned: bool,
    pub show_tags: bool,
    pub charge_strength: f64,
    pub link_distance: f64,
    pub alpha_decay: f64,
    pub velocity_decay: f64,
    pub link_curvature: f64,
    /// Opt-in 3D rendering (mobile). Desktop always renders 3D; this flag is
    /// only consumed by the mobile variant (see src/lib/graph3d.ts for the
    /// WebGL-capability fallback).
    #[serde(default)]
    pub use_3d: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResearchSettingsDto {
    pub searxng_endpoint: String,
    pub max_results: usize,
    pub max_depth: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ThemeSettingsDto {
    pub dark_mode: bool,
    pub primary_color: String,
    pub secondary_color: String,
    pub font_size: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AiSettingsDto {
    pub provider: String,
    pub endpoint: Option<String>,
    pub api_key: Option<String>,
    #[serde(default)]
    pub api_key_from_env: bool,
    pub model: String,
    pub models: Vec<AiModelDto>,
    pub rag_enabled: bool,
    pub rag_chunk_count: usize,
    /// Expected embedding vector dimensionality; `0` = infer from response.
    pub embedding_dimensions: usize,
    #[serde(default)]
    pub use_llm_gateway_and_auth: bool,
    #[serde(default)]
    pub extra_local_hosts: Vec<String>,
    #[serde(default)]
    pub last_discovered: Vec<pkm_core::DiscoveredEndpoint>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AiModelDto {
    pub name: String,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SyncSettingsDto {
    pub mode: String,
    pub remote_url: Option<String>,
    pub branch: String,
    pub auto_commit_interval_secs: u64,
    pub auto_sync_interval_secs: u64,
    pub ssh_key_path: Option<String>,
    pub commit_template: String,
    #[serde(default)]
    pub backup: BackupSettingsDto,
}

/// Cloud/folder backup settings (E10.F3), mirroring `pkm_core::BackupConfig`.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BackupSettingsDto {
    #[serde(default)]
    pub enabled: bool,
    /// `"folder"` | `"webdav"` | `"both"`
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub interval_minutes: u64,
    #[serde(default)]
    pub exclude: Vec<String>,
    #[serde(default)]
    pub webdav_url: Option<String>,
    #[serde(default)]
    pub webdav_username: Option<String>,
    #[serde(default)]
    pub webdav_password: Option<String>,
    #[serde(default)]
    pub folder_path: Option<String>,
}

impl Default for BackupSettingsDto {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: "folder".to_string(),
            interval_minutes: 60,
            exclude: pkm_core::default_backup_exclude(),
            webdav_url: None,
            webdav_username: None,
            webdav_password: None,
            folder_path: None,
        }
    }
}

/// Mask an API key for safe display, keeping only the first 3 and last 4 chars.
pub(crate) fn mask_api_key(key: &Option<String>) -> Option<String> {
    match key {
        Some(k) if k.len() > 8 => {
            let masked = format!("{}****{}", &k[..3], &k[k.len() - 4..]);
            Some(masked)
        }
        Some(_) => Some("****".to_string()),
        None => None,
    }
}

/// Human-readable provider name for error messages.
pub(crate) fn provider_display(provider: &pkm_core::AiProvider) -> &'static str {
    match provider {
        pkm_core::AiProvider::Ollama => "Ollama",
        pkm_core::AiProvider::OpenAI => "OpenAI",
        pkm_core::AiProvider::Anthropic => "Anthropic",
        pkm_core::AiProvider::Google => "Google",
        pkm_core::AiProvider::Zai => "Z-AI",
        pkm_core::AiProvider::Custom => "Custom",
        pkm_core::AiProvider::CustomOpenAI => "Custom OpenAI",
        pkm_core::AiProvider::CustomAnthropic => "Custom Anthropic",
    }
}
