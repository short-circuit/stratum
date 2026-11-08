//! Headless tests for the settings service commands:
//! `get_settings`, `save_settings`, `save_graph_settings`, `fetch_models`,
//! `get_onboarding_state`, `mark_onboarding_done`, `set_onboarding_pending`.

mod common;

use common::create_test_vault;

#[tokio::test]
async fn get_settings_returns_defaults_without_a_config_file() {
    let vault = create_test_vault();

    let settings = vault.service.get_settings().await.unwrap();

    assert_eq!(settings.vault_path, vault.vault_path.to_string_lossy());
    assert_eq!(settings.ai.provider, "ollama");
    assert_eq!(settings.sync.mode, "manual");
    assert_eq!(settings.sync.backup.mode, "folder");
    assert_eq!(settings.layout.pages_view, "flat");
}

#[tokio::test]
async fn save_settings_persists_the_dto_to_config() {
    let vault = create_test_vault();
    let mut settings = vault.service.get_settings().await.unwrap();
    settings.locale = "de".into();
    settings.theme.dark_mode = true;
    settings.theme.primary_color = "#123456".into();
    settings.ai.provider = "openai".into();
    settings.ai.model = "gpt-4o".into();
    settings.sync.mode = "auto_sync".into();
    settings.sync.branch = "develop".into();

    let saved = vault.service.save_settings(settings).await.unwrap();

    assert_eq!(saved.locale, "de");
    assert!(saved.theme.dark_mode);
    assert_eq!(saved.sync.branch, "develop");

    let on_disk = pkm_core::Config::load(vault.vault_path.join(".pkm/config.toml")).unwrap();
    assert_eq!(on_disk.locale, "de");
    assert!(on_disk.theme.dark_mode);
    assert_eq!(on_disk.ai.model, "gpt-4o");
    assert_eq!(on_disk.sync.branch, "develop");
    assert_eq!(on_disk.sync.mode, pkm_core::SyncMode::AutoSync);
}

#[tokio::test]
async fn save_settings_preserves_a_masked_api_key() {
    let vault = create_test_vault();
    let mut config = vault.config();
    config.ai.provider = pkm_core::AiProvider::CustomOpenAI;
    config.ai.api_key = Some("sk-secret-key-1234".into());
    vault.save_config(&config);

    let settings = vault.service.get_settings().await.unwrap();
    assert_eq!(settings.ai.api_key.as_deref(), Some("sk-****1234"));

    vault.service.save_settings(settings).await.unwrap();

    let on_disk = pkm_core::Config::load(vault.vault_path.join(".pkm/config.toml")).unwrap();
    assert_eq!(
        on_disk.ai.api_key.as_deref(),
        Some("sk-secret-key-1234"),
        "the masked placeholder must not overwrite the stored key"
    );
}

#[tokio::test]
async fn save_graph_settings_persists_only_the_graph_section() {
    let vault = create_test_vault();
    let settings = vault.service.get_settings().await.unwrap();
    let mut graph = settings.graph;
    graph.show_orphaned = true;
    graph.charge_strength = -321.0;
    graph.link_distance = 77.0;
    graph.use_3d = true;

    vault.service.save_graph_settings(graph).await.unwrap();

    let on_disk = pkm_core::Config::load(vault.vault_path.join(".pkm/config.toml")).unwrap();
    assert!(on_disk.graph.show_orphaned);
    assert_eq!(on_disk.graph.charge_strength, -321.0);
    assert_eq!(on_disk.graph.link_distance, 77.0);
    assert!(on_disk.graph.use_3d);
}

#[tokio::test]
async fn fetch_models_returns_provider_model_ids() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, ResponseTemplate};

    let vault = create_test_vault();
    let server = wiremock::MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [
                { "id": "gpt-4o", "object": "model" },
                { "id": "gpt-4o-mini", "object": "model" }
            ]
        })))
        .mount(&server)
        .await;

    let mut config = vault.config();
    config.ai.provider = pkm_core::AiProvider::CustomOpenAI;
    config.ai.endpoint = Some(format!("{}/v1", server.uri()));
    config.ai.api_key = Some("sk-test".into());
    config.ai.model = "gpt-4o".into();
    vault.save_config(&config);

    let models = vault.service.fetch_models().await.unwrap();

    assert_eq!(models, vec!["gpt-4o", "gpt-4o-mini"]);
}

#[tokio::test]
async fn fetch_models_without_config_fails_cleanly() {
    let vault = create_test_vault();

    let err = vault.service.fetch_models().await.unwrap_err();

    assert!(err.contains("config"), "error must be actionable: {err}");
}

#[tokio::test]
async fn onboarding_state_roundtrip() {
    let vault = create_test_vault();

    // Brand-new empty vault: onboarding pending.
    assert!(vault.service.get_onboarding_state().await.unwrap());

    vault.service.mark_onboarding_done().await.unwrap();
    assert!(!vault.service.get_onboarding_state().await.unwrap());

    vault.service.set_onboarding_pending(true).await.unwrap();
    assert!(vault.service.get_onboarding_state().await.unwrap());

    let on_disk = pkm_core::Config::load(vault.vault_path.join(".pkm/config.toml")).unwrap();
    assert!(on_disk.onboarding_pending);
}

#[tokio::test]
async fn onboarding_state_is_false_for_a_vault_with_pages_and_no_config() {
    let vault = create_test_vault();
    vault.write_file("pages/existing.md", "# existing\n");
    vault
        .service
        .save_blocks(
            "pages/existing.md".into(),
            vec![common::block_dto("existing")],
            None,
        )
        .await
        .unwrap();

    assert!(!vault.service.get_onboarding_state().await.unwrap());
}
