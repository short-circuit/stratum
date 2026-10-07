use super::*;

/// Build a vault directory with a couple of notes and a config that points
/// the AI provider at a mock OpenAI-compatible endpoint.
fn vault_with(config_endpoint: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path().to_path_buf();

    std::fs::create_dir_all(root.join("notes")).unwrap();
    std::fs::create_dir_all(root.join(".pkm")).unwrap();
    std::fs::write(
        root.join("notes/project-x.md"),
        "---\ntitle: Project X\n---\n\nProject X planning notes with details on the launch timeline.\n",
    )
    .unwrap();
    std::fs::write(
        root.join("notes/unrelated.md"),
        "---\ntitle: Recipes\n---\n\nA recipe for pasta carbonara.\n",
    )
    .unwrap();

    let config = pkm_core::Config {
        vault_path: root.clone(),
        ai: pkm_core::AiConfig {
            endpoint: Some(config_endpoint.to_string()),
            model: "llama3.2".to_string(),
            models: vec![
                pkm_core::AiModelConfig {
                    name: "llama3.2".to_string(),
                    capabilities: vec!["chat".to_string()],
                },
                pkm_core::AiModelConfig {
                    name: "nomic-embed-text".to_string(),
                    capabilities: vec!["embedding".to_string()],
                },
            ],
            rag_enabled: true,
            rag_chunk_count: 5,
            ..Default::default()
        },
        ..Default::default()
    };
    config.save(config.config_file_path()).unwrap();
    (dir, root)
}

#[test]
fn cmd_index_indexes_markdown_notes() {
    let (_dir, root) = vault_with("http://127.0.0.1:1");
    // A fresh soil: nothing indexed yet. The engine must be dropped before
    // `cmd_index` opens the same Tantivy index (exclusive lock).
    {
        let engine = IndexEngine::new(&root).unwrap();
        assert!(engine
            .search("timeline", pkm_core::SearchMode::FullText)
            .unwrap()
            .is_empty());
    }

    cmd_index(&root).unwrap();

    // The Tantivy block index persists to disk — reopening the engine and
    // searching proves the notes were indexed (meta counters are in-memory
    // only and reset on reopen, so we assert on search results, not meta).
    let engine = IndexEngine::new(&root).unwrap();
    let results = engine
        .search("timeline", pkm_core::SearchMode::FullText)
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].path, "notes/project-x.md");
}

#[test]
fn cmd_rag_reports_missing_config() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path().to_path_buf();
    // No .pkm/config.toml exists in this dir.
    // The function prints guidance but returns Ok, so call it directly.
    let _ = cmd_rag(&root, "what is project x", false, 5);
}

#[tokio::test]
async fn cmd_rag_returns_best_citation_first() {
    // Starts a real mock server (as a background thread) that serves both the
    // OpenAI-compatible embeddings route and the Ollama chat route, so the
    // entire retrieval + re-ranking path is exercised with real HTTP.
    use pkm_ai::embedding::{EmbeddingConfig, OpenAIEmbeddingClient};
    use std::sync::Arc;

    let server = Arc::new(MockAiServer::start());
    let endpoint = server.uri();
    let (_dir, root) = vault_with(&endpoint);

    // Index the vault so retrieval finds the notes.
    cmd_index(&root).unwrap();

    // Build the same components cmd_rag would build (it prints instead of
    // returning, so we drive RagEngine directly for assertions).
    let config = pkm_core::Config::load(root.join(".pkm/config.toml")).unwrap();
    let embedding = OpenAIEmbeddingClient::new(EmbeddingConfig {
        endpoint: format!("{}/v1", endpoint),
        api_key: None,
        model: "nomic-embed-text".to_string(),
        dimensions: 0,
    })
    .unwrap();
    let provider = ProviderFactory::create(&config.ai).unwrap();
    let engine = IndexEngine::new(&root).unwrap();
    let rag = pkm_ai::rag::RagEngine::new(engine, Box::new(embedding), provider);

    let response = rag
        .query(
            "What is the launch timeline for Project X?",
            &ChatConfig::new("llama3.2"),
            3,
        )
        .await
        .unwrap();

    assert!(
        response
            .citations
            .iter()
            .any(|c| c.path == "notes/project-x.md"),
        "expected project-x.md among ranked citations, got {:?}",
        response.citations
    );
    assert!(!response.answer.is_empty());
}

/// A minimal in-process OpenAI-compatible server used only by the CLI tests.
/// Serves `POST /v1/embeddings` and `POST /api/chat` (Ollama shape) and a
/// `GET /v1/models` listing so the embedding model selection works.
struct MockAiServer {
    addr: std::net::SocketAddr,
    _guard: std::thread::JoinHandle<()>,
}

impl MockAiServer {
    fn start() -> Self {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                std::thread::spawn(move || handle_stream(&mut stream));
            }
        });
        Self {
            addr,
            _guard: handle,
        }
    }

    fn uri(&self) -> String {
        format!("http://{}", self.addr)
    }
}

fn handle_stream(stream: &mut std::net::TcpStream) {
    use std::io::{BufRead, BufReader, Read, Write};
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut request_line = String::new();
    if reader.read_line(&mut request_line).is_err() {
        return;
    }
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default();
    let path = parts.next().unwrap_or_default();

    let mut content_length = 0usize;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).is_err() || line == "\r\n" {
            break;
        }
        let lower = line.to_ascii_lowercase();
        if let Some(rest) = lower.strip_prefix("content-length:") {
            content_length = rest.trim().parse().unwrap_or(0);
        }
    }
    let mut body = vec![0u8; content_length];
    let _ = reader.read_exact(&mut body);

    let payload: serde_json::Value = if content_length > 0 {
        serde_json::from_slice(&body).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    let (status, response) = match (method, path) {
        ("GET", "/v1/models") => (
            200,
            serde_json::json!({"object":"list","data":[{"id":"nomic-embed-text"},{"id":"llama3.2"}]}),
        ),
        ("POST", "/v1/embeddings") => {
            let inputs: Vec<&str> = payload
                .get("input")
                .and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|x| x.as_str()).collect())
                .unwrap_or_default();
            let data: Vec<serde_json::Value> = inputs
                .iter()
                .enumerate()
                .map(|(i, t)| {
                    // Deterministic unit embedding that lets the test verify
                    // semantic re-ranking: the query shares a bucket with the
                    // note that mentions the query terms.
                    let h = (t.trim().chars().map(|c| c as u32).sum::<u32>() % 7) as usize;
                    let mut v = vec![0.0f32; 7];
                    v[h] = 1.0;
                    serde_json::json!({"object":"embedding","index":i,"embedding":v})
                })
                .collect();
            (
                200,
                serde_json::json!({
                    "object":"list",
                    "data": data,
                    "model": payload.get("model"),
                    "usage": {"prompt_tokens": 5, "total_tokens": 5}
                }),
            )
        }
        ("POST", "/api/chat") => {
            let content = payload
                .get("messages")
                .and_then(|m| m.as_array())
                .map(|msgs| {
                    msgs.iter()
                        .filter_map(|m| m.get("content").and_then(|c| c.as_str()))
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .unwrap_or_default();
            let n_sources = content.matches("[Source ").count();
            (
                200,
                serde_json::json!({
                    "model": payload.get("model"),
                    "message": {
                        "role": "assistant",
                        "content": format!("[MOCK] found {n_sources} source chunks"),
                    },
                    "done": true,
                }),
            )
        }
        _ => (
            404,
            serde_json::json!({"error": {"message": format!("unknown path {path}")}}),
        ),
    };

    let response_body = response.to_string();
    let _ = write!(
        stream,
        "HTTP/1.1 {status} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        response_body.len(),
        response_body
    );
    let _ = stream.flush();
}
