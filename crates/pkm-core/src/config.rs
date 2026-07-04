use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Top-level configuration for a Stratum vault.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Path to the vault root (where .md notes live).
    pub vault_path: PathBuf,
    /// Storage backend configuration.
    pub storage: StorageConfig,
    /// Vault layout configuration (page/journal directories).
    pub layout: VaultLayout,
    /// Sync configuration.
    pub sync: SyncConfig,
    /// UI configuration.
    pub theme: ThemeConfig,
    /// AI / LLM provider configuration.
    pub ai: AiConfig,
    /// Web research configuration (SearXNG).
    pub research: ResearchConfig,
    /// Plugin enable/disable.
    pub plugins: Vec<PluginConfig>,
    /// File watcher configuration.
    pub watcher: WatcherConfig,
    /// Graph visualization settings.
    pub graph: GraphConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            vault_path: PathBuf::from("."),
            storage: StorageConfig::default(),
            layout: VaultLayout::default(),
            sync: SyncConfig::default(),
            theme: ThemeConfig::default(),
            ai: AiConfig::default(),
            research: ResearchConfig::default(),
            plugins: Vec::new(),
            watcher: WatcherConfig::default(),
            graph: GraphConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum StorageBackend {
    #[default]
    Hybrid,
    FileOnly,
    Sqlite,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct StorageConfig {
    pub backend: StorageBackend,
    pub auto_save_md: bool,
    pub md_write_delay_ms: u64,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            backend: StorageBackend::default(),
            auto_save_md: true,
            md_write_delay_ms: 500,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct VaultLayout {
    pub pages_dir: String,
    pub journals_dir: String,
}

impl Default for VaultLayout {
    fn default() -> Self {
        Self {
            pages_dir: "pages".to_string(),
            journals_dir: "journals".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SyncConfig {
    /// Sync mode.
    pub mode: SyncMode,
    /// Remote URL (e.g. git@github.com:user/vault.git).
    pub remote_url: Option<String>,
    /// Git branch to sync.
    pub branch: String,
    /// Auto-commit interval in seconds (if mode >= AutoCommit).
    pub auto_commit_interval_secs: u64,
    /// Auto-sync / push-pull interval in seconds (if mode == AutoSync).
    pub auto_sync_interval_secs: u64,
    /// GPG key ID for signing commits (optional).
    pub signing_key: Option<String>,
    /// Path to SSH private key for git authentication.
    /// If set, used for push/pull via git2::Cred::ssh_key.
    /// If None, falls back to ssh-agent.
    pub ssh_key_path: Option<String>,
    /// Commit message template with placeholders:
    /// {datetime}, {editedfiles}, {newfiles}, {deletedfiles}, {count}
    pub commit_template: String,
}

/// Detect the default SSH private key path.
/// Checks ~/.ssh/id_ed25519 first, then ~/.ssh/id_rsa.
/// Returns the path of the first existing key, or None.
pub fn detect_default_ssh_key() -> Option<String> {
    let home = dirs::home_dir()?;
    for candidate in &["id_ed25519", "id_rsa"] {
        let p = home.join(".ssh").join(candidate);
        if p.exists() {
            return Some(p.to_string_lossy().to_string());
        }
    }
    None
}

impl Default for SyncConfig {
    fn default() -> Self {
        let default_template = concat!(
            "stratum({datetime}): ",
            "{editedfiles} edited, {newfiles} added, {deletedfiles} deleted"
        );
        Self {
            mode: SyncMode::Manual,
            remote_url: None,
            branch: "main".to_string(),
            auto_commit_interval_secs: 300,
            auto_sync_interval_secs: 1800,
            signing_key: None,
            ssh_key_path: detect_default_ssh_key(),
            commit_template: default_template.to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncMode {
    /// User manually triggers sync.
    Manual,
    /// Auto-commit on save, manual push/pull.
    AutoCommit,
    /// Auto-commit + periodic push/pull.
    AutoSync,
    /// Background daemon mode.
    Background,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeConfig {
    pub dark_mode: bool,
    pub font_size: u32,
    pub font_family: String,
    pub show_line_numbers: bool,
    pub sidebar_width: u32,
    /// Primary color as hex (accent, buttons, interactive elements).
    pub primary_color: String,
    /// Secondary color as hex (backgrounds, borders, text).
    pub secondary_color: String,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            dark_mode: true,
            font_size: 16,
            font_family: "Inter, sans-serif".to_string(),
            show_line_numbers: false,
            sidebar_width: 280,
            primary_color: "#f97316".to_string(),
            secondary_color: "#6b7280".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AiConfig {
    /// LLM provider type.
    pub provider: AiProvider,
    /// API endpoint (for OpenAI-compatible APIs).
    pub endpoint: Option<String>,
    /// API key (stored in config, masked in UI).
    pub api_key: Option<String>,
    /// Chat model name (backward compat).
    pub model: String,
    /// Configured models with capabilities.
    pub models: Vec<AiModelConfig>,
    /// Enable RAG for chat.
    pub rag_enabled: bool,
    /// Max chunks to retrieve for RAG context.
    pub rag_chunk_count: usize,
    /// Embedding model path (for local ONNX/llama.cpp).
    pub embedding_model_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AiModelConfig {
    pub name: String,
    pub capabilities: Vec<String>, // "chat", "embedding", "tts"
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            provider: AiProvider::Ollama,
            endpoint: Some("http://localhost:11434".to_string()),
            api_key: None,
            model: "llama3.2".to_string(),
            models: Vec::new(),
            rag_enabled: true,
            rag_chunk_count: 5,
            embedding_model_path: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AiProvider {
    Ollama,
    OpenAI,
    Anthropic,
    Google,
    #[serde(rename = "z.ai")]
    Zai,
    Custom,
    CustomOpenAI,
    CustomAnthropic,
}

/// Graph visualization configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GraphConfig {
    pub show_connected: bool,
    pub show_orphaned: bool,
    pub show_tags: bool,
    pub charge_strength: f64,
    pub link_distance: f64,
    pub alpha_decay: f64,
    pub velocity_decay: f64,
    pub link_curvature: f64,
}

impl Default for GraphConfig {
    fn default() -> Self {
        Self {
            show_connected: true,
            show_orphaned: true,
            show_tags: true,
            charge_strength: -8.0,
            link_distance: 40.0,
            alpha_decay: 0.08,
            velocity_decay: 0.3,
            link_curvature: 0.15,
        }
    }
}

/// Configuration for web research (SearXNG).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ResearchConfig {
    /// SearXNG endpoint URL.
    pub searxng_endpoint: String,
    /// Max search results to read per query.
    pub max_results: usize,
    /// Max research depth (search-read cycles).
    pub max_depth: u32,
}

impl Default for ResearchConfig {
    fn default() -> Self {
        Self {
            searxng_endpoint: "http://localhost:8888".to_string(),
            max_results: 3,
            max_depth: 2,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    pub name: String,
    pub enabled: bool,
    pub wasm_path: PathBuf,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WatcherConfig {
    pub enabled: bool,
    pub debounce_ms: u64,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            debounce_ms: 500,
        }
    }
}

impl Config {
    /// Load config from a TOML file.
    pub fn load(path: impl Into<PathBuf>) -> Result<Self, ConfigError> {
        let path: PathBuf = path.into();
        let content = std::fs::read_to_string(&path)
            .map_err(|e| ConfigError::Io(format!("Failed to read {}: {}", path.display(), e)))?;
        toml::from_str(&content).map_err(|e| ConfigError::Parse(e.to_string()))
    }

    /// Save config to a TOML file.
    pub fn save(&self, path: impl Into<PathBuf>) -> Result<(), ConfigError> {
        let path: PathBuf = path.into();
        let content =
            toml::to_string_pretty(self).map_err(|e| ConfigError::Parse(e.to_string()))?;
        std::fs::write(&path, &content)
            .map_err(|e| ConfigError::Io(format!("Failed to write {}: {}", path.display(), e)))?;
        Ok(())
    }

    /// Get the .pkm directory path inside the vault.
    pub fn pkm_dir(&self) -> PathBuf {
        self.vault_path.join(".pkm")
    }

    /// Get the cache database path.
    pub fn cache_db_path(&self) -> PathBuf {
        self.pkm_dir().join("cache.db")
    }

    /// Get the Tantivy search index path.
    pub fn search_index_path(&self) -> PathBuf {
        self.pkm_dir().join("search.idx")
    }

    /// Get the config file path inside the vault.
    pub fn config_file_path(&self) -> PathBuf {
        self.pkm_dir().join("config.toml")
    }

    /// Get the history directory path.
    pub fn history_dir(&self) -> PathBuf {
        self.pkm_dir().join("history")
    }
}

#[derive(Debug)]
pub enum ConfigError {
    Io(String),
    Parse(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Io(msg) => write!(f, "IO error: {}", msg),
            ConfigError::Parse(msg) => write!(f, "Parse error: {}", msg),
        }
    }
}

impl std::error::Error for ConfigError {}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_config_defaults() {
        let cfg = Config::default();
        assert_eq!(cfg.sync.mode, SyncMode::Manual);
        assert!(cfg.theme.dark_mode);
        assert_eq!(cfg.theme.font_size, 16);
        assert_eq!(cfg.watcher.debounce_ms, 500);
    }

    #[test]
    fn test_config_save_and_load() {
        let dir = TempDir::new().unwrap();
        let config_path = dir.path().join("config.toml");

        let cfg = Config {
            vault_path: PathBuf::from("/tmp/test-vault"),
            sync: SyncConfig {
                mode: SyncMode::AutoSync,
                remote_url: Some("git@github.com:user/vault.git".to_string()),
                auto_sync_interval_secs: 900,
                ..Default::default()
            },
            theme: ThemeConfig {
                dark_mode: false,
                font_size: 18,
                ..Default::default()
            },
            ..Default::default()
        };

        cfg.save(&config_path).unwrap();

        let loaded = Config::load(&config_path).unwrap();
        assert_eq!(loaded.vault_path, PathBuf::from("/tmp/test-vault"));
        assert_eq!(loaded.sync.mode, SyncMode::AutoSync);
        assert_eq!(
            loaded.sync.remote_url,
            Some("git@github.com:user/vault.git".to_string())
        );
        assert_eq!(loaded.sync.auto_sync_interval_secs, 900);
        assert!(!loaded.theme.dark_mode);
        assert_eq!(loaded.theme.font_size, 18);
    }

    #[test]
    fn test_config_pkm_paths() {
        let cfg = Config {
            vault_path: PathBuf::from("/home/user/vault"),
            ..Default::default()
        };
        assert_eq!(cfg.pkm_dir(), PathBuf::from("/home/user/vault/.pkm"));
        assert_eq!(
            cfg.cache_db_path(),
            PathBuf::from("/home/user/vault/.pkm/cache.db")
        );
        assert_eq!(
            cfg.search_index_path(),
            PathBuf::from("/home/user/vault/.pkm/search.idx")
        );
    }

    #[test]
    fn test_config_parse_error() {
        let dir = TempDir::new().unwrap();
        let bad_path = dir.path().join("nonexistent.toml");
        let result = Config::load(&bad_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_sync_mode_serde() {
        let modes = [
            (SyncMode::Manual, "Manual"),
            (SyncMode::AutoCommit, "AutoCommit"),
            (SyncMode::AutoSync, "AutoSync"),
            (SyncMode::Background, "Background"),
        ];
        for (mode, expected) in &modes {
            let serialized = serde_yaml::to_string(mode).unwrap();
            assert!(serialized.contains(expected));
            let deserialized: SyncMode = serde_yaml::from_str(&serialized).unwrap();
            assert_eq!(deserialized, *mode);
        }
    }

    #[test]
    fn test_ai_config_defaults() {
        let ai = AiConfig::default();
        assert_eq!(ai.provider, AiProvider::Ollama);
        assert_eq!(ai.model, "llama3.2");
        assert!(ai.rag_enabled);
    }

    #[test]
    fn test_watcher_config_defaults() {
        let w = WatcherConfig::default();
        assert!(w.enabled);
        assert_eq!(w.debounce_ms, 500);
    }

    #[test]
    fn test_graph_config_link_curvature_default() {
        let cfg = GraphConfig::default();
        assert_eq!(cfg.link_curvature, 0.15);
    }

    #[test]
    fn test_graph_config_serde_round_trip() {
        let cfg = GraphConfig {
            link_curvature: 0.3,
            ..Default::default()
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let deserialized: GraphConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.link_curvature, 0.3);
    }

    #[test]
    fn test_config_with_plugins() {
        let cfg = Config {
            plugins: vec![PluginConfig {
                name: "my-plugin".to_string(),
                enabled: true,
                wasm_path: PathBuf::from("/tmp/plugin.wasm"),
                permissions: vec!["file:read".to_string(), "network".to_string()],
            }],
            ..Default::default()
        };
        assert_eq!(cfg.plugins.len(), 1);
        assert_eq!(cfg.plugins[0].name, "my-plugin");
        assert!(cfg.plugins[0].enabled);
    }
}
