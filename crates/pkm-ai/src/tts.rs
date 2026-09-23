//! Text-to-speech synthesis backed by an OpenAI-compatible audio endpoint.
//!
//! Stratum synthesizes speech by POSTing a JSON body to the configured AI
//! endpoint's `POST {endpoint}/v1/audio/speech` route (LocalAI/OpenAI
//! compatible) and reading back the raw audio bytes:
//!
//! ```json
//! {
//!   "model": "<tts-model>",
//!   "input": "text to speak",
//!   "voice": "alloy",
//!   "response_format": "mp3",
//!   "speed": 1.0
//! }
//! ```
//!
//! The endpoint and credentials are resolved from the app's AI settings,
//! mirroring the pattern used by [`crate::embedding::OpenAIEmbeddingClient`]:
//! the AI base URL is shared, an optional per-feature override endpoint is
//! respected, and an optional bearer key is applied.

use pkm_core::endpoint::validate_endpoint_safe;
use pkm_core::{AiConfig, PkmError, PkmResult, TtsConfig};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Maximum time to wait for a single synthesis request to complete.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

/// Maximum number of attempts per `synthesize` call (initial + retries).
const MAX_ATTEMPTS: usize = 3;

/// Retryable HTTP status codes (transient server + rate-limit errors).
const RETRYABLE_STATUS: [u16; 4] = [429, 500, 502, 503];

/// Base backoff for retries; doubles per retry, capped at [`MAX_BACKOFF`].
const BASE_BACKOFF: Duration = Duration::from_millis(250);

/// Upper bound for per-attempt backoff.
const MAX_BACKOFF: Duration = Duration::from_secs(4);

/// Resolve the base endpoint for OpenAI-compatible audio routes.
fn resolve_endpoint(ai: &AiConfig) -> String {
    let base = ai.endpoint.as_deref().unwrap_or("").trim_end_matches('/');
    if ai.provider == pkm_core::AiProvider::Ollama {
        format!("{base}/v1")
    } else {
        base.to_string()
    }
}

/// Select the TTS model from an [`AiConfig`].
///
/// Prefers a model whose capabilities include `tts`; falls back to the
/// generic `model` field when none is declared.
fn select_tts_model(ai: &AiConfig) -> Option<String> {
    ai.models
        .iter()
        .find(|m| m.capabilities.iter().any(|c| c == "tts"))
        .map(|m| m.name.clone())
        .or_else(|| Some(ai.model.clone()))
}

/// Configuration for the OpenAI-compatible TTS client.
#[derive(Debug, Clone)]
pub struct TtsConfigResolved {
    /// Base URL of the OpenAI-compatible endpoint (no trailing slash).
    pub endpoint: String,
    /// Optional bearer API key. Sent as `Authorization: Bearer ***` when set.
    pub api_key: Option<String>,
    /// TTS model name (e.g. `tts-1`, `kokoro`, `voice-en-us-amy`).
    pub model: String,
    /// Voice name.
    pub voice: String,
    /// Output audio format.
    pub format: String,
    /// Playback speed multiplier.
    pub speed: f32,
}

impl TtsConfigResolved {
    /// Build the TTS configuration from the app's settings.
    ///
    /// Decision (E7.10-5): TTS is wired to honor `TtsConfig::use_llm_gateway_and_auth`
    /// rather than being gated. TTS has live consumers (read-aloud, test button,
    /// `tts_synthesize`/`tts_speak` commands), so the checkbox is active, not
    /// hidden. When the flag is set, the main LLM gateway endpoint/auth from
    /// [`AiConfig`] is used for synthesis exclusively — any TTS-specific
    /// endpoint/api_key override is ignored. When the flag is unset (the default,
    /// or absent from an older config file), the existing behavior is preserved
    /// exactly: the TTS endpoint is used when set, otherwise the AI endpoint,
    /// and the TTS api_key is used when set, otherwise the AI key.
    ///
    /// Returns an error when the resolved endpoint is missing or fails
    /// SSRF/URL validation (see [`validate_endpoint_safe`]).
    pub fn from_config(ai: &AiConfig, tts: &TtsConfig) -> PkmResult<Self> {
        let use_gateway = tts.use_llm_gateway_and_auth;
        let endpoint = if use_gateway || tts.endpoint.trim().is_empty() {
            resolve_endpoint(ai)
        } else {
            tts.endpoint.trim_end_matches('/').to_string()
        };
        if endpoint.is_empty() {
            return Err(PkmError::Ai(
                "No AI endpoint configured to synthesize speech — set the AI API endpoint or a TTS endpoint in Settings → AI"
                    .to_string(),
            ));
        }
        validate_endpoint_safe(&endpoint)
            .map_err(|e| PkmError::Ai(format!("Invalid AI endpoint: {e}")))?;

        let model = select_tts_model(ai).ok_or_else(|| {
            PkmError::Ai("No TTS model configured (no 'tts' capability assigned)".to_string())
        })?;

        let voice = if tts.voice.trim().is_empty() {
            "alloy".to_string()
        } else {
            tts.voice.clone()
        };

        let format = if tts.format.trim().is_empty() {
            "mp3".to_string()
        } else {
            tts.format.clone()
        };

        let speed = if !(0.1..=4.0).contains(&tts.speed) {
            1.0
        } else {
            tts.speed
        };

        let api_key = if use_gateway {
            ai.effective_api_key()
        } else if tts.api_key.as_deref().is_some_and(|k| !k.trim().is_empty()) {
            tts.api_key.clone()
        } else {
            ai.effective_api_key()
        };

        Ok(Self {
            endpoint: endpoint.trim_end_matches('/').to_string(),
            api_key,
            model,
            voice,
            format,
            speed,
        })
    }
}

/// Serialized request body for `POST /v1/audio/speech`.
#[derive(Debug, Serialize)]
struct SpeechRequest<'a> {
    model: &'a str,
    input: &'a str,
    voice: &'a str,
    #[serde(rename = "response_format")]
    response_format: &'a str,
    speed: f32,
}

/// Outcome of a single synthesis attempt (used internally for retry logic).
struct AttemptError {
    message: String,
    transient: bool,
}

/// Client for the OpenAI-compatible TTS endpoint.
///
/// Synthesizes text into audio bytes. Requests carry a timeout and are
/// retried on transient transport failures and 5xx / 429 responses with
/// capped exponential backoff.
#[derive(Debug, Clone)]
pub struct TtsClient {
    config: TtsConfigResolved,
    client: reqwest::Client,
}

impl TtsClient {
    /// Create a client from a resolved configuration.
    pub fn new(config: TtsConfigResolved) -> PkmResult<Self> {
        if config.endpoint.is_empty() {
            return Err(PkmError::Ai("Empty endpoint for TtsClient".to_string()));
        }
        validate_endpoint_safe(&config.endpoint)
            .map_err(|e| PkmError::Ai(format!("Invalid AI endpoint: {e}")))?;
        if config.model.is_empty() {
            return Err(PkmError::Ai("Empty TTS model for TtsClient".to_string()));
        }
        let client = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(|e| PkmError::Ai(format!("Failed to build HTTP client: {e}")))?;
        Ok(Self { config, client })
    }

    /// Build the client from the app's settings (convenience wrapper).
    pub fn from_config(ai: &AiConfig, tts: &TtsConfig) -> PkmResult<Self> {
        Self::new(TtsConfigResolved::from_config(ai, tts)?)
    }

    /// The model name in use (exposed for callers/DTOs).
    pub fn model(&self) -> &str {
        &self.config.model
    }

    /// The resolved endpoint (exposed for callers/DTOs).
    pub fn endpoint(&self) -> &str {
        &self.config.endpoint
    }

    fn url(&self) -> String {
        format!("{}/audio/speech", self.config.endpoint)
    }

    /// Build an error message from a non-2xx response, preferring the OpenAI
    /// `{ "error": { "message": ... } }` envelope.
    async fn error_message(resp: reqwest::Response) -> String {
        let status = resp.status();
        let text = match resp.text().await {
            Ok(t) if !t.is_empty() => t,
            _ => return format!("HTTP {status}"),
        };
        if let Ok(body) = serde_json::from_str::<ApiErrorBody>(&text) {
            return format!("HTTP {status}: {}", body.error.message);
        }
        let snippet: String = text.chars().take(300).collect();
        format!("HTTP {status}: {snippet}")
    }

    /// Perform a single synthesis attempt.
    async fn attempt(&self, text: &str) -> Result<Vec<u8>, AttemptError> {
        let url = self.url();
        let req = SpeechRequest {
            model: &self.config.model,
            input: text,
            voice: &self.config.voice,
            response_format: &self.config.format,
            speed: self.config.speed,
        };

        let mut request = self.client.post(&url).json(&req);
        if let Some(ref key) = self.config.api_key {
            request = request.bearer_auth(key);
        }

        let resp = request.send().await.map_err(|e| AttemptError {
            message: format!("Speech request to {url} failed: {e}"),
            transient: true,
        })?;

        let status = resp.status();
        if !status.is_success() {
            let msg = Self::error_message(resp).await;
            return Err(AttemptError {
                transient: RETRYABLE_STATUS.contains(&status.as_u16()),
                message: format!("Speech endpoint returned {status}: {msg}"),
            });
        }

        let bytes = resp
            .bytes()
            .await
            .map_err(|e| AttemptError {
                message: format!("Failed to read audio response from {url}: {e}"),
                transient: false,
            })?
            .to_vec();

        if bytes.is_empty() {
            return Err(AttemptError {
                message: format!("Speech endpoint returned an empty audio body from {url}"),
                transient: false,
            });
        }

        tracing::debug!(
            "Synthesized {} bytes of {} audio using model '{}' (voice '{}') from {}",
            bytes.len(),
            self.config.format,
            self.config.model,
            self.config.voice,
            url
        );

        Ok(bytes)
    }

    /// Synthesize `text` and return the raw audio bytes.
    pub async fn synthesize(&self, text: &str) -> PkmResult<Vec<u8>> {
        if text.trim().is_empty() {
            return Err(PkmError::Ai("Text to synthesize is empty".to_string()));
        }

        let mut attempt_num = 0usize;
        loop {
            attempt_num += 1;
            match self.attempt(text).await {
                Ok(bytes) => return Ok(bytes),
                Err(err) => {
                    if attempt_num < MAX_ATTEMPTS && err.transient {
                        let base = BASE_BACKOFF.as_millis() as u64;
                        let exp = base.saturating_mul(1 << (attempt_num - 1));
                        let delay = exp.min(MAX_BACKOFF.as_millis() as u64);
                        tracing::warn!(
                            "Speech attempt {attempt_num} failed (will retry in {delay}ms): {}",
                            err.message
                        );
                        tokio::time::sleep(Duration::from_millis(delay)).await;
                    } else {
                        return Err(PkmError::Ai(err.message));
                    }
                }
            }
        }
    }
}

/// OpenAI-compatible error envelope returned by the endpoint on non-2xx.
#[derive(Debug, Deserialize)]
struct ApiErrorBody {
    error: ApiErrorDetail,
}

#[derive(Debug, Deserialize)]
struct ApiErrorDetail {
    message: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use pkm_core::AiProvider;
    use wiremock::matchers::{body_partial_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn ai_config(endpoint: &str) -> AiConfig {
        AiConfig {
            provider: AiProvider::CustomOpenAI,
            endpoint: Some(endpoint.to_string()),
            api_key: Some("sk-test".to_string()),
            model: "tts-1".to_string(),
            models: vec![pkm_core::AiModelConfig {
                name: "tts-1".to_string(),
                capabilities: vec!["tts".to_string()],
            }],
            ..Default::default()
        }
    }

    fn tts_config() -> TtsConfig {
        TtsConfig::default()
    }

    #[test]
    fn test_resolved_config_uses_ai_endpoint_when_tts_empty() {
        let cfg =
            TtsConfigResolved::from_config(&ai_config("http://localhost:18080/v1"), &tts_config())
                .expect("should resolve");
        assert_eq!(cfg.endpoint, "http://localhost:18080/v1");
        assert_eq!(cfg.model, "tts-1");
        assert_eq!(cfg.voice, "alloy");
        assert_eq!(cfg.format, "mp3");
        assert_eq!(cfg.speed, 1.0);
        assert_eq!(cfg.api_key.as_deref(), Some("sk-test"));
    }

    #[test]
    fn test_resolved_config_prefers_tts_endpoint_override() {
        let tts = TtsConfig {
            endpoint: "https://tts.example.com/v1".to_string(),
            api_key: Some("tts-key".to_string()),
            voice: "onyx".to_string(),
            format: "flac".to_string(),
            speed: 0.8,
            ..TtsConfig::default()
        };
        let cfg = TtsConfigResolved::from_config(&ai_config("http://localhost:18080/v1"), &tts)
            .expect("should resolve");
        assert_eq!(cfg.endpoint, "https://tts.example.com/v1");
        assert_eq!(cfg.api_key.as_deref(), Some("tts-key"));
        assert_eq!(cfg.voice, "onyx");
        assert_eq!(cfg.format, "flac");
        assert!((cfg.speed - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn test_resolved_config_rejects_speed_out_of_range() {
        let cfg = TtsConfigResolved::from_config(
            &ai_config("http://localhost:18080/v1"),
            &TtsConfig {
                speed: 99.0,
                ..TtsConfig::default()
            },
        )
        .expect("should normalize speed");
        assert_eq!(cfg.speed, 1.0);
    }

    #[test]
    fn test_resolved_config_missing_endpoint_is_error() {
        let mut ai = ai_config("http://localhost:18080/v1");
        ai.endpoint = None;
        let err = TtsConfigResolved::from_config(&ai, &tts_config())
            .expect_err("should fail without endpoint");
        assert!(matches!(err, PkmError::Ai(_)));
    }

    #[test]
    fn test_resolved_config_reuses_llm_gateway_and_auth_when_flag_true() {
        // Flag set → the AI gateway endpoint and AI api_key are used even when
        // a TTS-specific endpoint/api_key would otherwise override them.
        let tts = TtsConfig {
            endpoint: "https://tts.example.com/v1".to_string(),
            api_key: Some("tts-key".to_string()),
            use_llm_gateway_and_auth: true,
            ..TtsConfig::default()
        };
        let ai = AiConfig {
            provider: AiProvider::CustomOpenAI,
            endpoint: Some("https://gateway.example.com/v1".to_string()),
            api_key: Some("llm-secret".to_string()),
            model: "tts-1".to_string(),
            models: vec![pkm_core::AiModelConfig {
                name: "tts-1".to_string(),
                capabilities: vec!["tts".to_string()],
            }],
            ..Default::default()
        };
        let cfg = TtsConfigResolved::from_config(&ai, &tts).expect("should resolve");
        assert_eq!(cfg.endpoint, "https://gateway.example.com/v1");
        assert_eq!(cfg.api_key.as_deref(), Some("llm-secret"));
    }

    #[test]
    fn test_resolved_config_flag_true_without_ai_endpoint_is_an_error() {
        // Flag set + no AI endpoint configured → hard error, even though a TTS
        // endpoint is set (the gateway is the only allowed source when enabled).
        let tts = TtsConfig {
            endpoint: "https://tts.example.com/v1".to_string(),
            use_llm_gateway_and_auth: true,
            ..TtsConfig::default()
        };
        let mut ai = ai_config("http://localhost:18080/v1");
        ai.endpoint = None;
        let err = TtsConfigResolved::from_config(&ai, &tts)
            .expect_err("gateway mode with no AI endpoint must fail");
        assert!(matches!(err, PkmError::Ai(_)));
    }

    #[test]
    fn test_resolved_config_flag_false_preserves_tts_override() {
        // Default (flag false) → TTS-specific endpoint/api_key win as before.
        let tts = TtsConfig {
            endpoint: "https://tts.example.com/v1".to_string(),
            api_key: Some("tts-key".to_string()),
            use_llm_gateway_and_auth: false,
            ..TtsConfig::default()
        };
        let cfg = TtsConfigResolved::from_config(&ai_config("http://localhost:18080/v1"), &tts)
            .expect("should resolve");
        assert_eq!(cfg.endpoint, "https://tts.example.com/v1");
        assert_eq!(cfg.api_key.as_deref(), Some("tts-key"));
    }

    #[test]
    fn test_resolved_config_flag_false_without_tts_uses_ai_gateway() {
        // Default (flag false), no TTS override → falls back to AI endpoint/key
        // (unchanged pre-flag behavior).
        let tts = TtsConfig {
            use_llm_gateway_and_auth: false,
            ..TtsConfig::default()
        };
        let cfg = TtsConfigResolved::from_config(&ai_config("http://localhost:18080/v1"), &tts)
            .expect("should resolve");
        assert_eq!(cfg.endpoint, "http://localhost:18080/v1");
        assert_eq!(cfg.api_key.as_deref(), Some("sk-test"));
    }

    #[tokio::test]
    async fn test_synthesize_happy_path() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/audio/speech"))
            .and(header("authorization", "Bearer sk-test"))
            .and(body_partial_json(serde_json::json!({
                "model": "tts-1",
                "input": "Hello world",
                "voice": "alloy",
                "response_format": "mp3",
                "speed": 1.0,
            })))
            .respond_with(
                ResponseTemplate::new(200).set_body_bytes(b"ID3\x00\x00fake-mp3-bytes".to_vec()),
            )
            .expect(1..)
            .mount(&server)
            .await;

        let client =
            TtsClient::from_config(&ai_config(&format!("{}/v1", server.uri())), &tts_config())
                .expect("client builds");
        let bytes = client.synthesize("Hello world").await.expect("synthesizes");
        assert_eq!(bytes, b"ID3\x00\x00fake-mp3-bytes");
    }

    #[tokio::test]
    async fn test_synthesize_empty_text_is_error() {
        let server = MockServer::start().await;
        let client = TtsClient::from_config(&ai_config(&server.uri()), &tts_config())
            .expect("client builds");
        let err = client.synthesize("   ").await.expect_err("should fail");
        assert!(matches!(err, PkmError::Ai(_)));
    }

    #[tokio::test]
    async fn test_synthesize_http_error_surfaces_envelope() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/audio/speech"))
            .respond_with(
                ResponseTemplate::new(400).set_body_json(serde_json::json!({
                    "error": { "message": "unknown voice: not-a-voice", "type": "invalid_request_error" }
                })),
            )
            .mount(&server)
            .await;

        let client =
            TtsClient::from_config(&ai_config(&format!("{}/v1", server.uri())), &tts_config())
                .expect("client builds");
        let err = client.synthesize("hi").await.expect_err("should fail");
        let msg = err.to_string();
        assert!(msg.contains("unknown voice"), "got: {msg}");
        assert!(msg.contains("400"), "got: {msg}");
    }

    #[tokio::test]
    async fn test_synthesize_retries_on_503_then_succeeds() {
        let server = MockServer::start().await;
        // First two attempts fail with 503, third succeeds.
        Mock::given(method("POST"))
            .and(path("/v1/audio/speech"))
            .respond_with(ResponseTemplate::new(503))
            .up_to_n_times(2)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/audio/speech"))
            .respond_with(
                ResponseTemplate::new(200).set_body_bytes(b"retry-success-audio".to_vec()),
            )
            .mount(&server)
            .await;

        let client =
            TtsClient::from_config(&ai_config(&format!("{}/v1", server.uri())), &tts_config())
                .expect("client builds");
        let bytes = client
            .synthesize("retry me")
            .await
            .expect("eventually succeeds");
        assert_eq!(bytes, b"retry-success-audio");
    }

    #[tokio::test]
    async fn test_synthesize_empty_body_is_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/audio/speech"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        let client =
            TtsClient::from_config(&ai_config(&format!("{}/v1", server.uri())), &tts_config())
                .expect("client builds");
        let err = client.synthesize("empty").await.expect_err("should fail");
        assert!(matches!(err, PkmError::Ai(_)));
    }
}
