//! Tests for provider types and the provider factory.

use super::*;
use async_trait::async_trait;
use futures::stream::BoxStream;
use mockall::mock;
use pkm_core::PkmResult;

// ---- Mock LlmProvider for testing ----
mock! {
    pub LlmProviderMock {}
    #[async_trait]
    impl LlmProvider for LlmProviderMock {
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
#[tokio::test]
async fn test_mock_provider() {
    let mut mock = MockLlmProviderMock::new();
    mock.expect_chat()
        .with(mockall::predicate::always(), mockall::predicate::always())
        .returning(|_, _| {
            Ok(ChatResponse {
                content: "Hello from mock!".to_string(),
                usage: TokenUsage {
                    prompt_tokens: 10,
                    completion_tokens: 5,
                },
            })
        });
    let messages = vec![ChatMessage::user("test")];
    let config = ChatConfig::default();
    let resp = mock.chat(&messages, &config).await.unwrap();
    assert_eq!(resp.content, "Hello from mock!");
    assert_eq!(resp.usage.total(), 15);
}
#[test]
fn test_chat_message_builders() {
    let msg = ChatMessage::system("You are a helpful assistant.");
    assert_eq!(msg.role, Role::System);
    assert_eq!(msg.content, "You are a helpful assistant.");
    let msg = ChatMessage::user("Hello");
    assert_eq!(msg.role, Role::User);
    assert_eq!(msg.content, "Hello");
    let msg = ChatMessage::assistant("Hi there!");
    assert_eq!(msg.role, Role::Assistant);
}
#[test]
fn test_chat_config_builder() {
    let config = ChatConfig::new("gpt-4")
        .with_temperature(0.5)
        .with_max_tokens(4096)
        .with_system_prompt("Be concise.");
    assert_eq!(config.model, "gpt-4");
    assert!((config.temperature - 0.5).abs() < f32::EPSILON);
    assert_eq!(config.max_tokens, 4096);
    assert_eq!(config.system_prompt.unwrap(), "Be concise.");
}
#[test]
fn test_token_usage() {
    let usage = TokenUsage {
        prompt_tokens: 100,
        completion_tokens: 50,
    };
    assert_eq!(usage.total(), 150);
    assert_eq!(usage.prompt_tokens, 100);
    assert_eq!(usage.completion_tokens, 50);
}
#[test]
fn test_role_display() {
    assert_eq!(Role::System.to_string(), "system");
    assert_eq!(Role::User.to_string(), "user");
    assert_eq!(Role::Assistant.to_string(), "assistant");
}
#[test]
fn test_factory_creates_ollama() {
    let config = AiConfig {
        provider: AiProvider::Ollama,
        endpoint: Some("http://localhost:11434".to_string()),
        ..Default::default()
    };
    let provider = ProviderFactory::create(&config);
    assert!(provider.is_ok(), "Ollama provider should be created");
}
#[test]
fn test_factory_creates_openai() {
    let config = AiConfig {
        provider: AiProvider::OpenAI,
        endpoint: Some("https://api.openai.com/v1".to_string()),
        api_key: Some("sk-test123".to_string()),
        ..Default::default()
    };
    let provider = ProviderFactory::create(&config);
    assert!(provider.is_ok(), "OpenAI provider should be created");
}
#[test]
fn test_factory_creates_anthropic() {
    let config = AiConfig {
        provider: AiProvider::Anthropic,
        endpoint: Some("https://api.anthropic.com".to_string()),
        api_key: Some("sk-ant-test123".to_string()),
        ..Default::default()
    };
    let provider = ProviderFactory::create(&config);
    assert!(provider.is_ok(), "Anthropic provider should be created");
}
#[test]
fn test_factory_creates_custom() {
    let config = AiConfig {
        provider: AiProvider::Custom,
        endpoint: Some("http://localhost:8080/v1".to_string()),
        api_key: Some("custom-key".to_string()),
        ..Default::default()
    };
    let provider = ProviderFactory::create(&config);
    assert!(provider.is_ok(), "Custom provider should be created");
}
#[test]
fn test_factory_missing_api_key_openai() {
    let config = AiConfig {
        provider: AiProvider::OpenAI,
        endpoint: Some("https://api.openai.com/v1".to_string()),
        api_key: None,
        ..Default::default()
    };
    let result = ProviderFactory::create(&config);
    assert!(result.is_err());
    let err = match result {
        Err(e) => e,
        _ => unreachable!(),
    };
    assert!(err.to_string().contains("API key"));
}
#[test]
fn test_factory_missing_api_key_anthropic() {
    let config = AiConfig {
        provider: AiProvider::Anthropic,
        endpoint: Some("https://api.anthropic.com".to_string()),
        api_key: None,
        ..Default::default()
    };
    let result = ProviderFactory::create(&config);
    assert!(result.is_err());
}
#[test]
fn test_factory_default_endpoints() {
    // Ollama with no endpoint should default
    let config = AiConfig {
        provider: AiProvider::Ollama,
        endpoint: None,
        ..Default::default()
    };
    assert!(ProviderFactory::create(&config).is_ok());
    // Custom with no endpoint should default to localhost:8080
    let config = AiConfig {
        provider: AiProvider::Custom,
        endpoint: None,
        api_key: None,
        ..Default::default()
    };
    assert!(ProviderFactory::create(&config).is_ok());
    // CustomOpenAI with no endpoint should default to localhost:8080/v1
    let config = AiConfig {
        provider: AiProvider::CustomOpenAI,
        endpoint: None,
        api_key: Some("sk-test".to_string()),
        ..Default::default()
    };
    assert!(ProviderFactory::create(&config).is_ok());
    // CustomAnthropic with no endpoint should default to api.anthropic.com
    let config = AiConfig {
        provider: AiProvider::CustomAnthropic,
        endpoint: None,
        api_key: Some("sk-ant-test".to_string()),
        ..Default::default()
    };
    assert!(ProviderFactory::create(&config).is_ok());
}
#[test]
fn test_factory_creates_custom_openai() {
    let config = AiConfig {
        provider: AiProvider::CustomOpenAI,
        endpoint: Some("http://localhost:8080/v1".to_string()),
        api_key: Some("sk-custom123".to_string()),
        ..Default::default()
    };
    let provider = ProviderFactory::create(&config);
    assert!(provider.is_ok(), "CustomOpenAI provider should be created");
}
#[test]
fn test_factory_creates_custom_anthropic() {
    let config = AiConfig {
        provider: AiProvider::CustomAnthropic,
        endpoint: Some("https://custom.anthropic.com".to_string()),
        api_key: Some("sk-ant-custom123".to_string()),
        ..Default::default()
    };
    let provider = ProviderFactory::create(&config);
    assert!(
        provider.is_ok(),
        "CustomAnthropic provider should be created"
    );
}
#[test]
fn test_factory_missing_api_key_custom_openai() {
    let config = AiConfig {
        provider: AiProvider::CustomOpenAI,
        endpoint: Some("http://localhost:8080/v1".to_string()),
        api_key: None,
        ..Default::default()
    };
    let result = ProviderFactory::create(&config);
    assert!(result.is_err());
    let err = match result {
        Err(e) => e,
        _ => unreachable!(),
    };
    assert!(err.to_string().contains("API key"));
}
#[test]
fn test_factory_missing_api_key_custom_anthropic() {
    let config = AiConfig {
        provider: AiProvider::CustomAnthropic,
        endpoint: Some("https://custom.anthropic.com".to_string()),
        api_key: None,
        ..Default::default()
    };
    let result = ProviderFactory::create(&config);
    assert!(result.is_err());
    let err = match result {
        Err(e) => e,
        _ => unreachable!(),
    };
    assert!(err.to_string().contains("API key"));
}
#[test]
fn test_factory_backward_compat_custom() {
    // Old Custom variant still works without api_key
    let config = AiConfig {
        provider: AiProvider::Custom,
        endpoint: None,
        api_key: None,
        ..Default::default()
    };
    assert!(ProviderFactory::create(&config).is_ok());
}
