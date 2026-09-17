//! Custom provider implementation.

use super::{ChatConfig, ChatDelta, ChatMessage, ChatResponse, LlmProvider, Role, TokenUsage};
use async_trait::async_trait;
use futures::stream::BoxStream;
use pkm_core::{endpoint, PkmError, PkmResult};
use serde::{Deserialize, Serialize};
use std::time::Duration;

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
    pub fn new(endpoint: impl Into<String>, api_key: Option<String>) -> PkmResult<Self> {
        Ok(Self {
            endpoint: endpoint.into(),
            api_key,
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .map_err(|e| PkmError::Ai(format!("Failed to create HTTP client: {e}")))?,
        })
    }
}

#[async_trait]
impl LlmProvider for CustomProvider {
    async fn chat(&self, messages: &[ChatMessage], config: &ChatConfig) -> PkmResult<ChatResponse> {
        endpoint::validate_endpoint_safe(&self.endpoint)?;
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

        let mut custom_messages: Vec<CustomMessage> = messages
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

        if let Some(ref prompt) = config.system_prompt {
            custom_messages.insert(
                0,
                CustomMessage {
                    role: "system",
                    content: prompt,
                },
            );
        }

        let req = CustomRequest {
            model: &config.model,
            messages: custom_messages,
            temperature: config.temperature,
            max_tokens: config.max_tokens,
        };

        let do_request = |target_url: &str| {
            let mut r = self.client.post(target_url).json(&req);
            if let Some(ref key) = self.api_key {
                r = r.bearer_auth(key);
            }
            r
        };

        let mut resp = do_request(&url)
            .send()
            .await
            .map_err(|e| PkmError::Ai(format!("Custom provider request failed: {e}")))?;

        // Retry with /chat/completions if base URL returned 405
        if resp.status() == 405 && !url.ends_with("/chat/completions") {
            let chat_url = format!("{}/chat/completions", url.trim_end_matches('/'));
            eprintln!("[CustomProvider] 405 on {}, retrying {}", url, chat_url);
            resp = do_request(&chat_url)
                .send()
                .await
                .map_err(|e| PkmError::Ai(format!("Custom provider retry failed: {e}")))?;
        }

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(PkmError::Ai(format!(
                "Custom provider returned {}: {}",
                status,
                body.chars().take(200).collect::<String>()
            )));
        }

        // Try to parse response – supports multiple common API shapes
        #[derive(Deserialize)]
        struct GenericResponse {
            choices: Option<Vec<GenericChoice>>,
            content: Option<String>,
            message: Option<GenericMessage>,
            response: Option<String>,
            generated_text: Option<String>,
            output: Option<String>,
            text: Option<String>,
            results: Option<Vec<GenericResultItem>>,
        }

        #[derive(Deserialize)]
        struct GenericChoice {
            message: GenericMessage,
        }

        #[derive(Deserialize)]
        struct GenericMessage {
            content: Option<String>,
        }

        #[derive(Deserialize)]
        struct GenericResultItem {
            text: Option<String>,
        }

        let body: GenericResponse = resp
            .json()
            .await
            .map_err(|e| PkmError::Ai(format!("Custom provider parse error: {e}")))?;

        let content = body
            .content
            .or_else(|| body.message.and_then(|m| m.content))
            .or_else(|| {
                body.choices
                    .and_then(|c| c.into_iter().next())
                    .and_then(|c| c.message.content)
            })
            .or(body.response)
            .or(body.generated_text)
            .or(body.output)
            .or_else(|| {
                body.results
                    .and_then(|r| r.into_iter().next())
                    .and_then(|r| r.text)
            })
            .or(body.text)
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
        endpoint::validate_endpoint_safe(&self.endpoint)?;
        Err(PkmError::Unsupported(
            "Streaming not supported for custom provider".to_string(),
        ))
    }
}

// ---------------------------------------------------------------------------
