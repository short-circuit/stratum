//! Integration tests wiring real embeddings + RAG retrieval end-to-end.
//!
//! These tests exercise the real embedding client (OpenAI-compatible), the real
//! LLM provider, and the real retrieval path against an in-process wiremock
//! server — no in-memory fakes.

use pkm_ai::provider::{ChatConfig, ProviderFactory};
use pkm_core::{AiConfig, AiModelConfig, AiProvider, Frontmatter, Note};
use pkm_index::indexer::IndexEngine;
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const EMBED_MODEL: &str = "nomic-embed-text";
const CHAT_MODEL: &str = "llama3.2";

/// Helper to mount the OpenAI-compatible embeddings + chat routes on a mock
/// server, returning the running server.
async fn mount_ai_server() -> MockServer {
    let server = MockServer::start().await;

    // POST /v1/embeddings → OpenAI-compatible response with per-input vectors.
    Mock::given(method("POST"))
        .and(path("/v1/embeddings"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "object": "list",
            "data": [
                { "index": 0, "embedding": [0.1, 0.0, 0.0, 0.0] },
                { "index": 1, "embedding": [0.1, 0.0, 0.0, 0.0] }
            ],
            "model": EMBED_MODEL,
            "usage": { "prompt_tokens": 10, "total_tokens": 10 }
        })))
        .mount(&server)
        .await;

    // POST /chat/completions → OpenAI-compatible LLM response. The provider's
    // endpoint is configured as `{server}/v1`, so the chat URL is
    // `{server}/v1/chat/completions`.
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            serde_json::json!({
                "choices": [{ "message": { "role": "assistant", "content": "Rust is a systems programming language that guarantees memory safety." } }],
                "usage": { "prompt_tokens": 10, "completion_tokens": 5 }
            }),
        ))
        .mount(&server)
        .await;

    server
}

#[tokio::test]
async fn rag_query_uses_real_embeddings_and_returns_ranked_chunks() {
    let server = mount_ai_server().await;

    // Build a temp vault with one indexable note so retrieval returns a chunk.
    let vault = TempDir::new().unwrap();
    let mut engine = IndexEngine::new(vault.path()).unwrap();
    let note = Note::new(
        vault.path().join("rust.md"),
        vault.path(),
        Frontmatter {
            title: Some("Rust".into()),
            ..Default::default()
        },
        "Rust is a systems programming language that guarantees memory safety.".into(),
        "Rust is a systems programming language that guarantees memory safety.".into(),
        vec![],
        vec![],
        chrono::Utc::now(),
    );
    engine.index_note(&note).unwrap();
    engine.flush().unwrap();

    // Real embedding client + real provider, both pointed at the mock server.
    let ai = AiConfig {
        provider: AiProvider::OpenAI,
        endpoint: Some(format!("{}/v1", server.uri())),
        api_key: Some("test-key".into()),
        model: CHAT_MODEL.into(),
        models: vec![AiModelConfig {
            name: EMBED_MODEL.into(),
            capabilities: vec!["embedding".into()],
        }],
        ..Default::default()
    };

    let embedding = pkm_ai::embedding::OpenAIEmbeddingClient::from_ai_config(&ai).unwrap();
    let provider = ProviderFactory::create(&ai).unwrap();
    let rag = pkm_ai::rag::RagEngine::new(engine, Box::new(embedding), provider);

    let response = rag
        .query("What is Rust?", &ChatConfig::new(CHAT_MODEL), 3)
        .await
        .unwrap();

    assert!(!response.answer.is_empty());
    assert_eq!(response.citations.len(), 1);
    assert_eq!(response.citations[0].path, "rust.md");
}
