use super::*;
use crate::provider::{ChatDelta, ProviderFactory};
use futures::stream::BoxStream;
use mockall::mock;

// ---- Mock Embedding ----

mock! {
    pub EmbeddingMock {}
    #[async_trait::async_trait]
    impl Embedding for EmbeddingMock {
        async fn embed(&self, texts: &[String]) -> PkmResult<Vec<Vec<f32>>>;
        fn dimensions(&self) -> usize;
    }
}

// ---- Mock LlmProvider ----

mock! {
    pub RagLlmProviderMock {}
    #[async_trait::async_trait]
    impl LlmProvider for RagLlmProviderMock {
        async fn chat(
            &self,
            messages: &[ChatMessage],
            config: &ChatConfig,
        ) -> PkmResult<ChatResponse>;

        async fn stream_chat(
            &self,
            messages: &[ChatMessage],
            config: &ChatConfig,
        ) -> PkmResult<BoxStream<'static, PkmResult<ChatDelta>>>;
    }
}

// ---- Mock IndexEngine (wrapping, since it's not trait-based) ----

/// Helper to create mock search results for testing.
fn make_search_results(count: usize) -> Vec<SearchResult> {
    (0..count)
        .map(|i| SearchResult {
            path: format!("notes/note-{}.md", i + 1),
            title: format!("Note {}", i + 1),
            snippet: format!(
                "This is the content of note number {}. It contains relevant information \
                     about the topic being discussed in the user's question.",
                i + 1
            ),
            score: 1.0 - (i as f64 * 0.1),
            matched_terms: vec!["test".to_string()],
        })
        .collect()
}

#[test]
fn test_build_context_with_results() {
    let (embedding, provider) = create_mock_components();
    let index = create_temp_index();
    let engine = RagEngine::new(index, embedding, provider);

    let results = make_search_results(3);
    let context = engine.build_context(&results);

    assert!(context.contains("[Source 1]"));
    assert!(context.contains("[Source 2]"));
    assert!(context.contains("[Source 3]"));
    assert!(context.contains("Note 1"));
    assert!(context.contains("notes/note-1.md"));
    assert!(context.contains("Answer the user's question"));
    assert!(context.contains("Cite sources by number"));
}

#[test]
fn test_build_context_empty() {
    let (embedding, provider) = create_mock_components();
    let index = create_temp_index();
    let engine = RagEngine::new(index, embedding, provider);

    let context = engine.build_context(&[]);
    assert!(context.is_empty());
}

#[test]
fn test_build_context_single_result() {
    let (embedding, provider) = create_mock_components();
    let index = create_temp_index();
    let engine = RagEngine::new(index, embedding, provider);

    let results = make_search_results(1);
    let context = engine.build_context(&results);

    assert!(context.contains("[Source 1]"));
    assert!(context.contains("Note 1 (notes/note-1.md)"));
    assert!(!context.contains("[Source 2]"));
}

#[test]
fn test_build_context_empty_title() {
    let (embedding, provider) = create_mock_components();
    let index = create_temp_index();
    let engine = RagEngine::new(index, embedding, provider);

    let results = vec![SearchResult {
        path: "notes/untitled.md".to_string(),
        title: String::new(),
        snippet: "Content without a title.".to_string(),
        score: 0.8,
        matched_terms: vec![],
    }];
    let context = engine.build_context(&results);

    assert!(context.contains("notes/untitled.md"));
    // Should use path instead of empty title
    assert!(context.contains("[Source 1]"));
}

#[tokio::test]
async fn test_ask_with_context_empty() {
    let embedding = create_mock_embedding();
    let mut provider = MockRagLlmProviderMock::new();
    provider
        .expect_chat()
        .with(mockall::predicate::always(), mockall::predicate::always())
        .returning(|_, _| {
            Ok(ChatResponse {
                content: "I can't answer without context.".to_string(),
                usage: TokenUsage::default(),
            })
        });

    let index = create_temp_index();
    let engine = RagEngine::new(index, embedding, Box::new(provider));

    // Call ask_with_context with empty context to test system prompt generation
    let resp = engine
        .ask_with_context("What is Rust?", "", &ChatConfig::default())
        .await
        .unwrap();
    assert_eq!(resp.content, "I can't answer without context.");
}

#[tokio::test]
async fn test_ask_with_context_provided() {
    let embedding = create_mock_embedding();
    let mut provider = MockRagLlmProviderMock::new();
    provider
        .expect_chat()
        .with(mockall::predicate::always(), mockall::predicate::always())
        .returning(|_, _| {
            Ok(ChatResponse {
                content: "Based on [Source 1], Rust is a systems programming language.".to_string(),
                usage: TokenUsage {
                    prompt_tokens: 50,
                    completion_tokens: 15,
                },
            })
        });

    let index = create_temp_index();
    let engine = RagEngine::new(index, embedding, Box::new(provider));

    let context = "Relevant context about Rust.";
    let resp = engine
        .ask_with_context("What is Rust?", context, &ChatConfig::default())
        .await
        .unwrap();
    assert!(resp.content.contains("[Source 1]"));
    assert_eq!(resp.usage.prompt_tokens, 50);
    assert_eq!(resp.usage.completion_tokens, 15);
}

#[tokio::test]
async fn test_query_with_mocks() {
    let mut embedding = MockEmbeddingMock::new();
    embedding.expect_embed().returning(|texts| {
        // Return normalized unit vectors
        let dim = 4;
        Ok(texts
            .iter()
            .map(|_| vec![0.5f32; dim]) // same vector = high similarity
            .collect())
    });
    embedding.expect_dimensions().return_const(4usize);

    let mut provider = MockRagLlmProviderMock::new();
    provider
        .expect_chat()
        .with(mockall::predicate::always(), mockall::predicate::always())
        .returning(|_, _| {
            Ok(ChatResponse {
                content: "Rust is a systems programming language focused on safety.".to_string(),
                usage: TokenUsage {
                    prompt_tokens: 100,
                    completion_tokens: 20,
                },
            })
        });

    let mut index = create_temp_index();
    // Index a note so search returns results
    let vault_root = std::path::PathBuf::from("/vault");
    let note = pkm_core::Note::new(
        vault_root.join("rust.md"),
        &vault_root,
        pkm_core::Frontmatter {
            title: Some("Rust Programming".to_string()),
            ..Default::default()
        },
        "Rust is a systems programming language that focuses on safety and performance."
            .to_string(),
        "---\ntitle: Rust Programming\n---\nRust is a systems programming language...".to_string(),
        vec![],
        vec![],
        chrono::Utc::now(),
    );
    index.index_note(&note).unwrap();
    index.flush().unwrap();

    let engine = RagEngine::new(index, Box::new(embedding), Box::new(provider));

    let config = ChatConfig::new("llama3.2");
    let response = engine.query("What is Rust?", &config, 5).await.unwrap();

    assert!(response.answer.contains("Rust"));
    assert!(
        !response.citations.is_empty(),
        "Expected at least one citation"
    );
}

#[tokio::test]
async fn test_query_no_results() {
    let embedding = create_mock_embedding();
    let mut provider = MockRagLlmProviderMock::new();
    provider
        .expect_chat()
        .with(mockall::predicate::always(), mockall::predicate::always())
        .returning(|_, _| {
            Ok(ChatResponse {
                content: "I don't have enough information in your notes to answer that question."
                    .to_string(),
                usage: TokenUsage::default(),
            })
        });

    let index = create_temp_index();
    let engine = RagEngine::new(index, embedding, Box::new(provider));

    let config = ChatConfig::new("llama3.2");
    let response = engine
        .query("Something completely unrelated xyz123", &config, 5)
        .await
        .unwrap();

    // Should still get an answer, just without citations
    assert!(!response.answer.is_empty());
    assert!(response.citations.is_empty());
}

// ---- Helper functions ----

fn create_temp_index() -> IndexEngine {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().to_path_buf();
    // Leak the TempDir so the directory stays alive for the test
    std::mem::forget(dir);
    IndexEngine::new(&path).unwrap()
}

fn create_mock_embedding() -> Box<dyn Embedding> {
    let mut embedding = MockEmbeddingMock::new();
    embedding.expect_dimensions().return_const(4usize);
    embedding.expect_embed().returning(|texts| {
        // Deterministic unit vectors so cosine similarity is well-defined.
        Ok(texts
            .iter()
            .enumerate()
            .map(|(i, _)| {
                let mut v = vec![0.0f32; 4];
                v[i % 4] = 1.0;
                v
            })
            .collect())
    });
    Box::new(embedding)
}

fn create_mock_components() -> (Box<dyn Embedding>, Box<dyn LlmProvider>) {
    let embedding = create_mock_embedding();

    let ai_config = pkm_core::AiConfig {
        provider: pkm_core::AiProvider::Ollama,
        endpoint: Some("http://localhost:11434".to_string()),
        ..Default::default()
    };
    let provider = ProviderFactory::create(&ai_config).unwrap();

    (embedding, provider)
}
