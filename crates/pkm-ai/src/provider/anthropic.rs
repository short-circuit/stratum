//! Anthropic provider implementation.

use super::stream::StreamBuffer;
use super::{ChatConfig, ChatDelta, ChatMessage, ChatResponse, LlmProvider, Role, TokenUsage};
use async_trait::async_trait;
use futures::stream::BoxStream;
use futures::StreamExt;
use pkm_core::{endpoint, PkmError, PkmResult};
use serde::{Deserialize, Serialize};
use std::time::Duration;

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
    pub fn new(endpoint: impl Into<String>, api_key: impl Into<String>) -> PkmResult<Self> {
        Ok(Self {
            endpoint: endpoint.into(),
            api_key: api_key.into(),
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .map_err(|e| PkmError::Ai(format!("Failed to create HTTP client: {e}")))?,
        })
    }
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    async fn chat(&self, messages: &[ChatMessage], config: &ChatConfig) -> PkmResult<ChatResponse> {
        endpoint::validate_endpoint_safe(&self.endpoint)?;
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
        endpoint::validate_endpoint_safe(&self.endpoint)?;
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

        let mut buffer = StreamBuffer::new();
        let stream = response.bytes_stream().flat_map(move |chunk_result| {
            let items: Vec<PkmResult<ChatDelta>> = match chunk_result {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    let events = buffer.feed(&text, "\n\n");

                    events
                        .into_iter()
                        .filter_map(|event| {
                            // SSE format: "data: {...}"
                            let data = event.strip_prefix("data: ")?.trim();
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

                            if let Ok(chunk) = serde_json::from_str::<AnthropicStreamChunk>(data) {
                                let is_done = chunk.chunk_type == "message_stop";
                                let content = chunk.delta.and_then(|d| d.text).unwrap_or_default();
                                Some(Ok(ChatDelta {
                                    content,
                                    done: is_done,
                                }))
                            } else {
                                Some(Ok(ChatDelta {
                                    content: String::new(),
                                    done: false,
                                }))
                            }
                        })
                        .collect()
                }
                Err(e) => vec![Err(PkmError::Ai(format!(
                    "Anthropic stream read error: {e}"
                )))],
            };
            futures::stream::iter(items)
        });

        Ok(Box::pin(stream))
    }
}
