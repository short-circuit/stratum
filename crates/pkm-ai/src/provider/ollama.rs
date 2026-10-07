//! Ollama provider implementation.

use super::stream::StreamBuffer;
use super::{ChatConfig, ChatDelta, ChatMessage, ChatResponse, LlmProvider, Role, TokenUsage};
use async_trait::async_trait;
use futures::stream::BoxStream;
use futures::StreamExt;
use pkm_core::{endpoint, PkmError, PkmResult};
use serde::{Deserialize, Serialize};
use std::time::Duration;

// ---------------------------------------------------------------------------

/// Provider backed by a local Ollama instance.
#[derive(Debug, Clone)]
pub struct OllamaProvider {
    endpoint: String,
    client: reqwest::Client,
}

impl OllamaProvider {
    pub fn new(endpoint: impl Into<String>) -> PkmResult<Self> {
        Ok(Self {
            endpoint: endpoint.into(),
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .map_err(|e| PkmError::Ai(format!("Failed to create HTTP client: {e}")))?,
        })
    }
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    async fn chat(&self, messages: &[ChatMessage], config: &ChatConfig) -> PkmResult<ChatResponse> {
        endpoint::validate_endpoint_safe(&self.endpoint)?;
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
        endpoint::validate_endpoint_safe(&self.endpoint)?;
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

        let mut buffer = StreamBuffer::new();
        let stream = response.bytes_stream().flat_map(move |chunk_result| {
            let items: Vec<PkmResult<ChatDelta>> = match chunk_result {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    let lines = buffer.feed(&text, "\n");

                    #[derive(Deserialize)]
                    struct OllamaStreamChunk {
                        message: Option<OllamaStreamMessage>,
                        _done: bool,
                    }
                    #[derive(Deserialize)]
                    struct OllamaStreamMessage {
                        content: String,
                    }

                    lines
                        .into_iter()
                        .map(|line| {
                            if let Ok(chunk) = serde_json::from_str::<OllamaStreamChunk>(&line) {
                                let content = chunk.message.map(|m| m.content).unwrap_or_default();
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
                        })
                        .collect()
                }
                Err(e) => vec![Err(PkmError::Ai(format!("Ollama stream read error: {e}")))],
            };
            futures::stream::iter(items)
        });

        Ok(Box::pin(stream))
    }
}
