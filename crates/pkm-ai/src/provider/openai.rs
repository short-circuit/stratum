//! OpenAI provider implementation.

use super::stream::StreamBuffer;
use super::{ChatConfig, ChatDelta, ChatMessage, ChatResponse, LlmProvider, Role, TokenUsage};
use async_trait::async_trait;
use futures::stream::BoxStream;
use futures::StreamExt;
use pkm_core::{endpoint, PkmError, PkmResult};
use serde::{Deserialize, Serialize};
use std::time::Duration;

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
impl LlmProvider for OpenAIProvider {
    async fn chat(&self, messages: &[ChatMessage], config: &ChatConfig) -> PkmResult<ChatResponse> {
        endpoint::validate_endpoint_safe(&self.endpoint)?;
        let url = format!("{}/chat/completions", self.endpoint.trim_end_matches('/'));

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
        endpoint::validate_endpoint_safe(&self.endpoint)?;
        let url = format!("{}/chat/completions", self.endpoint.trim_end_matches('/'));

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
                            if data == "[DONE]" {
                                return Some(Ok(ChatDelta {
                                    content: String::new(),
                                    done: true,
                                }));
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

                            if let Ok(chunk) = serde_json::from_str::<OpenAIStreamChunk>(data) {
                                let content = chunk
                                    .choices
                                    .first()
                                    .and_then(|c| c.delta.content.clone())
                                    .unwrap_or_default();
                                Some(Ok(ChatDelta {
                                    content,
                                    done: false,
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
                Err(e) => vec![Err(PkmError::Ai(format!("OpenAI stream read error: {e}")))],
            };
            futures::stream::iter(items)
        });

        Ok(Box::pin(stream))
    }
}
