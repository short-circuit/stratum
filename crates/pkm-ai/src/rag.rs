use crate::embedding::Embedding;
use crate::provider::{ChatConfig, ChatMessage, ChatResponse, LlmProvider, TokenUsage};
use pkm_core::{PkmResult, SearchResult};
use pkm_index::indexer::IndexEngine;
use serde::{Deserialize, Serialize};

/// A citation linking an answer back to a source note.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Citation {
    /// Path to the source note (vault-relative).
    pub path: String,
    /// Snippet of the relevant content.
    pub snippet: String,
    /// Relevance score (0.0 to 1.0).
    pub score: f32,
}

/// Full response from the RAG pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagResponse {
    /// The LLM-generated answer text.
    pub answer: String,
    /// Source citations that informed the answer.
    pub citations: Vec<Citation>,
    /// Token usage for the LLM call.
    pub usage: TokenUsage,
}

/// The RAG (Retrieval-Augmented Generation) engine.
///
/// Coordinates between the search index, embedding model, and LLM provider
/// to answer questions grounded in the user's notes.
pub struct RagEngine {
    index: IndexEngine,
    embedding: Box<dyn Embedding>,
    provider: Box<dyn LlmProvider>,
}

impl RagEngine {
    /// Create a new RAG engine.
    pub fn new(
        index: IndexEngine,
        embedding: Box<dyn Embedding>,
        provider: Box<dyn LlmProvider>,
    ) -> Self {
        Self {
            index,
            embedding,
            provider,
        }
    }

    /// Run the full RAG pipeline: retrieve relevant chunks, build context,
    /// and ask the LLM.
    pub async fn query(
        &self,
        question: &str,
        config: &ChatConfig,
        top_k: usize,
    ) -> PkmResult<RagResponse> {
        tracing::info!("RAG query: \"{question}\" (top_k={top_k})");

        // Step 1: Retrieve relevant chunks from the index
        let results = self.retrieve(question, top_k).await?;

        tracing::debug!("RAG retrieved {} results", results.len());

        // Step 2: Build context from retrieved chunks
        let context = self.build_context(&results);

        // Step 3: Ask the LLM with question + context
        let response = self.ask_with_context(question, &context, config).await?;

        // Step 4: Build citations from search results
        let citations: Vec<Citation> = results
            .into_iter()
            .map(|r| Citation {
                path: r.path,
                snippet: r.snippet,
                score: r.score as f32,
            })
            .collect();

        tracing::info!(
            "RAG query complete: {} chars, {} citations",
            response.content.len(),
            citations.len()
        );

        Ok(RagResponse {
            answer: response.content,
            citations,
            usage: response.usage,
        })
    }

    /// Retrieve the most relevant chunks from the search index.
    async fn retrieve(&self, query: &str, top_k: usize) -> PkmResult<Vec<SearchResult>> {
        tracing::debug!("RAG retrieve: \"{query}\" (top_k={top_k})");

        // Use full-text search from the index engine
        let results = self.index.search(query, pkm_core::SearchMode::FullText)?;

        // Sort by score descending and take top_k
        let mut sorted = results;
        sorted.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        sorted.truncate(top_k);

        // If available, use embeddings to re-rank results by semantic similarity
        if top_k > 0 && !sorted.is_empty() {
            let reranked = self
                .rerank_with_embeddings(query, &sorted)
                .await
                .unwrap_or_else(|e| {
                    tracing::warn!("Embedding re-ranking failed, using raw scores: {e}");
                    sorted
                });
            return Ok(reranked);
        }

        Ok(sorted)
    }

    /// Re-rank search results using cosine similarity between query and result embeddings.
    async fn rerank_with_embeddings(
        &self,
        query: &str,
        results: &[SearchResult],
    ) -> PkmResult<Vec<SearchResult>> {
        // Build texts to embed: query + all result snippets
        let mut texts = vec![query.to_string()];
        texts.extend(results.iter().map(|r| r.snippet.clone()));

        let vectors = self.embedding.embed(&texts).await?;

        if vectors.len() < 2 {
            return Ok(results.to_vec());
        }

        let query_vector = &vectors[0];
        let result_vectors = &vectors[1..];

        let mut scored: Vec<(f64, &SearchResult)> = results
            .iter()
            .zip(result_vectors.iter())
            .map(|(result, vec)| {
                let sim = crate::embedding::cosine_similarity(query_vector, vec);
                // Blend BM25 score (from index) with semantic similarity
                let blended = result.score * 0.3 + (sim as f64) * 0.7;
                (blended, result)
            })
            .collect();

        // Sort by blended score descending
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        Ok(scored
            .into_iter()
            .map(|(score, result)| SearchResult {
                path: result.path.clone(),
                title: result.title.clone(),
                snippet: result.snippet.clone(),
                score,
                matched_terms: result.matched_terms.clone(),
            })
            .collect())
    }

    /// Build a formatted context string from search results for the LLM prompt.
    fn build_context(&self, results: &[SearchResult]) -> String {
        if results.is_empty() {
            return String::new();
        }

        let mut parts = Vec::new();
        parts.push(
            "Here are relevant excerpts from the user's notes to help answer the question:\n"
                .to_string(),
        );

        for (i, result) in results.iter().enumerate() {
            let citation_text = if result.title.is_empty() {
                result.path.clone()
            } else {
                format!("{} ({})", result.title, result.path)
            };

            parts.push(format!(
                "[Source {}] {}:\n{}\n",
                i + 1,
                citation_text,
                result.snippet.trim()
            ));
        }

        parts.push(
            "\nAnswer the user's question based on the above sources. \
             If the sources don't contain enough information to answer, \
             say so clearly. Cite sources by number when referencing specific information."
                .to_string(),
        );

        parts.join("\n")
    }

    /// Send the question and context to the LLM and get a response.
    async fn ask_with_context(
        &self,
        question: &str,
        context: &str,
        config: &ChatConfig,
    ) -> PkmResult<ChatResponse> {
        let system_prompt = if context.is_empty() {
            "You are a helpful assistant. Answer the user's question based on your \
             general knowledge. If you don't know the answer, say so."
                .to_string()
        } else {
            format!(
                "You are a helpful assistant with access to the user's personal notes. \
                 Use the provided context to answer the question accurately. \
                 Always cite your sources by their number when referencing specific information. \
                 If the context doesn't contain enough information, say so clearly.\n\n\
                 {context}"
            )
        };

        let messages = vec![ChatMessage::user(question)];

        let chat_config = ChatConfig {
            model: config.model.clone(),
            temperature: config.temperature,
            max_tokens: config.max_tokens,
            system_prompt: Some(system_prompt),
        };

        self.provider.chat(&messages, &chat_config).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embedding::{EmbeddingConfig, LocalEmbedding};
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
                    content: "Based on [Source 1], Rust is a systems programming language."
                        .to_string(),
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
                    content: "Rust is a systems programming language focused on safety."
                        .to_string(),
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
            "---\ntitle: Rust Programming\n---\nRust is a systems programming language..."
                .to_string(),
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
                    content:
                        "I don't have enough information in your notes to answer that question."
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
        let config = EmbeddingConfig {
            dimensions: 4,
            ..Default::default()
        };
        Box::new(LocalEmbedding::new(&config))
    }

    fn create_mock_components() -> (Box<dyn Embedding>, Box<dyn LlmProvider>) {
        let config = EmbeddingConfig {
            dimensions: 4,
            ..Default::default()
        };
        let embedding = Box::new(LocalEmbedding::new(&config));

        let ai_config = pkm_core::AiConfig {
            provider: pkm_core::AiProvider::Ollama,
            endpoint: Some("http://localhost:11434".to_string()),
            ..Default::default()
        };
        let provider = ProviderFactory::create(&ai_config).unwrap();

        (embedding, provider)
    }
}
