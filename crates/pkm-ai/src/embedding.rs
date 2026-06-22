use async_trait::async_trait;
use pkm_core::{PkmError, PkmResult};
use serde::{Deserialize, Serialize};

/// Configuration for an embedding model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    /// Path to the local embedding model (ONNX, GGUF, etc.).
    pub model_path: Option<String>,
    /// Dimension of the embedding vectors.
    pub dimensions: usize,
    /// Provider type: "local" or "remote".
    pub provider: EmbeddingProvider,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            model_path: None,
            dimensions: 384,
            provider: EmbeddingProvider::Local,
        }
    }
}

/// Embedding provider selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmbeddingProvider {
    Local,
    Remote,
}

/// Abstraction over different embedding generation sources.
#[async_trait]
pub trait Embedding: Send + Sync {
    /// Generate embedding vectors for a list of texts.
    async fn embed(&self, texts: &[String]) -> PkmResult<Vec<Vec<f32>>>;

    /// Get the dimensionality of the embedding vectors.
    fn dimensions(&self) -> usize;
}

// ---------------------------------------------------------------------------
// LocalEmbedding (stub)
// ---------------------------------------------------------------------------

/// Local embedding provider that returns deterministic-ish vectors.
///
/// This is a stub implementation for environments without a real ONNX/llama.cpp
/// runtime. It generates pseudo-random vectors based on a hash of the input
/// text, producing consistent vectors for the same input across calls.
#[derive(Debug, Clone)]
pub struct LocalEmbedding {
    dimensions: usize,
    _model_path: Option<String>,
}

impl LocalEmbedding {
    pub fn new(config: &EmbeddingConfig) -> Self {
        tracing::info!(
            "Initializing LocalEmbedding with dimensions={}, model_path={:?}",
            config.dimensions,
            config.model_path
        );
        Self {
            dimensions: config.dimensions,
            _model_path: config.model_path.clone(),
        }
    }

    /// Generate a deterministic pseudo-random vector from a hash of the input text.
    fn hash_to_vector(text: &str, dimensions: usize) -> Vec<f32> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        let seed = hasher.finish();

        // Use a simple seedable RNG to produce deterministic vectors
        let mut vec = Vec::with_capacity(dimensions);
        let mut state = seed;
        for _ in 0..dimensions {
            // Simple xorshift64
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            // Map to [-1.0, 1.0]
            let val = (state as f64 / u64::MAX as f64) * 2.0 - 1.0;
            vec.push(val as f32);
        }

        // Normalize the vector to unit length for cosine similarity
        let magnitude: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        if magnitude > 0.0 {
            for x in &mut vec {
                *x /= magnitude;
            }
        }

        vec
    }
}

#[async_trait]
impl Embedding for LocalEmbedding {
    async fn embed(&self, texts: &[String]) -> PkmResult<Vec<Vec<f32>>> {
        tracing::debug!(
            "LocalEmbedding: generating {} embeddings (d={})",
            texts.len(),
            self.dimensions
        );

        let vectors: Vec<Vec<f32>> = texts
            .iter()
            .map(|text| Self::hash_to_vector(text, self.dimensions))
            .collect();

        tracing::trace!(
            "LocalEmbedding: generated {} embedding vectors",
            vectors.len()
        );

        Ok(vectors)
    }

    fn dimensions(&self) -> usize {
        self.dimensions
    }
}

// ---------------------------------------------------------------------------
// RemoteEmbedding (OpenAI-compatible)
// ---------------------------------------------------------------------------

/// Remote embedding provider that calls an external API.
///
/// Uses the OpenAI-compatible embeddings API format.
#[derive(Debug, Clone)]
pub struct RemoteEmbedding {
    endpoint: String,
    api_key: Option<String>,
    model: String,
    dimensions: usize,
    client: reqwest::Client,
}

impl RemoteEmbedding {
    pub fn new(
        endpoint: impl Into<String>,
        api_key: Option<String>,
        model: impl Into<String>,
        dimensions: usize,
    ) -> Self {
        Self {
            endpoint: endpoint.into(),
            api_key,
            model: model.into(),
            dimensions,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl Embedding for RemoteEmbedding {
    async fn embed(&self, texts: &[String]) -> PkmResult<Vec<Vec<f32>>> {
        let url = format!(
            "{}/embeddings",
            self.endpoint.trim_end_matches('/')
        );

        #[derive(Serialize)]
        struct RemoteEmbeddingRequest<'a> {
            model: &'a str,
            input: &'a [String],
        }

        let req = RemoteEmbeddingRequest {
            model: &self.model,
            input: texts,
        };

        let mut request = self.client.post(&url).json(&req);
        if let Some(ref key) = self.api_key {
            request = request.bearer_auth(key);
        }

        let resp = request
            .send()
            .await
            .map_err(|e| PkmError::Ai(format!("Remote embedding request failed: {e}")))?;

        #[derive(Deserialize)]
        struct RemoteEmbeddingResponse {
            data: Vec<EmbeddingData>,
            model: String,
            _usage: Option<EmbeddingUsage>,
        }

        #[derive(Deserialize)]
        struct EmbeddingData {
            embedding: Vec<f32>,
            index: usize,
        }

        #[derive(Deserialize)]
        struct EmbeddingUsage {
            _prompt_tokens: u32,
            _total_tokens: u32,
        }

        let body: RemoteEmbeddingResponse = resp
            .json()
            .await
            .map_err(|e| PkmError::Ai(format!("Remote embedding parse error: {e}")))?;

        let mut results = vec![vec![0.0f32; self.dimensions]; texts.len()];
        for data in body.data {
            if data.index < results.len() {
                results[data.index] = data.embedding;
            }
        }

        tracing::debug!(
            "RemoteEmbedding: generated {} embeddings using model '{}'",
            results.len(),
            body.model
        );

        Ok(results)
    }

    fn dimensions(&self) -> usize {
        self.dimensions
    }
}

// ---------------------------------------------------------------------------
// Utility functions
// ---------------------------------------------------------------------------

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
mod tests {
    use super::*;

    // ---- Cosine similarity tests ----

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!(
            (sim - 1.0).abs() < 1e-6,
            "Identical vectors should have similarity 1.0, got {sim}"
        );
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!(
            sim.abs() < 1e-6,
            "Orthogonal vectors should have similarity 0.0, got {sim}"
        );
    }

    #[test]
    fn test_cosine_similarity_opposite() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![-1.0, 0.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!(
            (sim - (-1.0)).abs() < 1e-6,
            "Opposite vectors should have similarity -1.0, got {sim}"
        );
    }

    #[test]
    fn test_cosine_similarity_zero_vector() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 0.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!(
            sim.abs() < 1e-6,
            "Zero vector should give similarity 0.0, got {sim}"
        );
    }

    #[test]
    fn test_cosine_similarity_both_zero() {
        let a = vec![0.0, 0.0, 0.0];
        let b = vec![0.0, 0.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!(
            sim.abs() < 1e-6,
            "Both zero vectors should give similarity 0.0, got {sim}"
        );
    }

    #[test]
    fn test_cosine_similarity_different_lengths() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!(
            sim.abs() < 1e-6,
            "Different length vectors should give similarity 0.0, got {sim}"
        );
    }

    #[test]
    fn test_cosine_similarity_empty_vectors() {
        let a: Vec<f32> = vec![];
        let b: Vec<f32> = vec![];
        let sim = cosine_similarity(&a, &b);
        assert!(
            sim.abs() < 1e-6,
            "Empty vectors should give similarity 0.0, got {sim}"
        );
    }

    #[test]
    fn test_cosine_similarity_partial_overlap() {
        let a = vec![1.0, 0.5, 0.0];
        let b = vec![0.5, 1.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        // Both have magnitude sqrt(1.25), dot = 1.0
        // similarity = 1.0 / 1.25 = 0.8
        let expected = 0.8;
        assert!(
            (sim - expected).abs() < 1e-6,
            "Expected similarity {expected}, got {sim}"
        );
    }

    #[test]
    fn test_cosine_similarity_negative_component() {
        let a = vec![1.0, 0.0];
        let b = vec![0.5, -0.5];
        let sim = cosine_similarity(&a, &b);
        let expected = (0.5) / (1.0 * (0.5_f32.powi(2) + (-0.5_f32).powi(2)).sqrt());
        assert!(
            (sim - expected).abs() < 1e-6,
            "Expected similarity {expected}, got {sim}"
        );
    }

    #[test]
    fn test_cosine_similarity_matrix() {
        let a = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
        let b = vec![vec![1.0, 0.0], vec![0.5, 0.5]];
        let matrix = cosine_similarity_matrix(&a, &b);
        assert_eq!(matrix.len(), 2);
        assert_eq!(matrix[0].len(), 2);
        // a[0]=[1,0] vs b[0]=[1,0] -> 1.0
        assert!((matrix[0][0] - 1.0).abs() < 1e-6);
        // a[0]=[1,0] vs b[1]=[0.5,0.5] -> 0.5/sqrt(0.5) ~ 0.707
        assert!((matrix[0][1] - 0.70710677).abs() < 1e-4);
        // a[1]=[0,1] vs b[0]=[1,0] -> 0.0
        assert!(matrix[1][0].abs() < 1e-6);
        // a[1]=[0,1] vs b[1]=[0.5,0.5] -> 0.5/sqrt(0.5) ~ 0.707
        assert!((matrix[1][1] - 0.70710677).abs() < 1e-4);
    }

    // ---- LocalEmbedding tests ----

    #[tokio::test]
    async fn test_local_embedding_deterministic() {
        let config = EmbeddingConfig {
            dimensions: 8,
            provider: EmbeddingProvider::Local,
            ..Default::default()
        };
        let embedding = LocalEmbedding::new(&config);

        let texts = vec!["hello world".to_string(), "hello world".to_string()];
        let vectors = embedding.embed(&texts).await.unwrap();
        assert_eq!(vectors.len(), 2);
        // Same input should produce same output
        assert_eq!(vectors[0], vectors[1]);
    }

    #[tokio::test]
    async fn test_local_embedding_dimensions() {
        let config = EmbeddingConfig {
            dimensions: 64,
            provider: EmbeddingProvider::Local,
            ..Default::default()
        };
        let embedding = LocalEmbedding::new(&config);

        let texts = vec!["test".to_string()];
        let vectors = embedding.embed(&texts).await.unwrap();
        assert_eq!(vectors[0].len(), 64);
        assert_eq!(embedding.dimensions(), 64);
    }

    #[tokio::test]
    async fn test_local_embedding_normalized() {
        let config = EmbeddingConfig {
            dimensions: 16,
            provider: EmbeddingProvider::Local,
            ..Default::default()
        };
        let embedding = LocalEmbedding::new(&config);

        let texts = vec!["normalize me".to_string()];
        let vectors = embedding.embed(&texts).await.unwrap();
        let magnitude: f32 = vectors[0].iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!(
            (magnitude - 1.0).abs() < 1e-5,
            "Vectors should be unit normalized, got magnitude {magnitude}"
        );
    }

    #[tokio::test]
    async fn test_local_embedding_different_inputs() {
        let config = EmbeddingConfig {
            dimensions: 8,
            provider: EmbeddingProvider::Local,
            ..Default::default()
        };
        let embedding = LocalEmbedding::new(&config);

        let texts = vec!["apple".to_string(), "banana".to_string()];
        let vectors = embedding.embed(&texts).await.unwrap();
        // Different inputs should produce different vectors
        assert_ne!(vectors[0], vectors[1]);
    }

    #[tokio::test]
    async fn test_local_embedding_empty_input() {
        let config = EmbeddingConfig {
            dimensions: 4,
            provider: EmbeddingProvider::Local,
            ..Default::default()
        };
        let embedding = LocalEmbedding::new(&config);

        let texts: Vec<String> = vec![];
        let vectors = embedding.embed(&texts).await.unwrap();
        assert!(vectors.is_empty());
    }

    #[tokio::test]
    async fn test_local_embedding_multiple_texts() {
        let config = EmbeddingConfig {
            dimensions: 4,
            ..Default::default()
        };
        let embedding = LocalEmbedding::new(&config);

        let texts = vec![
            "first".to_string(),
            "second".to_string(),
            "third".to_string(),
        ];
        let vectors = embedding.embed(&texts).await.unwrap();
        assert_eq!(vectors.len(), 3);
        assert_eq!(vectors[0].len(), 4);
    }

    // ---- EmbeddingConfig tests ----

    #[test]
    fn test_embedding_config_default() {
        let config = EmbeddingConfig::default();
        assert_eq!(config.dimensions, 384);
        assert_eq!(config.provider, EmbeddingProvider::Local);
        assert!(config.model_path.is_none());
    }

    #[test]
    fn test_embedding_provider_serde() {
        let local = serde_json::to_string(&EmbeddingProvider::Local).unwrap();
        let remote = serde_json::to_string(&EmbeddingProvider::Remote).unwrap();
        assert_eq!(local, "\"Local\"");
        assert_eq!(remote, "\"Remote\"");

        let deserialized: EmbeddingProvider = serde_json::from_str("\"Local\"").unwrap();
        assert_eq!(deserialized, EmbeddingProvider::Local);
    }
}
