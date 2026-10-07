//! LLM provider abstraction: shared chat types, the `LlmProvider` trait,
//! and provider-specific implementations (Ollama, OpenAI-compatible,
//! Anthropic, and a generic custom provider).

use async_trait::async_trait;
use futures::stream::BoxStream;
use pkm_core::{AiConfig, AiProvider, PkmError, PkmResult};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Role of a chat message sender.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Role::System => write!(f, "system"),
            Role::User => write!(f, "user"),
            Role::Assistant => write!(f, "assistant"),
        }
    }
}

/// A single chat message in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: Role,
    pub content: String,
}

impl ChatMessage {
    pub fn new(role: Role, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
        }
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self::new(Role::System, content)
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self::new(Role::User, content)
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new(Role::Assistant, content)
    }
}

/// Configuration for a chat completion request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatConfig {
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u32,
    pub system_prompt: Option<String>,
}

impl Default for ChatConfig {
    fn default() -> Self {
        Self {
            model: "llama3.2".to_string(),
            temperature: 0.7,
            max_tokens: 2048,
            system_prompt: None,
        }
    }
}

impl ChatConfig {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            ..Default::default()
        }
    }

    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature;
        self
    }

    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }
}

/// Token usage statistics for a completion.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
}

impl TokenUsage {
    pub fn total(&self) -> u32 {
        self.prompt_tokens + self.completion_tokens
    }
}

/// Response from a chat completion request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub content: String,
    pub usage: TokenUsage,
}

/// A delta chunk in a streaming chat response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatDelta {
    pub content: String,
    pub done: bool,
}

/// Abstraction over different LLM providers.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Send a chat completion request and receive the full response.
    async fn chat(&self, messages: &[ChatMessage], config: &ChatConfig) -> PkmResult<ChatResponse>;

    /// Send a chat completion request and receive a stream of response deltas.
    async fn stream_chat(
        &self,
        messages: &[ChatMessage],
        config: &ChatConfig,
    ) -> PkmResult<BoxStream<'static, PkmResult<ChatDelta>>>;
}

mod anthropic;
mod custom;
mod ollama;
mod openai;
mod stream;
#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------

/// Create an LLM provider from an `AiConfig`.
pub struct ProviderFactory;

impl ProviderFactory {
    pub fn create(config: &AiConfig) -> PkmResult<Box<dyn LlmProvider>> {
        let endpoint = config
            .endpoint
            .clone()
            .unwrap_or_else(|| match config.provider {
                AiProvider::Ollama => "http://localhost:11434".to_string(),
                AiProvider::OpenAI => "https://api.openai.com/v1".to_string(),
                AiProvider::Anthropic => "https://api.anthropic.com".to_string(),
                AiProvider::Custom => "http://localhost:8080/v1/chat/completions".to_string(),
                AiProvider::CustomOpenAI => "http://localhost:8080/v1".to_string(),
                AiProvider::CustomAnthropic => "https://api.anthropic.com".to_string(),
                AiProvider::Google => {
                    "https://generativelanguage.googleapis.com/v1beta".to_string()
                }
                AiProvider::Zai => "https://api.z.ai".to_string(),
            });

        match config.provider {
            AiProvider::Ollama => Ok(Box::new(OllamaProvider::new(endpoint)?)),
            AiProvider::OpenAI => {
                let api_key = config
                    .effective_api_key()
                    .ok_or_else(|| PkmError::Config("OpenAI requires an API key".to_string()))?;
                Ok(Box::new(OpenAIProvider::new(endpoint, api_key)?))
            }
            AiProvider::Anthropic => {
                let api_key = config
                    .effective_api_key()
                    .ok_or_else(|| PkmError::Config("Anthropic requires an API key".to_string()))?;
                Ok(Box::new(AnthropicProvider::new(endpoint, api_key)?))
            }
            AiProvider::Custom => Ok(Box::new(CustomProvider::new(
                endpoint,
                config.effective_api_key(),
            )?)),
            AiProvider::CustomOpenAI => {
                let api_key = config.effective_api_key().ok_or_else(|| {
                    PkmError::Config("CustomOpenAI requires an API key".to_string())
                })?;
                Ok(Box::new(OpenAIProvider::new(endpoint, api_key)?))
            }
            AiProvider::CustomAnthropic => {
                let api_key = config.effective_api_key().ok_or_else(|| {
                    PkmError::Config("CustomAnthropic requires an API key".to_string())
                })?;
                Ok(Box::new(AnthropicProvider::new(endpoint, api_key)?))
            }
            AiProvider::Google | AiProvider::Zai => Ok(Box::new(CustomProvider::new(
                endpoint,
                config.effective_api_key(),
            )?)),
        }
    }
}

pub use anthropic::AnthropicProvider;
pub use custom::CustomProvider;
pub use ollama::OllamaProvider;
pub use openai::OpenAIProvider;
