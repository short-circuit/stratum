//! Provider model discovery for the Settings page.

use crate::settings::dto::provider_display;
use crate::state::StratumService;

impl StratumService {
    /// Fetch available models from the provider's API.
    pub async fn fetch_models(&self) -> Result<Vec<String>, String> {
        // Extract config data before async operation (MutexGuard is not Send)
        let (endpoint, api_key, provider) = {
            let state = self.lock()?;
            let config_path = state.vault_path.join(".pkm").join("config.toml");

            let config = if config_path.exists() {
                pkm_core::Config::load(&config_path).map_err(|e| e.to_string())?
            } else {
                return Err("No config found. Save settings first.".into());
            };

            let endpoint = config.ai.effective_endpoint().ok_or_else(|| {
                format!(
                    "No endpoint configured for {} provider — set one in Settings → AI",
                    provider_display(&config.ai.provider)
                )
            })?;
            (endpoint, config.ai.effective_api_key(), config.ai.provider)
        };

        // Validate endpoint URL to prevent SSRF
        pkm_core::endpoint::validate_endpoint_safe(&endpoint).map_err(|e| e.to_string())?;

        // A Custom/CustomOpenAI endpoint that already ends with "/v1" must not get
        // a doubled "/v1/v1/models" URL. Bare-base endpoints (and Ollama) keep the
        // existing "/v1/models" behavior.
        let base = endpoint.trim_end_matches('/');
        let models_url = if base.ends_with("/v1") && provider != pkm_core::AiProvider::Ollama {
            format!("{base}/models")
        } else {
            format!("{base}/v1/models")
        };

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
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            if text.trim().is_empty() {
                return Err(format!("API returned status {status}"));
            }
            let snippet: String = text.chars().take(300).collect();
            return Err(format!("API returned status {status}: {snippet}"));
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
}
