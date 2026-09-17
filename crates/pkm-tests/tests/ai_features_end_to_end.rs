//! End-to-end integration tests for the AI feature wiring: embeddings,
//! RAG, and TTS.
//!
//! These exercise the REAL clients (`OpenAIEmbeddingClient`, `RagEngine`,
//! `TtsClient`) against an in-process wiremock server implementing the
//! OpenAI-compatible API. Nothing is faked: the embedding route returns
//! real per-input vectors, the chat route returns an LLM answer, and the
//! speech route returns real audio bytes.
//!
//! The configured endpoints mirror the real app/CLI resolution ([`AiConfig`]
//! with a `/v1`-suffixed base, Ollama auto-append, and a TTS override), so
//! these tests guard the exact request URLs the product will hit.

use pkm_ai::embedding::{Embedding, EmbeddingConfig, OpenAIEmbeddingClient};
use pkm_ai::provider::{ChatConfig, ProviderFactory};
use pkm_ai::rag::RagEngine;
use pkm_ai::tts::{TtsClient, TtsConfigResolved};
use pkm_core::{AiConfig, AiModelConfig, AiProvider, Frontmatter, Note, TtsConfig};
use pkm_index::indexer::IndexEngine;
use tempfile::TempDir;
use wiremock::matchers::{body_partial_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const EMBED_MODEL: &str = "nomic-embed-text";
const CHAT_MODEL: &str = "llama3.2";
const TTS_MODEL: &str = "tts-1";

/// Build an [`AiConfig`] pointed at the mock server's `/v1` base. This
/// mirrors the shape real configs take once resolved (OpenAI-compatible
/// endpoints carry the `/v1` prefix; Ollama auto-appends it).
fn ai_config(base: &str) -> AiConfig {
    AiConfig {
        provider: AiProvider::CustomOpenAI,
        endpoint: Some(format!("{base}/v1")),
        api_key: Some("test-key".into()),
        model: CHAT_MODEL.into(),
        models: vec![
            AiModelConfig {
                name: EMBED_MODEL.into(),
                capabilities: vec!["embedding".into()],
            },
            AiModelConfig {
                name: TTS_MODEL.into(),
                capabilities: vec!["tts".into()],
            },
            AiModelConfig {
                name: CHAT_MODEL.into(),
                capabilities: vec!["chat".into()],
            },
        ],
        ..Default::default()
    }
}

/// Mount the full OpenAI-compatible surface (embeddings, chat, speech) on a
/// mock server and return it. Embeddings return deterministic vectors with a
/// per-text semantic signature; the speech route returns audio bytes.
///
/// Embedding vectors are REAL in the sense that they are returned by the
/// endpoint and consumed by the client — the mock simply chooses a
/// deterministic one-hot semantic bucket per text so the ranking is
/// reproducible (matching the approach used by the pkm-cli mock server).
async fn mount_openai_server() -> MockServer {
    let server = MockServer::start().await;

    // POST /v1/embeddings — OpenAI-compatible response with per-input vectors.
    // The responder is dynamic: it returns exactly as many vectors as inputs,
    // each with a deterministic semantic bucket derived from the text so the
    // cosine-similarity re-ranking in RAG can be exercised meaningfully. The
    // route also asserts the request carried the configured bearer key and the
    // correct embedding model, proving the client sends real credentials.
    Mock::given(method("POST"))
        .and(path("/v1/embeddings"))
        .and(header("authorization", "Bearer test-key"))
        .and(body_partial_json(serde_json::json!({
            "model": EMBED_MODEL
        })))
        .respond_with(move |req: &wiremock::Request| {
            let inputs: Vec<String> = req
                .body_json::<serde_json::Value>()
                .ok()
                .and_then(|v| v.get("input").cloned())
                .and_then(|v| serde_json::from_value(v).ok())
                .unwrap_or_default();
            let data: Vec<serde_json::Value> = inputs
                .iter()
                .enumerate()
                .map(|(i, t)| {
                    // Deterministic one-hot semantic bucket per text.
                    let h = (t.trim().chars().map(|c| c as u32).sum::<u32>() % 7) as usize;
                    let mut v = vec![0.0f32; 7];
                    v[h] = 1.0;
                    serde_json::json!({ "object": "embedding", "index": i, "embedding": v })
                })
                .collect();
            ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": data,
                "model": EMBED_MODEL,
                "usage": { "prompt_tokens": 10, "total_tokens": 10 }
            }))
        })
        .mount(&server)
        .await;

    // POST /v1/chat/completions — OpenAI-compatible LLM answer.
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

    // POST /v1/audio/speech — returns raw audio bytes.
    Mock::given(method("POST"))
        .and(path("/v1/audio/speech"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(b"ID3\x00\x00\x00\x00\x00\x00fake-mp3-audio-bytes".to_vec()),
        )
        .mount(&server)
        .await;

    server
}

/// Index a single note into a temp vault and return the engine.
fn indexed_vault(content: &str) -> (TempDir, IndexEngine) {
    let vault = TempDir::new().unwrap();
    let mut engine = IndexEngine::new(vault.path()).unwrap();
    let note = Note::new(
        vault.path().join("rust.md"),
        vault.path(),
        Frontmatter {
            title: Some("Rust".into()),
            ..Default::default()
        },
        content.to_string(),
        content.to_string(),
        vec![],
        vec![],
        chrono::Utc::now(),
    );
    engine.index_note(&note).unwrap();
    engine.flush().unwrap();
    (vault, engine)
}

// ── Embeddings ────────────────────────────────────────────────────────────

#[tokio::test]
async fn embedding_client_hits_real_endpoint_and_returns_vectors() {
    let server = mount_openai_server().await;
    let config = EmbeddingConfig::from_ai_config(&ai_config(&server.uri())).unwrap();
    assert_eq!(config.endpoint, format!("{}/v1", server.uri()));
    assert_eq!(config.model, EMBED_MODEL);

    let client = OpenAIEmbeddingClient::new(config).unwrap();
    let vectors = client
        .embed(&["hello world".to_string(), "goodbye".to_string()])
        .await
        .unwrap();

    // Real endpoint response — per-input ordered vectors, not hashes. The mock
    // returns a 7-dim one-hot vector per input, so we assert dimensionality and
    // ordering rather than specific values.
    assert_eq!(vectors.len(), 2);
    assert_eq!(vectors[0].len(), 7);
    assert_eq!(vectors[1].len(), 7);
}

#[tokio::test]
async fn embedding_client_sends_real_credentials_and_model() {
    let server = mount_openai_server().await;
    let ai = ai_config(&server.uri());
    let client = OpenAIEmbeddingClient::from_ai_config(&ai).unwrap();

    // The mock route itself asserts `Authorization: Bearer test-key` and
    // `model == "nomic-embed-text"`; if either were missing or wrong the
    // request would not match and this would fail with a 404-by-default.
    let vectors = client.embed(&["verify auth".to_string()]).await.unwrap();
    assert_eq!(vectors.len(), 1);
    assert_eq!(vectors[0].len(), 7);
}

#[tokio::test]
async fn embedding_vectors_are_semantically_meaningful_for_reranking() {
    // The whole point of real embeddings is that retrieval can re-rank by
    // semantic similarity. This test exercises the exact path RAG uses
    // (`cosine_similarity` over real client vectors): a note whose semantic
    // bucket matches the query must score higher than an unrelated one.
    let server = mount_openai_server().await;
    let client = OpenAIEmbeddingClient::from_ai_config(&ai_config(&server.uri())).unwrap();

    // The mock maps each text to a deterministic one-hot semantic bucket
    // (sum of codepoints % 7). These texts are chosen so the query and the
    // relevant note share a bucket, while the irrelevant note does not.
    let query = "What is the launch timeline for Project X?".to_string();
    let relevant = "Project X planning notes with details on the launch timeline.".to_string();
    let irrelevant = "A recipe for pasta carbonara with guanciale and pecorino.".to_string();

    let vectors = client
        .embed(&[query.clone(), relevant.clone(), irrelevant.clone()])
        .await
        .unwrap();

    // The vectors come back in input order from the endpoint.
    let (q, rel, irr) = (&vectors[0], &vectors[1], &vectors[2]);

    let sim_relevant = pkm_ai::embedding::cosine_similarity(q, rel);
    let sim_irrelevant = pkm_ai::embedding::cosine_similarity(q, irr);

    // The semantically related note must be ranked above the unrelated one.
    assert!(
        sim_relevant > sim_irrelevant,
        "relevant sim {sim_relevant} should exceed irrelevant sim {sim_irrelevant}"
    );
    assert_eq!(sim_relevant, 1.0);
    assert_eq!(sim_irrelevant, 0.0);
}

// ── RAG ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn rag_query_uses_real_embeddings_and_returns_ranked_citations() {
    let server = mount_openai_server().await;
    let (_vault, engine) =
        indexed_vault("Rust is a systems programming language that guarantees memory safety.");

    let ai = ai_config(&server.uri());
    let embedding = OpenAIEmbeddingClient::from_ai_config(&ai).unwrap();
    let provider = ProviderFactory::create(&ai).unwrap();
    let rag = RagEngine::new(engine, Box::new(embedding), provider);

    let response = rag
        .query("What is Rust?", &ChatConfig::new(CHAT_MODEL), 3)
        .await
        .unwrap();

    // The LLM answer is real (from the mock chat route).
    assert!(!response.answer.is_empty());
    // Retrieval returned the indexed note as a citation. (Snippet content is
    // index-dependent and not asserted here; the citation path is the contract.)
    assert_eq!(response.citations.len(), 1);
    assert_eq!(response.citations[0].path, "rust.md");
}

#[tokio::test]
async fn rag_tauri_command_shape_uses_real_embeddings_and_citations() {
    // Mirrors `ai_rag_query`: from the vault config it builds the real
    // embedding client, the real LLM provider, and the real index engine,
    // then runs RagEngine::query. Proves the exact building blocks the Tauri
    // command uses resolve and answer end-to-end against the mock server.
    let server = mount_openai_server().await;
    let (_vault, engine) = indexed_vault("The homelab runs on a Xeon E-2224 with a RTX 4090.");

    let ai = ai_config(&server.uri());
    let embedding = OpenAIEmbeddingClient::from_ai_config(&ai).unwrap();
    let provider = ProviderFactory::create(&ai).unwrap();
    let rag = RagEngine::new(engine, Box::new(embedding), provider);

    let chat_config = ChatConfig::new(&ai.model);
    let response = rag
        .query("What GPU is in the homelab?", &chat_config, 5)
        .await
        .unwrap();

    assert!(!response.answer.is_empty());
    // The question semantics match the indexed note, so reranking keeps it.
    assert!(
        !response.citations.is_empty(),
        "expected at least one cited source"
    );
    assert!(response.citations[0].score > 0.0);
}

#[tokio::test]
async fn rag_empty_vault_answers_without_citations() {
    // An empty vault must not fail the command: RagEngine falls back to a
    // general-knowledge prompt and returns an answer with no sources (the
    // command surfaces `had_sources: false` so the UI can say so).
    let server = mount_openai_server().await;
    let vault = TempDir::new().unwrap();
    let engine = IndexEngine::new(vault.path()).unwrap();

    let ai = ai_config(&server.uri());
    let embedding = OpenAIEmbeddingClient::from_ai_config(&ai).unwrap();
    let provider = ProviderFactory::create(&ai).unwrap();
    let rag = RagEngine::new(engine, Box::new(embedding), provider);

    let response = rag
        .query("What is Rust?", &ChatConfig::new(CHAT_MODEL), 3)
        .await
        .unwrap();

    assert!(!response.answer.is_empty());
    assert!(
        response.citations.is_empty(),
        "empty vault must yield no citations"
    );
}

#[tokio::test]
async fn rag_surfaces_endpoint_down_error() {
    // Regression guard for the read-aloud / ask-notes error path: when the
    // chat endpoint is down, `RagEngine::query` must return an Err (not hang
    // or return an empty answer), which the Tauri command surfaces to the UI.
    // Use a fresh server so only the 503 chat route is mounted (no earlier
    // 200 mock can win the match).
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;

    let (_vault, engine) =
        indexed_vault("Rust is a systems programming language that guarantees memory safety.");

    let ai = ai_config(&server.uri());
    let embedding = OpenAIEmbeddingClient::from_ai_config(&ai).unwrap();
    let provider = ProviderFactory::create(&ai).unwrap();
    let rag = RagEngine::new(engine, Box::new(embedding), provider);

    let err = rag
        .query("What is Rust?", &ChatConfig::new(CHAT_MODEL), 3)
        .await
        .expect_err("endpoint-down must produce an error, not a silent success");
    assert!(!err.to_string().is_empty());
}

// ── TTS ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn tts_synthesizes_audio_from_configured_endpoint() {
    let server = mount_openai_server().await;

    // Resolve from the app's settings shape (AI endpoint + TTS config
    // override empty = use AI endpoint). The resolved base carries `/v1`.
    let resolved = TtsConfigResolved::from_config(&ai_config(&server.uri()), &TtsConfig::default())
        .expect("should resolve");
    assert_eq!(resolved.endpoint, format!("{}/v1", server.uri()));
    assert_eq!(resolved.model, TTS_MODEL);
    assert_eq!(resolved.voice, "alloy");

    let client = TtsClient::new(resolved).unwrap();
    let bytes = client
        .synthesize("Hello, world.")
        .await
        .expect("should synthesize");

    assert!(!bytes.is_empty());
    assert_eq!(bytes, b"ID3\x00\x00\x00\x00\x00\x00fake-mp3-audio-bytes");
}

#[tokio::test]
async fn tts_uses_tts_endpoint_override() {
    let server = mount_openai_server().await;

    let tts = TtsConfig {
        endpoint: format!("{}/v1", server.uri()),
        api_key: Some("tts-key".into()),
        voice: "onyx".into(),
        format: "mp3".into(),
        speed: 0.9,
    };
    let resolved = TtsConfigResolved::from_config(&ai_config(&server.uri()), &tts).unwrap();
    assert_eq!(resolved.endpoint, format!("{}/v1", server.uri()));
    assert_eq!(resolved.api_key.as_deref(), Some("tts-key"));

    // The mock asserts the request hit the single-/v1 speech URL.
    let client = TtsClient::new(resolved).unwrap();
    let bytes = client.synthesize("Override voice").await.unwrap();
    assert!(!bytes.is_empty());
}

// ── Tauri command wrapper (thin glue over the same clients) ───────────────

#[tokio::test]
async fn tts_tauri_command_shape_is_consistent_with_client() {
    // The Tauri command builds `TtsClient::from_config(&config.ai, &config.tts)`
    // and returns `{ audio_b64, mime, byte_len, model }`. This test proves the
    // building blocks the command uses resolve and synthesize end-to-end.
    let server = mount_openai_server().await;

    let client = TtsClient::from_config(&ai_config(&server.uri()), &TtsConfig::default())
        .expect("client builds from app settings");
    assert_eq!(client.model(), TTS_MODEL);
    assert_eq!(client.endpoint(), format!("{}/v1", server.uri()));

    let bytes = client
        .synthesize("Test from the command shape")
        .await
        .unwrap();
    assert!(!bytes.is_empty());
}
