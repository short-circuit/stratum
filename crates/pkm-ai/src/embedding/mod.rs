//! Embedding generation backed by an OpenAI-compatible embeddings endpoint.
//!
//! Stratum generates embeddings by POSTing text to the configured AI
//! endpoint's `POST {endpoint}/v1/embeddings` route (LocalAI-compatible).
//! The request and response follow the OpenAI embeddings API shape:
//!
//! ```json
//! { "model": "<embedding-model>", "input": ["text one", "text two"] }
//! ```
//!
//! The returned vectors are real model embeddings (semantically meaningful),
//! not hashes of the input text. The [`Embedding`] trait is the interface RAG
//! consumes; the concrete implementation is [`OpenAIEmbeddingClient`].
//!
//! Requests are batched (all texts sent in a single call), carry a timeout,
//! are retried on transient failures and 5xx / 429 responses with capped
//! exponential backoff, and surface the endpoint's error body in the returned
//! [`PkmError`] instead of dropping it.

use async_trait::async_trait;
use pkm_core::endpoint::validate_endpoint_safe;
use pkm_core::{AiConfig, PkmError, PkmResult};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

/// Maximum time to wait for a single embeddings request to complete.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

/// Maximum number of attempts per `embed` call (initial attempt + retries).
const MAX_ATTEMPTS: usize = 4;

/// Retryable HTTP status codes (transient server + rate-limit errors).
const RETRYABLE_STATUS: [u16; 4] = [429, 500, 502, 503];

/// Base backoff for retries; doubles on each subsequent retry, capped at
/// [`MAX_BACKOFF`].
const BASE_BACKOFF: Duration = Duration::from_millis(250);

/// Upper bound for per-attempt backoff.
const MAX_BACKOFF: Duration = Duration::from_secs(5);

/// Configuration for the OpenAI-compatible embedding client.
///
/// Built from [`AiConfig`] — embeddings reuse the same endpoint, API key and
/// model selection as the rest of the AI features in the app.
#[derive(Debug, Clone)]
pub struct EmbeddingConfig {
    /// Base URL of the OpenAI-compatible endpoint (no trailing slash), e.g.
    /// `https://api.openai.com/v1` or `http://localhost:11434/v1`.
    pub endpoint: String,
    /// Optional bearer API key. Sent as `Authorization: Bearer <key>` when set.
    pub api_key: Option<String>,
    /// Embedding model name, e.g. `nomic-embed-text` (Ollama) or
    /// `text-embedding-3-small` (OpenAI).
    pub model: String,
    /// Expected dimensionality of the returned vectors. `0` means "infer from
    /// the first response".
    pub dimensions: usize,
}

/// Abstraction over embedding generation sources.
///
/// RAG consumes embeddings exclusively through this trait so that the
/// retrieval path is agnostic to how vectors are produced.
#[async_trait]
pub trait Embedding: Send + Sync {
    /// Generate embedding vectors for a list of texts.
    async fn embed(&self, texts: &[String]) -> PkmResult<Vec<Vec<f32>>>;

    /// Get the dimensionality of the embedding vectors.
    fn dimensions(&self) -> usize;
}

/// Resolve the OpenAI-compatible embedding endpoint from an [`AiConfig`].
///
/// Ollama exposes the OpenAI-compatible API on `{endpoint}/v1`; other
/// providers are expected to already carry the `/v1` prefix in their
/// configured endpoint, so the base URL is used untouched.
fn resolve_endpoint(ai: &AiConfig) -> String {
    let base = ai.endpoint.as_deref().unwrap_or("").trim_end_matches('/');
    if ai.provider == pkm_core::AiProvider::Ollama {
        format!("{base}/v1")
    } else {
        base.to_string()
    }
}

/// Select the embedding model from an [`AiConfig`].
///
/// Prefers a model whose capabilities include `embedding`; falls back to the
/// generic `model` field when none is declared.
fn select_embedding_model(ai: &AiConfig) -> Option<String> {
    ai.models
        .iter()
        .find(|m| m.capabilities.iter().any(|c| c == "embedding"))
        .map(|m| m.name.clone())
        .or_else(|| Some(ai.model.clone()))
}

impl EmbeddingConfig {
    /// Build the embedding configuration from the app's AI settings.
    ///
    /// Returns an error when the configured endpoint is missing or fails
    /// SSRF/URL validation (see [`validate_endpoint_safe`]).
    pub fn from_ai_config(ai: &AiConfig) -> PkmResult<Self> {
        let endpoint = resolve_endpoint(ai);
        if endpoint.is_empty() {
            return Err(PkmError::Ai(
                "No AI endpoint configured — set Settings → AI → API Endpoint to enable embedding generation"
                    .to_string(),
            ));
        }
        validate_endpoint_safe(&endpoint)
            .map_err(|e| PkmError::Ai(format!("Invalid AI endpoint: {e}")))?;

        let model = select_embedding_model(ai).ok_or_else(|| {
            PkmError::Ai(
                "No embedding model configured (no 'embedding' capability assigned)".to_string(),
            )
        })?;

        Ok(Self {
            endpoint,
            api_key: ai.effective_api_key(),
            model,
            dimensions: 0,
        })
    }
}

/// Serialized request body for `POST /v1/embeddings`.
#[derive(Debug, Serialize)]
pub(crate) struct EmbeddingsRequest<'a> {
    pub(crate) model: &'a str,
    pub(crate) input: Vec<&'a str>,
}

/// One embedding entry in the endpoint response.
#[derive(Debug, Deserialize)]
pub(crate) struct EmbeddingData {
    pub(crate) embedding: Vec<f32>,
    pub(crate) index: usize,
}

/// Usage report in the endpoint response.
#[derive(Debug, Deserialize)]
pub(crate) struct EmbeddingUsage {
    pub(crate) prompt_tokens: u32,
    pub(crate) total_tokens: u32,
}

/// Serialized response body for `POST /v1/embeddings`.
#[derive(Debug, Deserialize)]
pub(crate) struct EmbeddingsResponse {
    pub(crate) data: Vec<EmbeddingData>,
    pub(crate) model: String,
    #[allow(dead_code)]
    pub(crate) usage: Option<EmbeddingUsage>,
}

/// OpenAI-compatible error envelope returned by the endpoint on non-2xx.
#[derive(Debug, Deserialize)]
struct ApiErrorBody {
    error: ApiErrorDetail,
}

#[derive(Debug, Deserialize)]
struct ApiErrorDetail {
    message: String,
    #[allow(dead_code)]
    #[serde(rename = "type")]
    kind: Option<String>,
}

/// Internal failure from a single request attempt.
///
/// Carries the HTTP status (when one was received) so the retry loop can
/// decide whether the failure is transient without string-parsing errors.
struct AttemptError {
    message: String,
    status: Option<u16>,
}

impl AttemptError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            status: None,
        }
    }

    fn with_status(status: u16, message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            status: Some(status),
        }
    }
}

/// Real embedding client backed by an OpenAI-compatible endpoint.
///
/// Sends the whole batch to `POST {endpoint}/v1/embeddings` with the
/// configured model and bearer key, applies a timeout and capped retries with
/// exponential backoff, and parses the OpenAI response shape back into
/// per-input vectors.
#[derive(Debug)]
pub struct OpenAIEmbeddingClient {
    endpoint: String,
    api_key: Option<String>,
    model: String,
    dimensions: AtomicUsize,
    client: reqwest::Client,
}

impl Clone for OpenAIEmbeddingClient {
    fn clone(&self) -> Self {
        Self {
            endpoint: self.endpoint.clone(),
            api_key: self.api_key.clone(),
            model: self.model.clone(),
            dimensions: AtomicUsize::new(self.dimensions.load(Ordering::Relaxed)),
            client: self.client.clone(),
        }
    }
}

impl OpenAIEmbeddingClient {
    /// Create a client from an explicit configuration.
    ///
    /// The endpoint is validated (SSRF guard) before the client is returned.
    pub fn new(config: EmbeddingConfig) -> PkmResult<Self> {
        if config.endpoint.is_empty() {
            return Err(PkmError::Ai(
                "Empty endpoint for OpenAIEmbeddingClient".to_string(),
            ));
        }
        validate_endpoint_safe(&config.endpoint)
            .map_err(|e| PkmError::Ai(format!("Invalid AI endpoint: {e}")))?;
        if config.model.is_empty() {
            return Err(PkmError::Ai(
                "Empty embedding model for OpenAIEmbeddingClient".to_string(),
            ));
        }

        let client = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .expect("reqwest client build should not fail");
        Ok(Self {
            endpoint: config.endpoint.trim_end_matches('/').to_string(),
            api_key: config.api_key,
            model: config.model,
            dimensions: AtomicUsize::new(config.dimensions),
            client,
        })
    }

    /// Build the client from the app's AI settings.
    ///
    /// Convenience wrapper around [`EmbeddingConfig::from_ai_config`] +
    /// [`OpenAIEmbeddingClient::new`].
    pub fn from_ai_config(ai: &AiConfig) -> PkmResult<Self> {
        Self::new(EmbeddingConfig::from_ai_config(ai)?)
    }

    fn url(&self) -> String {
        format!("{}/embeddings", self.endpoint)
    }

    /// Extract a readable message from a non-2xx response, preferring the
    /// OpenAI `{ "error": { "message": ... } }` envelope and falling back to
    /// the raw text body.
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

    /// Send one embeddings request and parse the response into a vector
    /// indexed by the original input order.
    async fn attempt(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, AttemptError> {
        let url = self.url();
        let req = EmbeddingsRequest {
            model: &self.model,
            input: texts.iter().map(String::as_str).collect(),
        };

        let mut request = self.client.post(&url).json(&req);
        if let Some(ref key) = self.api_key {
            request = request.bearer_auth(key);
        }

        let resp = request
            .send()
            .await
            .map_err(|e| AttemptError::new(format!("Embedding request to {url} failed: {e}")))?;

        let status = resp.status();
        if !status.is_success() {
            let msg = Self::error_message(resp).await;
            return Err(AttemptError::with_status(
                status.as_u16(),
                format!("Embedding endpoint returned {status}: {msg}"),
            ));
        }

        let body: EmbeddingsResponse = resp.json().await.map_err(|e| {
            AttemptError::new(format!(
                "Failed to parse embedding response from {url}: {e}"
            ))
        })?;

        let mut results = Vec::new();
        for data in body.data {
            if data.index >= results.len() {
                results.resize(data.index + 1, Vec::new());
            }
            results[data.index] = data.embedding;
        }

        if results.len() != texts.len() {
            return Err(AttemptError::new(format!(
                "Embedding endpoint returned {} vectors for {} inputs",
                results.len(),
                texts.len()
            )));
        }

        // Infer and pin the dimensionality from the first returned vector when
        // it was not configured up front.
        let mut dims = self.dimensions.load(Ordering::Relaxed);
        if dims == 0 {
            if let Some(first) = results.first() {
                dims = first.len();
            }
        }
        self.dimensions.store(dims, Ordering::Relaxed);

        tracing::debug!(
            "Generated {} embeddings using model '{}' ({}) tokens: {}",
            results.len(),
            body.model,
            url,
            body.usage
                .as_ref()
                .map(|u| format!("{} ({} prompt)", u.total_tokens, u.prompt_tokens))
                .unwrap_or_else(|| "n/a".to_string())
        );

        Ok(results)
    }
}

#[async_trait]
impl Embedding for OpenAIEmbeddingClient {
    async fn embed(&self, texts: &[String]) -> PkmResult<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let mut attempt = 0usize;
        loop {
            attempt += 1;
            match self.attempt(texts).await {
                Ok(vectors) => return Ok(vectors),
                Err(err) => {
                    let transient = match err.status {
                        Some(code) => RETRYABLE_STATUS.contains(&code),
                        None => true, // transport/connect/parse error → retry
                    };
                    if attempt < MAX_ATTEMPTS && transient {
                        let base = BASE_BACKOFF.as_millis() as u64;
                        let exp = base.saturating_mul(1 << (attempt - 1));
                        let delay_ms = exp.min(MAX_BACKOFF.as_millis() as u64);
                        tracing::warn!(
                            "Embedding attempt {attempt} failed (will retry in {}ms): {}",
                            delay_ms,
                            err.message
                        );
                        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                    } else {
                        return Err(PkmError::Ai(err.message));
                    }
                }
            }
        }
    }

    fn dimensions(&self) -> usize {
        self.dimensions.load(Ordering::Relaxed)
    }
}

/// Compute the cosine similarity between two vectors.
///
/// Returns a value in the range [-1.0, 1.0], where 1.0 means identical
/// direction, 0.0 means orthogonal, and -1.0 means opposite direction.
///
/// If either vector is zero-length, returns 0.0.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() || b.is_empty() {
        return 0.0;
    }

    let mut dot_product = 0.0;
    let mut mag_a = 0.0;
    let mut mag_b = 0.0;

    for (x, y) in a.iter().zip(b.iter()) {
        dot_product += x * y;
        mag_a += x * x;
        mag_b += y * y;
    }

    let magnitude = mag_a.sqrt() * mag_b.sqrt();
    if magnitude == 0.0 {
        0.0
    } else {
        (dot_product / magnitude).clamp(-1.0, 1.0)
    }
}

/// Compute cosine similarity matrix between two sets of vectors.
///
/// Returns a matrix of shape (a_rows x b_rows).
pub fn cosine_similarity_matrix(a: &[Vec<f32>], b: &[Vec<f32>]) -> Vec<Vec<f32>> {
    let mut matrix = Vec::with_capacity(a.len());
    for a_vec in a {
        let mut row = Vec::with_capacity(b.len());
        for b_vec in b {
            row.push(cosine_similarity(a_vec, b_vec));
        }
        matrix.push(row);
    }
    matrix
}

#[cfg(test)]
mod tests;
