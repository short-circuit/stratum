use super::*;
use pkm_core::AiProvider;
use wiremock::matchers::{body_partial_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Build a client whose base URL points at the mock server's `/v1` path,
/// mirroring the real resolution performed by `EmbeddingConfig`.
fn client_for(server: &MockServer, model: &str, dimensions: usize) -> OpenAIEmbeddingClient {
    OpenAIEmbeddingClient::new(EmbeddingConfig {
        endpoint: format!("{}/v1", server.uri()),
        api_key: Some("test-key".to_string()),
        model: model.to_string(),
        dimensions,
    })
    .unwrap()
}

fn embeddings_response(model: &str, inputs: usize) -> serde_json::Value {
    let data: Vec<serde_json::Value> = (0..inputs)
        .map(|i| {
            serde_json::json!({
                "index": i,
                "embedding": vec![1.0f32, 0.0f32, 0.0f32]
            })
        })
        .collect();
    serde_json::json!({
        "object": "list",
        "data": data,
        "model": model,
        "usage": { "prompt_tokens": 7, "total_tokens": 7 }
    })
}

// ---- Configuration builder tests ----

#[test]
fn from_ai_config_builds_basic_client() {
    let ai = AiConfig {
        provider: AiProvider::CustomOpenAI,
        endpoint: Some("https://api.example.com/v1".to_string()),
        api_key: Some("sk-abc".to_string()),
        model: "text-embedding-3-small".to_string(),
        ..Default::default()
    };
    let cfg = EmbeddingConfig::from_ai_config(&ai).unwrap();
    assert_eq!(cfg.endpoint, "https://api.example.com/v1");
    assert_eq!(cfg.api_key.as_deref(), Some("sk-abc"));
    assert_eq!(cfg.model, "text-embedding-3-small");
}

#[test]
fn from_ai_config_prefers_embedding_capability_model() {
    let ai = AiConfig {
        provider: AiProvider::Ollama,
        endpoint: Some("http://localhost:11434".to_string()),
        model: "llama3.2".to_string(),
        models: vec![
            pkm_core::AiModelConfig {
                name: "nomic-embed-text".to_string(),
                capabilities: vec!["embedding".to_string()],
            },
            pkm_core::AiModelConfig {
                name: "llama3.2".to_string(),
                capabilities: vec!["chat".to_string()],
            },
        ],
        ..Default::default()
    };
    let cfg = EmbeddingConfig::from_ai_config(&ai).unwrap();
    assert_eq!(cfg.model, "nomic-embed-text");
}

#[test]
fn from_ai_config_ollama_appends_v1() {
    let ai = AiConfig {
        provider: AiProvider::Ollama,
        endpoint: Some("http://localhost:11434".to_string()),
        model: "nomic-embed-text".to_string(),
        ..Default::default()
    };
    let cfg = EmbeddingConfig::from_ai_config(&ai).unwrap();
    assert_eq!(cfg.endpoint, "http://localhost:11434/v1");
}

#[test]
fn from_ai_config_rejects_missing_endpoint() {
    let ai = AiConfig {
        provider: AiProvider::CustomOpenAI,
        endpoint: None,
        ..Default::default()
    };
    let err = EmbeddingConfig::from_ai_config(&ai).unwrap_err();
    assert!(err.to_string().contains("No AI endpoint"));
}

#[test]
fn from_ai_config_does_not_append_v1_for_openai() {
    let ai = AiConfig {
        provider: AiProvider::OpenAI,
        endpoint: Some("https://api.openai.com/v1".to_string()),
        ..Default::default()
    };
    let cfg = EmbeddingConfig::from_ai_config(&ai).unwrap();
    assert_eq!(cfg.endpoint, "https://api.openai.com/v1");
}

#[test]
fn new_rejects_empty_model() {
    let err = OpenAIEmbeddingClient::new(EmbeddingConfig {
        endpoint: "http://127.0.0.1:9/v1".to_string(),
        api_key: None,
        model: String::new(),
        dimensions: 0,
    })
    .unwrap_err();
    assert!(err.to_string().contains("Empty embedding model"));
}

// ---- Mock HTTP server tests (wiremock) ----

#[tokio::test]
async fn sends_payload_and_parses_response() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/embeddings"))
        .and(header("authorization", "Bearer test-key"))
        .and(body_partial_json(serde_json::json!({
            "model": "nomic-embed-text",
            "input": ["hello world", "goodbye"]
        })))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(embeddings_response("nomic-embed-text", 2)),
        )
        .mount(&server)
        .await;

    let client = client_for(&server, "nomic-embed-text", 3);
    let vectors = client
        .embed(&["hello world".to_string(), "goodbye".to_string()])
        .await
        .unwrap();

    assert_eq!(vectors.len(), 2);
    assert_eq!(vectors[0], vec![1.0, 0.0, 0.0]);
    assert_eq!(vectors[1], vec![1.0, 0.0, 0.0]);
}

#[tokio::test]
async fn retries_on_transient_error_then_succeeds() {
    let server = MockServer::start().await;
    // First attempt returns 503, subsequent attempts return 200.
    Mock::given(path("/v1/embeddings"))
        .respond_with(ResponseTemplate::new(503))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(path("/v1/embeddings"))
        .respond_with(ResponseTemplate::new(200).set_body_json(embeddings_response("m", 1)))
        .mount(&server)
        .await;

    let client = client_for(&server, "m", 2);
    let vectors = client.embed(&["only one".to_string()]).await.unwrap();
    assert_eq!(vectors.len(), 1);

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 2, "expected one retry after the 503");
}

#[tokio::test]
async fn retries_then_surfaces_error_after_exhaustion() {
    let server = MockServer::start().await;
    // Always 503 — should be exhausted after MAX_ATTEMPTS.
    Mock::given(path("/v1/embeddings"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;

    let client = client_for(&server, "m", 2);
    let err = client.embed(&["will fail".to_string()]).await.unwrap_err();
    assert!(err.to_string().contains("503"));

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), MAX_ATTEMPTS);
}

#[tokio::test]
async fn surfaces_error_body_message() {
    let server = MockServer::start().await;
    Mock::given(path("/v1/embeddings"))
        .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "error": { "message": "invalid input", "type": "invalid_request_error" }
        })))
        .mount(&server)
        .await;

    let client = client_for(&server, "m", 2);
    let err = client.embed(&["bad".to_string()]).await.unwrap_err();
    assert!(err.to_string().contains("invalid input"));
}

#[tokio::test]
async fn dimensions_inferred_from_first_response() {
    let server = MockServer::start().await;
    Mock::given(path("/v1/embeddings"))
        .respond_with(ResponseTemplate::new(200).set_body_json(embeddings_response("m", 1)))
        .mount(&server)
        .await;

    let client = client_for(&server, "m", 0);
    let _ = client.embed(&["x".to_string()]).await.unwrap();
    assert_eq!(client.dimensions(), 3);
}

#[tokio::test]
async fn empty_input_is_a_noop() {
    let server = MockServer::start().await;
    let client = client_for(&server, "m", 3);
    let vectors: Vec<Vec<f32>> = client.embed(&[]).await.unwrap();
    assert!(vectors.is_empty());
    // No request should have been made.
    let requests = server.received_requests().await;
    assert!(requests.is_none() || requests.unwrap().is_empty());
}

#[tokio::test]
async fn returns_vectors_in_input_same_order() {
    let server = MockServer::start().await;
    let data = serde_json::json!([
        { "index": 0, "embedding": vec![0.1f32, 0.2f32] },
        { "index": 1, "embedding": vec![0.3f32, 0.4f32] }
    ]);
    Mock::given(path("/v1/embeddings"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "object": "list",
            "data": data,
            "model": "m"
        })))
        .mount(&server)
        .await;

    let client = client_for(&server, "m", 2);
    let vectors = client
        .embed(&["a".to_string(), "b".to_string()])
        .await
        .unwrap();
    assert_eq!(vectors[0], vec![0.1, 0.2]);
    assert_eq!(vectors[1], vec![0.3, 0.4]);
}

#[tokio::test]
async fn no_api_key_omits_auth_header() {
    let server = MockServer::start().await;
    Mock::given(path("/v1/embeddings"))
        .respond_with(ResponseTemplate::new(200).set_body_json(embeddings_response("m", 1)))
        .mount(&server)
        .await;

    let client = OpenAIEmbeddingClient::new(EmbeddingConfig {
        endpoint: format!("{}/v1", server.uri()),
        api_key: None,
        model: "m".to_string(),
        dimensions: 2,
    })
    .unwrap();
    let vectors = client.embed(&["x".to_string()]).await.unwrap();
    assert_eq!(vectors.len(), 1);

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    let headers = requests[0].headers.clone();
    assert!(
        !headers
            .iter()
            .any(|(k, _)| k.as_str().eq_ignore_ascii_case("authorization")),
        "request must not carry an Authorization header when no API key is set"
    );
}

// ---- Cosine similarity tests (unchanged behavior) ----

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
    assert!((matrix[0][0] - 1.0).abs() < 1e-6);
    assert!((matrix[0][1] - 0.70710677).abs() < 1e-4);
    assert!(matrix[1][0].abs() < 1e-6);
    assert!((matrix[1][1] - 0.70710677).abs() < 1e-4);
}

// ---- Config selection tests ----

#[test]
fn test_embedding_config_from_ai_config_defaults() {
    let ai = AiConfig {
        provider: AiProvider::CustomOpenAI,
        endpoint: Some("https://example.com/v1".to_string()),
        api_key: None,
        model: "text-embedding-3-small".to_string(),
        ..Default::default()
    };
    let cfg = EmbeddingConfig::from_ai_config(&ai).unwrap();
    assert_eq!(cfg.dimensions, 0); // inferred from first response
    assert_eq!(cfg.model, "text-embedding-3-small");
    assert_eq!(cfg.api_key, ai.effective_api_key());
}
