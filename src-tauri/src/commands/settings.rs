//! Settings and AI configuration commands.

use crate::commands::vault::AppState;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SettingsDto {
    pub vault_path: String,
    pub theme: ThemeSettingsDto,
    pub ai: AiSettingsDto,
    pub research: ResearchSettingsDto,
    pub graph: GraphSettingsDto,
    pub sync: SyncSettingsDto,
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
}

/// Mask an API key for safe display, keeping only the first 3 and last 4 chars.
fn mask_api_key(key: &Option<String>) -> Option<String> {
    match key {
        Some(k) if k.len() > 8 => {
            let masked = format!("{}****{}", &k[..3], &k[k.len() - 4..]);
            Some(masked)
        }
        Some(_) => Some("****".to_string()),
        None => None,
    }
}

#[tauri::command]
pub async fn get_settings(state: tauri::State<'_, AppState>) -> Result<SettingsDto, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let config_path = state.vault_path.join(".pkm").join("config.toml");

    let config = if config_path.exists() {
        pkm_core::Config::load(&config_path).map_err(|e| e.to_string())?
    } else {
        pkm_core::Config {
            vault_path: state.vault_path.clone(),
            ..Default::default()
        }
    };

    let api_key_from_env = match config.ai.provider {
        pkm_core::AiProvider::OpenAI | pkm_core::AiProvider::CustomOpenAI => {
            std::env::var("OPENAI_API_KEY").is_ok()
        }
        pkm_core::AiProvider::Anthropic | pkm_core::AiProvider::CustomAnthropic => {
            std::env::var("ANTHROPIC_API_KEY").is_ok()
        }
        pkm_core::AiProvider::Google => std::env::var("GOOGLE_API_KEY").is_ok(),
        _ => false,
    };

    Ok(SettingsDto {
        vault_path: config.vault_path.to_string_lossy().to_string(),
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
        },
    })
}

#[tauri::command]
pub async fn save_settings(
    settings: SettingsDto,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let state = state.lock().map_err(|e| e.to_string())?;
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

    let config = pkm_core::Config {
        vault_path: std::path::PathBuf::from(&settings.vault_path),
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
            embedding_model_path: None,
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
        },
        sync: pkm_core::SyncConfig {
            mode: sync_mode,
            remote_url: settings.sync.remote_url,
            branch: settings.sync.branch,
            auto_commit_interval_secs: settings.sync.auto_commit_interval_secs,
            auto_sync_interval_secs: settings.sync.auto_sync_interval_secs,
            ssh_key_path: settings.sync.ssh_key_path,
            commit_template: settings.sync.commit_template,
            ..pkm_core::SyncConfig::default()
        },
        ..pkm_core::Config::default()
    };

    config.save(&config_path).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn save_graph_settings(
    graph: GraphSettingsDto,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let config_path = state.vault_path.join(".pkm").join("config.toml");

    let mut config = if config_path.exists() {
        pkm_core::Config::load(&config_path).map_err(|e| e.to_string())?
    } else {
        pkm_core::Config {
            vault_path: state.vault_path.clone(),
            ..Default::default()
        }
    };

    config.graph = pkm_core::GraphConfig {
        show_connected: graph.show_connected,
        show_orphaned: graph.show_orphaned,
        show_tags: graph.show_tags,
        charge_strength: graph.charge_strength,
        link_distance: graph.link_distance,
        alpha_decay: graph.alpha_decay,
        velocity_decay: graph.velocity_decay,
        link_curvature: graph.link_curvature,
    };

    config.save(&config_path).map_err(|e| e.to_string())?;
    Ok(())
}

/// Fetch available models from the provider's API.
#[tauri::command]
pub async fn fetch_models(state: tauri::State<'_, AppState>) -> Result<Vec<String>, String> {
    // Extract config data before async operation (MutexGuard is not Send)
    let (endpoint, api_key) = {
        let state = state.lock().map_err(|e| e.to_string())?;
        let config_path = state.vault_path.join(".pkm").join("config.toml");

        let config = if config_path.exists() {
            pkm_core::Config::load(&config_path).map_err(|e| e.to_string())?
        } else {
            return Err("No config found. Save settings first.".into());
        };

        (
            config
                .ai
                .endpoint
                .clone()
                .unwrap_or_else(|| "http://localhost:11434".into()),
            config.ai.effective_api_key(),
        )
    };

    // Validate endpoint URL to prevent SSRF
    pkm_core::endpoint::validate_endpoint_safe(&endpoint).map_err(|e| e.to_string())?;

    let models_url = format!("{}/v1/models", endpoint.trim_end_matches('/'));

    // Validate the full models URL too
    pkm_core::endpoint::validate_endpoint_safe(&models_url).map_err(|e| e.to_string())?;

    let client = reqwest::Client::new();
    let mut request = client
        .get(&models_url)
        .header("Content-Type", "application/json");

    if let Some(ref key) = api_key {
        request = request.header("Authorization", format!("Bearer {}", key));
    }

    let response = request.send().await.map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        return Err(format!("API returned status {}", response.status()));
    }

    let body: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;

    // OpenAI-compatible response: { "data": [ { "id": "model-name", ... }, ... ] }
    // Ollama response: { "models": [ { "name": "model-name", ... }, ... ] }
    let models: Vec<String> = if let Some(data) = body.get("data").and_then(|d| d.as_array()) {
        data.iter()
            .filter_map(|m| m.get("id").and_then(|id| id.as_str()))
            .map(|s| s.to_string())
            .collect()
    } else if let Some(data) = body.get("models").and_then(|d| d.as_array()) {
        data.iter()
            .filter_map(|m| m.get("name").and_then(|n| n.as_str()))
            .map(|s| s.to_string())
            .collect()
    } else {
        Vec::new()
    };

    Ok(models)
}
