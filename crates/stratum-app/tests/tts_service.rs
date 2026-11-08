//! Headless tests for the text-to-speech service commands:
//! `tts_synthesize` and `tts_speak`.

mod common;

use common::create_test_vault;

/// Seed an OpenAI-compatible config whose AI endpoint also serves TTS.
fn seed_tts_config(vault: &common::TestVault, endpoint: &str) {
    let mut config = vault.config();
    config.ai.provider = pkm_core::AiProvider::CustomOpenAI;
    config.ai.endpoint = Some(endpoint.to_string());
    config.ai.api_key = Some("sk-test".into());
    config.ai.model = "tts-1".into();
    // Empty TTS endpoint → falls back to the AI endpoint.
    config.tts.endpoint = String::new();
    config.tts.format = "mp3".into();
    vault.save_config(&config);
}

async fn mount_speech(server: &wiremock::MockServer) {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, ResponseTemplate};

    Mock::given(method("POST"))
        .and(path("/v1/audio/speech"))
        .respond_with(
            ResponseTemplate::new(200).set_body_bytes(b"ID3\x00\x00fake-mp3-bytes".to_vec()),
        )
        .mount(server)
        .await;
}

#[tokio::test]
async fn tts_synthesize_returns_audio_from_the_configured_endpoint() {
    let vault = create_test_vault();
    let server = wiremock::MockServer::start().await;
    mount_speech(&server).await;
    seed_tts_config(&vault, &format!("{}/v1", server.uri()));

    let result = vault
        .service
        .tts_synthesize("Hello world".into())
        .await
        .unwrap();

    assert_eq!(result.mime, "audio/mpeg");
    assert_eq!(result.model, "tts-1");
    assert_eq!(result.byte_len, b"ID3\x00\x00fake-mp3-bytes".len());

    use base64::Engine;
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(&result.audio_b64)
        .unwrap();
    assert_eq!(decoded, b"ID3\x00\x00fake-mp3-bytes");
}

#[tokio::test]
async fn tts_speak_returns_audio_from_the_configured_endpoint() {
    let vault = create_test_vault();
    let server = wiremock::MockServer::start().await;
    mount_speech(&server).await;
    seed_tts_config(&vault, &format!("{}/v1", server.uri()));

    let result = vault
        .service
        .tts_speak("Read this aloud".into())
        .await
        .unwrap();

    assert_eq!(result.mime, "audio/mpeg");
    assert!(result.byte_len > 0);
}

#[tokio::test]
async fn tts_without_config_fails_with_an_actionable_error() {
    let vault = create_test_vault();

    let err = vault.service.tts_synthesize("x".into()).await.unwrap_err();

    assert!(
        err.contains("configure") || err.contains("config"),
        "error must be actionable: {err}"
    );
}

#[tokio::test]
async fn tts_surfaces_provider_http_error() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, ResponseTemplate};

    let vault = create_test_vault();
    let server = wiremock::MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/audio/speech"))
        .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
        .mount(&server)
        .await;
    seed_tts_config(&vault, &format!("{}/v1", server.uri()));

    let err = vault.service.tts_speak("hello".into()).await.unwrap_err();

    assert!(err.contains("TTS failed"), "error must be prefixed: {err}");
    assert!(err.contains("500"), "error must include the status: {err}");
}
