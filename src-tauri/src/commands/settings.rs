//! Settings and AI configuration commands.

use crate::commands::vault::AppState;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SettingsDto {
    pub vault_path: String,
    pub theme: ThemeSettingsDto,
    pub ai: AiSettingsDto,
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

#[tauri::command]
pub async fn get_settings(
    state: tauri::State<'_, AppState>,
) -> Result<SettingsDto, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let config_path = state.vault_path.join(".pkm").join("config.toml");

    let config = if config_path.exists() {
        pkm_core::Config::load(&config_path).map_err(|e| e.to_string())?
    } else {
        let mut c = pkm_core::Config::default();
        c.vault_path = state.vault_path.clone();
        c
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
            },
            endpoint: config.ai.endpoint,
            api_key: config.ai.api_key,
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
    })
}

#[tauri::command]
pub async fn save_settings(
    settings: SettingsDto,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let config_path = state.vault_path.join(".pkm").join("config.toml");

    let provider = match settings.ai.provider.as_str() {
        "openai" => pkm_core::AiProvider::OpenAI,
        "anthropic" => pkm_core::AiProvider::Anthropic,
        "google" => pkm_core::AiProvider::Google,
        "zai" => pkm_core::AiProvider::Zai,
        "custom" => pkm_core::AiProvider::Custom,
        _ => pkm_core::AiProvider::Ollama,
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
            api_key: settings.ai.api_key,
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
        ..pkm_core::Config::default()
    };

    config.save(&config_path).map_err(|e| e.to_string())?;
    Ok(())
}

/// Fetch available models from the provider's API.
#[tauri::command]
pub async fn fetch_models(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<String>, String> {
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
            config.ai.endpoint.unwrap_or_else(|| "http://localhost:11434".into()),
            config.ai.api_key.clone(),
        )
    };

    let models_url = format!("{}/v1/models", endpoint.trim_end_matches('/'));

    let client = reqwest::Client::new();
    let mut request = client.get(&models_url).header("Content-Type", "application/json");

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
