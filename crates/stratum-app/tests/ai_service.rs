//! Headless tests for the AI service commands:
//! `ai_transform_block`, `generate_mermaid`, `ai_research`,
//! `ai_interlink_notes`, `ai_rag_query`, `discover_local_ai`,
//! `ping_search_instance`.

mod common;

use common::{create_test_vault, mount_llm_research_backend, seed_ai_config, RecordingSink};
use stratum_app::ai::AiAction;

#[tokio::test]
async fn ai_transform_returns_provider_output() {
    let vault = create_test_vault();
    let server = mount_llm_research_backend().await;
    seed_ai_config(&vault, &format!("{}/v1", server.uri()), &server.uri());

    let result = vault
        .service
        .ai_transform_block("Rust is memory safe.".into(), AiAction::Rewrite, None)
        .await
        .unwrap();

    assert!(
        result.content.contains("AI output"),
        "transform must return provider output: {}",
        result.content
    );
}

#[tokio::test]
async fn ai_transform_without_config_fails_with_actionable_error() {
    let vault = create_test_vault();

    let err = vault
        .service
        .ai_transform_block("x".into(), AiAction::Summarize, None)
        .await
        .unwrap_err();

    assert!(
        err.contains("configure") || err.contains("config"),
        "error must be actionable: {err}"
    );
}

#[tokio::test]
async fn generate_mermaid_returns_diagram_from_provider() {
    let vault = create_test_vault();
    let server = mount_llm_research_backend().await;
    seed_ai_config(&vault, &format!("{}/v1", server.uri()), &server.uri());

    let result = vault
        .service
        .generate_mermaid("a flowchart of review".into())
        .await
        .unwrap();

    assert!(result.content.contains("AI output"));
}

#[tokio::test]
async fn ai_research_runs_the_full_pipeline_against_searxng() {
    let vault = create_test_vault();
    let server = mount_llm_research_backend().await;
    seed_ai_config(&vault, &format!("{}/v1", server.uri()), &server.uri());

    let result = vault
        .service
        .ai_research("Rust memory safety".into())
        .await
        .unwrap();

    assert!(
        result.findings.contains("AI output"),
        "findings: {}",
        result.findings
    );
    assert_eq!(result.sources.len(), 1);
    assert_eq!(result.sources[0].title, "Rust Programming Language");
    assert!(result.sources[0].url.ends_with("/page1"));
}

#[tokio::test]
async fn ai_research_without_searxng_endpoint_fails_cleanly() {
    let vault = create_test_vault();
    let mut config = vault.config();
    config.research.searxng_endpoint = String::new();
    vault.save_config(&config);

    let err = vault
        .service
        .ai_research("anything".into())
        .await
        .unwrap_err();

    assert!(
        err.contains("SearXNG"),
        "error must explain the missing endpoint: {err}"
    );
}

#[tokio::test]
async fn ai_interlink_finds_related_notes_and_returns_llm_output() {
    let vault = create_test_vault();
    let server = mount_llm_research_backend().await;
    seed_ai_config(&vault, &format!("{}/v1", server.uri()), &server.uri());

    vault
        .service
        .save_blocks(
            "pages/rust-dev.md".into(),
            vec![common::block_dto(
                "memory safety in systems programming rust",
            )],
            Some("Rust Dev".into()),
        )
        .await
        .unwrap();
    vault
        .service
        .save_blocks(
            "pages/current.md".into(),
            vec![common::block_dto("a scratch note under edit")],
            Some("Current".into()),
        )
        .await
        .unwrap();

    let sink = RecordingSink::new();
    vault.service.rebuild_search_index(&sink).await.unwrap();

    let result = vault
        .service
        .ai_interlink_notes(
            "memory safety in systems programming".into(),
            Some("pages/current.md".into()),
        )
        .await
        .unwrap();

    assert!(
        result.content.contains("AI output"),
        "interlink must return the LLM rewrite: {}",
        result.content
    );
}

#[tokio::test]
async fn ai_interlink_returns_original_text_when_nothing_is_related() {
    let vault = create_test_vault();
    let server = mount_llm_research_backend().await;
    seed_ai_config(&vault, &format!("{}/v1", server.uri()), &server.uri());

    vault
        .service
        .save_blocks(
            "pages/current.md".into(),
            vec![common::block_dto("unique zebra quokka")],
            Some("Current".into()),
        )
        .await
        .unwrap();
    let sink = RecordingSink::new();
    vault.service.rebuild_search_index(&sink).await.unwrap();

    let text = "nothing in the vault matches this";
    let result = vault
        .service
        .ai_interlink_notes(text.into(), Some("pages/current.md".into()))
        .await
        .unwrap();

    assert_eq!(result.content, text);
}

#[tokio::test]
async fn ai_rag_query_answers_from_the_vault_index() {
    let vault = create_test_vault();
    let server = mount_llm_research_backend().await;
    seed_ai_config(&vault, &format!("{}/v1", server.uri()), &server.uri());

    vault
        .service
        .save_blocks(
            "pages/rust.md".into(),
            vec![common::block_dto(
                "rust guarantees memory safety and prevents data races",
            )],
            Some("Rust".into()),
        )
        .await
        .unwrap();

    let result = vault
        .service
        .ai_rag_query("What does Rust guarantee?".into())
        .await
        .unwrap();

    assert!(
        result.answer.contains("AI output"),
        "RAG must return the LLM answer: {}",
        result.answer
    );
    // The embedding re-rank may fail against the mock; the answer must still
    // come back (failures degrade to raw search scores).
    assert!(result.total_tokens > 0, "token usage must be reported");
}

#[tokio::test]
async fn discover_local_ai_reports_a_configured_extra_host() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, ResponseTemplate};

    let vault = create_test_vault();
    let server = wiremock::MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{ "id": "gpt-4o" }]
        })))
        .mount(&server)
        .await;

    let mut config = vault.config();
    config.ai.extra_local_hosts = vec![server.uri()];
    vault.save_config(&config);

    let endpoints = vault.service.discover_local_ai().await.unwrap();

    let found = endpoints
        .iter()
        .find(|e| e.status == "available" && e.models.iter().any(|m| m == "gpt-4o"));
    assert!(
        found.is_some(),
        "the configured extra host must be discovered: {endpoints:?}"
    );
    assert_eq!(found.unwrap().kind, "openai_compatible");
}

#[tokio::test]
async fn ping_search_instance_reports_reachable_searxng() {
    let server = mount_llm_research_backend().await;

    let result = vault_ping(&server.uri()).await;

    assert!(result.ok, "reachable instance must ping ok: {result:?}");
    assert!(result.error.is_none());
}

#[tokio::test]
async fn ping_search_instance_reports_unreachable_instance() {
    let result = vault_ping("http://127.0.0.1:9").await;

    assert!(!result.ok);
    assert!(result.error.is_some());
}

/// `ping_search_instance` does not need vault state; call it through a fresh
/// service to keep the test independent of a vault fixture.
async fn vault_ping(url: &str) -> pkm_core::SearchPingResult {
    let service = stratum_app::StratumService::new(std::path::PathBuf::from("/nonexistent"));
    service.ping_search_instance(url.to_string()).await.unwrap()
}
