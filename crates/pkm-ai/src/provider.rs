use async_trait::async_trait;
use futures::stream::BoxStream;
use futures::StreamExt;
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

// ---------------------------------------------------------------------------
// Ollama provider
// ---------------------------------------------------------------------------

/// Provider backed by a local Ollama instance.
#[derive(Debug, Clone)]
pub struct OllamaProvider {
    endpoint: String,
    client: reqwest::Client,
}

impl OllamaProvider {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    async fn chat(&self, messages: &[ChatMessage], config: &ChatConfig) -> PkmResult<ChatResponse> {
        let url = format!("{}/api/chat", self.endpoint.trim_end_matches('/'));

        #[derive(Serialize)]
        struct OllamaRequest<'a> {
            model: &'a str,
            messages: Vec<OllamaMessage<'a>>,
            stream: bool,
            options: OllamaOptions,
        }

        #[derive(Serialize)]
        struct OllamaMessage<'a> {
            role: &'a str,
            content: &'a str,
        }

        #[derive(Serialize)]
        struct OllamaOptions {
            temperature: f32,
            num_predict: u32,
        }

        let mut ollama_messages: Vec<OllamaMessage> = messages
            .iter()
            .map(|m| OllamaMessage {
                role: match m.role {
                    Role::System => "system",
                    Role::User => "user",
                    Role::Assistant => "assistant",
                },
                content: &m.content,
            })
            .collect();

        // Prepend system prompt if provided
        if let Some(ref prompt) = config.system_prompt {
            ollama_messages.insert(
                0,
                OllamaMessage {
                    role: "system",
                    content: prompt,
                },
            );
        }

        let req = OllamaRequest {
            model: &config.model,
            messages: ollama_messages,
            stream: false,
            options: OllamaOptions {
                temperature: config.temperature,
                num_predict: config.max_tokens,
            },
        };

        let resp = self
            .client
            .post(&url)
            .json(&req)
            .send()
            .await
            .map_err(|e| PkmError::Ai(format!("Ollama request failed: {e}")))?;

        #[derive(Deserialize)]
        struct OllamaResponse {
            message: OllamaResponseMessage,
            #[serde(default)]
            _done: bool,
        }

        #[derive(Deserialize)]
        struct OllamaResponseMessage {
            content: String,
        }

        let body: OllamaResponse = resp
            .json()
            .await
            .map_err(|e| PkmError::Ai(format!("Ollama parse error: {e}")))?;

        Ok(ChatResponse {
            content: body.message.content,
            usage: TokenUsage::default(),
        })
    }

    async fn stream_chat(
        &self,
        messages: &[ChatMessage],
        config: &ChatConfig,
    ) -> PkmResult<BoxStream<'static, PkmResult<ChatDelta>>> {
        let url = format!("{}/api/chat", self.endpoint.trim_end_matches('/'));

        #[derive(Serialize)]
        struct OllamaRequest<'a> {
            model: &'a str,
            messages: Vec<OllamaMessage<'a>>,
            stream: bool,
            options: OllamaOptions,
        }

        #[derive(Serialize)]
        struct OllamaMessage<'a> {
            role: &'a str,
            content: &'a str,
        }

        #[derive(Serialize)]
        struct OllamaOptions {
            temperature: f32,
            num_predict: u32,
        }

        let mut ollama_messages: Vec<OllamaMessage> = messages
            .iter()
            .map(|m| OllamaMessage {
                role: match m.role {
                    Role::System => "system",
                    Role::User => "user",
                    Role::Assistant => "assistant",
                },
                content: &m.content,
            })
            .collect();

        if let Some(ref prompt) = config.system_prompt {
            ollama_messages.insert(
                0,
                OllamaMessage {
                    role: "system",
                    content: prompt,
                },
            );
        }

        let req = OllamaRequest {
            model: &config.model,
            messages: ollama_messages,
            stream: true,
            options: OllamaOptions {
                temperature: config.temperature,
                num_predict: config.max_tokens,
            },
        };

        let response = self
            .client
            .post(&url)
            .json(&req)
            .send()
            .await
            .map_err(|e| PkmError::Ai(format!("Ollama stream request failed: {e}")))?;

        let stream = response.bytes_stream().map(|chunk_result| match chunk_result {
            Ok(bytes) => {
                // Ollama sends one JSON object per line when streaming
                let text = String::from_utf8_lossy(&bytes);
                #[derive(Deserialize)]
                struct OllamaStreamChunk {
                    message: Option<OllamaStreamMessage>,
            _done: bool,
                }
                #[derive(Deserialize)]
                struct OllamaStreamMessage {
                    content: String,
                }

                if let Ok(chunk) = serde_json::from_str::<OllamaStreamChunk>(&text) {
                    let content = chunk
                        .message
                        .map(|m| m.content)
                        .unwrap_or_default();
                    Ok(ChatDelta {
                        content,
                        done: chunk._done,
                    })
                } else {
                    Ok(ChatDelta {
                        content: String::new(),
                        done: false,
                    })
                }
            }
            Err(e) => Err(PkmError::Ai(format!("Ollama stream read error: {e}"))),
        });

        Ok(Box::pin(stream))
    }
}

// ---------------------------------------------------------------------------
// OpenAI provider
// ---------------------------------------------------------------------------

/// Provider backed by the OpenAI API (or any OpenAI-compatible endpoint).
#[derive(Debug, Clone)]
pub struct OpenAIProvider {
    endpoint: String,
    api_key: String,
    client: reqwest::Client,
}

impl OpenAIProvider {
    pub fn new(endpoint: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            api_key: api_key.into(),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl LlmProvider for OpenAIProvider {
    async fn chat(&self, messages: &[ChatMessage], config: &ChatConfig) -> PkmResult<ChatResponse> {
        let url = format!(
            "{}/chat/completions",
            self.endpoint.trim_end_matches('/')
        );

        #[derive(Serialize)]
        struct OpenAIMessage<'a> {
            role: &'a str,
            content: &'a str,
        }

        #[derive(Serialize)]
        struct OpenAIRequest<'a> {
            model: &'a str,
            messages: Vec<OpenAIMessage<'a>>,
            temperature: f32,
            max_tokens: u32,
            stream: bool,
        }

        let mut openai_messages: Vec<OpenAIMessage> = messages
            .iter()
            .map(|m| OpenAIMessage {
                role: match m.role {
                    Role::System => "system",
                    Role::User => "user",
                    Role::Assistant => "assistant",
                },
                content: &m.content,
            })
            .collect();

        if let Some(ref prompt) = config.system_prompt {
            openai_messages.insert(
                0,
                OpenAIMessage {
                    role: "system",
                    content: prompt,
                },
            );
        }

        let req = OpenAIRequest {
            model: &config.model,
            messages: openai_messages,
            temperature: config.temperature,
            max_tokens: config.max_tokens,
            stream: false,
        };

        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&req)
            .send()
            .await
            .map_err(|e| PkmError::Ai(format!("OpenAI request failed: {e}")))?;

        #[derive(Deserialize)]
        struct OpenAIResponse {
            choices: Vec<OpenAIChoice>,
            usage: Option<OpenAIUsage>,
        }

        #[derive(Deserialize)]
        struct OpenAIChoice {
            message: OpenAIMessageContent,
        }

        #[derive(Deserialize)]
        struct OpenAIMessageContent {
            content: Option<String>,
        }

        #[derive(Deserialize, Default)]
        struct OpenAIUsage {
            prompt_tokens: u32,
            completion_tokens: u32,
        }

        let body: OpenAIResponse = resp
            .json()
            .await
            .map_err(|e| PkmError::Ai(format!("OpenAI parse error: {e}")))?;

        let content = body
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .unwrap_or_default();

        let usage = body.usage.unwrap_or_default();

        Ok(ChatResponse {
            content,
            usage: TokenUsage {
                prompt_tokens: usage.prompt_tokens,
                completion_tokens: usage.completion_tokens,
            },
        })
    }

    async fn stream_chat(
        &self,
        messages: &[ChatMessage],
        config: &ChatConfig,
    ) -> PkmResult<BoxStream<'static, PkmResult<ChatDelta>>> {
        let url = format!(
            "{}/chat/completions",
            self.endpoint.trim_end_matches('/')
        );

        #[derive(Serialize)]
        struct OpenAIMessage<'a> {
            role: &'a str,
            content: &'a str,
        }

        #[derive(Serialize)]
        struct OpenAIRequest<'a> {
            model: &'a str,
            messages: Vec<OpenAIMessage<'a>>,
            temperature: f32,
            max_tokens: u32,
            stream: bool,
        }

        let mut openai_messages: Vec<OpenAIMessage> = messages
            .iter()
            .map(|m| OpenAIMessage {
                role: match m.role {
                    Role::System => "system",
                    Role::User => "user",
                    Role::Assistant => "assistant",
                },
                content: &m.content,
            })
            .collect();

        if let Some(ref prompt) = config.system_prompt {
            openai_messages.insert(
                0,
                OpenAIMessage {
                    role: "system",
                    content: prompt,
                },
            );
        }

        let req = OpenAIRequest {
            model: &config.model,
            messages: openai_messages,
            temperature: config.temperature,
            max_tokens: config.max_tokens,
            stream: true,
        };

        let response = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&req)
            .send()
            .await
            .map_err(|e| PkmError::Ai(format!("OpenAI stream request failed: {e}")))?;

        let stream = response.bytes_stream().map(|chunk_result| {
            chunk_result
                .map_err(|e| PkmError::Ai(format!("OpenAI stream read error: {e}")))
                .and_then(|bytes| {
                    let text = String::from_utf8_lossy(&bytes);
                    // SSE format: "data: {...}\n\n"
                    if let Some(data) = text.strip_prefix("data: ") {
                        let data = data.trim();
                        if data == "[DONE]" {
                            return Ok(ChatDelta {
                                content: String::new(),
                                done: true,
                            });
                        }
                        #[derive(Deserialize)]
                        struct OpenAIStreamChunk {
                            choices: Vec<OpenAIStreamChoice>,
                        }
                        #[derive(Deserialize)]
                        struct OpenAIStreamChoice {
                            delta: OpenAIStreamDelta,
                            #[allow(dead_code)]
                            finish_reason: Option<String>,
                        }
                        #[derive(Deserialize)]
                        struct OpenAIStreamDelta {
                            content: Option<String>,
                        }

                        if let Ok(chunk) =
                            serde_json::from_str::<OpenAIStreamChunk>(data)
                        {
                            let content = chunk
                                .choices
                                .first()
                                .and_then(|c| c.delta.content.clone())
                                .unwrap_or_default();
                            Ok(ChatDelta {
                                content,
                                done: false,
                            })
                        } else {
                            Ok(ChatDelta {
                                content: String::new(),
                                done: false,
                            })
                        }
                    } else {
                        Ok(ChatDelta {
                            content: String::new(),
                            done: false,
                        })
                    }
                })
        });

        Ok(Box::pin(stream))
    }
}

// ---------------------------------------------------------------------------
// Anthropic provider
// ---------------------------------------------------------------------------

/// Provider backed by the Anthropic API.
#[derive(Debug, Clone)]
pub struct AnthropicProvider {
    endpoint: String,
    api_key: String,
    client: reqwest::Client,
}

impl AnthropicProvider {
    pub fn new(endpoint: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            api_key: api_key.into(),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    async fn chat(&self, messages: &[ChatMessage], config: &ChatConfig) -> PkmResult<ChatResponse> {
        let url = format!("{}/v1/messages", self.endpoint.trim_end_matches('/'));

        #[derive(Serialize)]
        struct AnthropicMessage<'a> {
            role: &'a str,
            content: &'a str,
        }

        #[derive(Serialize)]
        struct AnthropicRequest<'a> {
            model: &'a str,
            messages: Vec<AnthropicMessage<'a>>,
            max_tokens: u32,
            system: Option<&'a str>,
            temperature: f32,
            stream: bool,
        }

        let anthropic_messages: Vec<AnthropicMessage> = messages
            .iter()
            .map(|m| AnthropicMessage {
                role: match m.role {
                    Role::System => "user", // Anthropic uses system via separate field
                    Role::User => "user",
                    Role::Assistant => "assistant",
                },
                content: &m.content,
            })
            .collect();

        let system = config.system_prompt.as_deref();

        let req = AnthropicRequest {
            model: &config.model,
            messages: anthropic_messages,
            max_tokens: config.max_tokens,
            system,
            temperature: config.temperature,
            stream: false,
        };

        let resp = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&req)
            .send()
            .await
            .map_err(|e| PkmError::Ai(format!("Anthropic request failed: {e}")))?;

        #[derive(Deserialize)]
        struct AnthropicResponse {
            content: Vec<AnthropicContentBlock>,
            usage: Option<AnthropicUsage>,
        }

        #[derive(Deserialize)]
        struct AnthropicContentBlock {
            #[allow(dead_code)]
            #[serde(rename = "type")]
            block_type: String,
            text: Option<String>,
        }

        #[derive(Deserialize)]
        struct AnthropicUsage {
            input_tokens: u32,
            output_tokens: u32,
        }

        let body: AnthropicResponse = resp
            .json()
            .await
            .map_err(|e| PkmError::Ai(format!("Anthropic parse error: {e}")))?;

        let content: String = body
            .content
            .iter()
            .filter_map(|b| b.text.clone())
            .collect::<Vec<_>>()
            .join("");

        let usage = body.usage.unwrap_or(AnthropicUsage {
            input_tokens: 0,
            output_tokens: 0,
        });

        Ok(ChatResponse {
            content,
            usage: TokenUsage {
                prompt_tokens: usage.input_tokens,
                completion_tokens: usage.output_tokens,
            },
        })
    }

    async fn stream_chat(
        &self,
        messages: &[ChatMessage],
        config: &ChatConfig,
    ) -> PkmResult<BoxStream<'static, PkmResult<ChatDelta>>> {
        let url = format!("{}/v1/messages", self.endpoint.trim_end_matches('/'));

        #[derive(Serialize)]
        struct AnthropicMessage<'a> {
            role: &'a str,
            content: &'a str,
        }

        #[derive(Serialize)]
        struct AnthropicRequest<'a> {
            model: &'a str,
            messages: Vec<AnthropicMessage<'a>>,
            max_tokens: u32,
            system: Option<&'a str>,
            temperature: f32,
            stream: bool,
        }

        let anthropic_messages: Vec<AnthropicMessage> = messages
            .iter()
            .map(|m| AnthropicMessage {
                role: match m.role {
                    Role::System => "user",
                    Role::User => "user",
                    Role::Assistant => "assistant",
                },
                content: &m.content,
            })
            .collect();

        let system = config.system_prompt.as_deref();

        let req = AnthropicRequest {
            model: &config.model,
            messages: anthropic_messages,
            max_tokens: config.max_tokens,
            system,
            temperature: config.temperature,
            stream: true,
        };

        let response = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&req)
            .send()
            .await
            .map_err(|e| PkmError::Ai(format!("Anthropic stream request failed: {e}")))?;

        let stream = response.bytes_stream().map(|chunk_result| {
            chunk_result
                .map_err(|e| PkmError::Ai(format!("Anthropic stream read error: {e}")))
                .and_then(|bytes| {
                    let text = String::from_utf8_lossy(&bytes);
                    // SSE format: "data: {...}\n\n"
                    if let Some(data) = text.strip_prefix("data: ") {
                        let data = data.trim();
                        #[derive(Deserialize)]
                        struct AnthropicStreamChunk {
                            #[serde(rename = "type")]
                            chunk_type: String,
                            delta: Option<AnthropicStreamDelta>,
                        }
                        #[derive(Deserialize)]
                        struct AnthropicStreamDelta {
                            text: Option<String>,
                        }

                        if let Ok(chunk) =
                            serde_json::from_str::<AnthropicStreamChunk>(data)
                        {
                            let is_done =
                                chunk.chunk_type == "message_stop";
                            let content = chunk
                                .delta
                                .and_then(|d| d.text)
                                .unwrap_or_default();
                            Ok(ChatDelta {
                                content,
                                done: is_done,
                            })
                        } else {
                            Ok(ChatDelta {
                                content: String::new(),
                                done: false,
                            })
                        }
                    } else {
                        Ok(ChatDelta {
                            content: String::new(),
                            done: false,
                        })
                    }
                })
        });

        Ok(Box::pin(stream))
    }
}

// ---------------------------------------------------------------------------
// Custom provider
// ---------------------------------------------------------------------------

/// A configurable custom provider that sends messages to an arbitrary endpoint.
#[derive(Debug, Clone)]
pub struct CustomProvider {
    endpoint: String,
    api_key: Option<String>,
    client: reqwest::Client,
}

impl CustomProvider {
    pub fn new(endpoint: impl Into<String>, api_key: Option<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            api_key,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl LlmProvider for CustomProvider {
    async fn chat(&self, messages: &[ChatMessage], config: &ChatConfig) -> PkmResult<ChatResponse> {
        let url = self.endpoint.trim_end_matches('/').to_string();

        #[derive(Serialize)]
        struct CustomMessage<'a> {
            role: &'a str,
            content: &'a str,
        }

        #[derive(Serialize)]
        struct CustomRequest<'a> {
            model: &'a str,
            messages: Vec<CustomMessage<'a>>,
            temperature: f32,
            max_tokens: u32,
        }

        let custom_messages: Vec<CustomMessage> = messages
            .iter()
            .map(|m| CustomMessage {
                role: match m.role {
                    Role::System => "system",
                    Role::User => "user",
                    Role::Assistant => "assistant",
                },
                content: &m.content,
            })
            .collect();

        let req = CustomRequest {
            model: &config.model,
            messages: custom_messages,
            temperature: config.temperature,
            max_tokens: config.max_tokens,
        };

        let mut request = self.client.post(&url).json(&req);
        if let Some(ref key) = self.api_key {
            request = request.bearer_auth(key);
        }

        let resp = request
            .send()
            .await
            .map_err(|e| PkmError::Ai(format!("Custom provider request failed: {e}")))?;

        // Try to parse as OpenAI-compatible response
        #[derive(Deserialize)]
        struct GenericResponse {
            choices: Option<Vec<GenericChoice>>,
            content: Option<String>,
            message: Option<GenericMessage>,
        }

        #[derive(Deserialize)]
        struct GenericChoice {
            message: GenericMessage,
        }

        #[derive(Deserialize)]
        struct GenericMessage {
            content: Option<String>,
        }

        let body: GenericResponse = resp
            .json()
            .await
            .map_err(|e| PkmError::Ai(format!("Custom provider parse error: {e}")))?;

        let content = body
            .content
            .or_else(|| {
                body.message
                    .and_then(|m| m.content)
            })
            .or_else(|| {
                body.choices
                    .and_then(|c| c.into_iter().next())
                    .and_then(|c| c.message.content)
            })
            .unwrap_or_default();

        Ok(ChatResponse {
            content,
            usage: TokenUsage::default(),
        })
    }

    async fn stream_chat(
        &self,
        _messages: &[ChatMessage],
        _config: &ChatConfig,
    ) -> PkmResult<BoxStream<'static, PkmResult<ChatDelta>>> {
        Err(PkmError::Unsupported(
            "Streaming not supported for custom provider".to_string(),
        ))
    }
}

// ---------------------------------------------------------------------------
// Provider factory
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
                AiProvider::Custom => "http://localhost:8080/v1".to_string(),
            });

        match config.provider {
            AiProvider::Ollama => Ok(Box::new(OllamaProvider::new(endpoint))),
            AiProvider::OpenAI => {
                let api_key = config
                    .api_key
                    .clone()
                    .ok_or_else(|| PkmError::Config("OpenAI requires an API key".to_string()))?;
                Ok(Box::new(OpenAIProvider::new(endpoint, api_key)))
            }
            AiProvider::Anthropic => {
                let api_key = config
                    .api_key
                    .clone()
                    .ok_or_else(|| PkmError::Config("Anthropic requires an API key".to_string()))?;
                Ok(Box::new(AnthropicProvider::new(endpoint, api_key)))
            }
            AiProvider::Custom => Ok(Box::new(CustomProvider::new(
                endpoint,
                config.api_key.clone(),
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::mock;

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
            .with(
                mockall::predicate::always(),
                mockall::predicate::always(),
            )
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
    }
}
