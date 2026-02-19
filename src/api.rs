use reqwest::{Client, Method};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;
use std::time::Duration;
use thiserror::Error;
use tracing::instrument;

const DEFAULT_BASE_URL: &str = "https://api.x.ai/v1";

/// Errors returned by the xAI API client.
#[derive(Error, Debug)]
pub enum ApiError {
    #[error("HTTP request failed: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("xAI API error ({status}): {body}")]
    Api {
        status: reqwest::StatusCode,
        body: String,
    },
}

/// Shared HTTP client for all xAI API calls.
pub struct XaiClient {
    api_key: String,
    base_url: String,
    http: Client,
}

impl XaiClient {
    /// Create a new client pointing at the default xAI API base URL.
    pub fn new(api_key: String) -> Self {
        Self::with_base_url(api_key, DEFAULT_BASE_URL.to_string())
    }

    /// Create a new client with a custom base URL (useful for testing with mockito).
    pub fn with_base_url(api_key: String, base_url: String) -> Self {
        Self {
            api_key,
            base_url,
            http: Client::builder()
                .timeout(Duration::from_secs(300))
                .build()
                .expect("Failed to build reqwest client"),
        }
    }

    /// Unified HTTP request method — handles GET and POST with optional body.
    #[instrument(skip(self, body), fields(path = %path))]
    pub async fn request<Req: Serialize, Resp: for<'de> Deserialize<'de>>(
        &self,
        method: Method,
        path: &str,
        body: Option<&Req>,
    ) -> Result<Resp, ApiError> {
        let url = format!("{}{path}", self.base_url);
        let mut builder = self
            .http
            .request(method, &url)
            .header("Authorization", format!("Bearer {}", self.api_key));

        if let Some(b) = body {
            builder = builder.json(b);
        }

        let response = builder.send().await?;

        let status = response.status();
        if !status.is_success() {
            let body = match response.text().await {
                Ok(text) => text,
                Err(e) => format!("<failed to read response body: {e}>"),
            };
            tracing::warn!(status = %status, "API request failed");
            return Err(ApiError::Api { status, body });
        }

        Ok(response.json::<Resp>().await?)
    }
}

// ---------------------------------------------------------------------------
// Chat Completions API types
// ---------------------------------------------------------------------------

/// A chat completion request to the xAI API.
#[derive(Serialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Value>>,
}

/// A single message in a chat conversation.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

/// The response from a chat completion request.
#[derive(Debug, Deserialize)]
pub struct ChatResponse {
    pub choices: Vec<ChatChoice>,
    pub usage: Option<Usage>,
}

/// A single choice in a chat completion response.
#[derive(Debug, Deserialize)]
pub struct ChatChoice {
    pub message: ChatResponseMessage,
    pub finish_reason: Option<String>,
}

/// The message content within a chat choice.
#[derive(Debug, Deserialize)]
pub struct ChatResponseMessage {
    pub role: String,
    pub content: Option<String>,
    pub tool_calls: Option<Vec<Value>>,
}

/// Token usage statistics.
#[derive(Debug, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

// ---------------------------------------------------------------------------
// Embeddings API types
// ---------------------------------------------------------------------------

/// An embedding request to the xAI API.
#[derive(Serialize)]
pub struct EmbeddingRequest {
    pub model: String,
    pub input: Value,
}

/// The response from an embedding request.
#[derive(Deserialize)]
pub struct EmbeddingResponse {
    pub data: Vec<EmbeddingData>,
    pub usage: Option<EmbeddingUsage>,
}

/// A single embedding vector in the response.
#[derive(Deserialize)]
pub struct EmbeddingData {
    pub embedding: Vec<f32>,
    pub index: usize,
}

/// Token usage for an embedding request.
#[derive(Deserialize)]
pub struct EmbeddingUsage {
    pub prompt_tokens: u32,
    pub total_tokens: u32,
}

// ---------------------------------------------------------------------------
// Models API types
// ---------------------------------------------------------------------------

/// The response from listing available models.
#[derive(Deserialize)]
pub struct ModelsResponse {
    pub data: Vec<ModelInfo>,
}

/// Information about a single model.
#[derive(Deserialize)]
pub struct ModelInfo {
    pub id: String,
    #[serde(default)]
    pub owned_by: Option<String>,
}

// ---------------------------------------------------------------------------
// Convenience builders
// ---------------------------------------------------------------------------

impl ChatRequest {
    /// Create a new chat request with the given model and messages.
    pub fn new(model: &str, messages: Vec<ChatMessage>) -> Self {
        Self {
            model: model.into(),
            messages,
            temperature: None,
            max_tokens: None,
            response_format: None,
            tools: None,
        }
    }
}

impl ChatMessage {
    /// Create a system message.
    pub fn system(text: &str) -> Self {
        Self {
            role: "system".into(),
            content: Some(Value::String(text.into())),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    /// Create a user text message.
    pub fn user(text: &str) -> Self {
        Self {
            role: "user".into(),
            content: Some(Value::String(text.into())),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    /// Create a user message with both text and an image URL.
    pub fn user_with_image(text: &str, image_url: &str, detail: &str) -> Self {
        Self {
            role: "user".into(),
            content: Some(serde_json::json!([
                { "type": "text", "text": text },
                {
                    "type": "image_url",
                    "image_url": { "url": image_url, "detail": detail }
                }
            ])),
            tool_calls: None,
            tool_call_id: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Display implementations (replace standalone format functions)
// ---------------------------------------------------------------------------

impl fmt::Display for ChatResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;
        for choice in &self.choices {
            if !first {
                writeln!(f)?;
            }
            first = false;

            if choice.message.role != "assistant" {
                write!(f, "[{}] ", choice.message.role)?;
            }
            if let Some(content) = &choice.message.content {
                write!(f, "{content}")?;
            }
            if let Some(tool_calls) = &choice.message.tool_calls {
                let formatted = serde_json::to_string_pretty(tool_calls)
                    .unwrap_or_else(|e| format!("<failed to format tool calls: {e}>"));
                write!(f, "\nTool calls: {formatted}")?;
            }
            if let Some(reason) = &choice.finish_reason {
                write!(f, "\n[finish_reason: {reason}]")?;
            }
        }

        if let Some(usage) = &self.usage {
            write!(
                f,
                "\n[tokens: {} prompt + {} completion = {} total]",
                usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
            )?;
        }

        Ok(())
    }
}

impl fmt::Display for EmbeddingResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, item) in self.data.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            let preview: Vec<String> = item
                .embedding
                .iter()
                .take(5)
                .map(|v| format!("{v:.6}"))
                .collect();
            write!(
                f,
                "[{}] dim={} [{}, ...]",
                item.index,
                item.embedding.len(),
                preview.join(", ")
            )?;
        }

        if let Some(usage) = &self.usage {
            write!(
                f,
                "\n[tokens: {} prompt, {} total]",
                usage.prompt_tokens, usage.total_tokens
            )?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_chat_response_basic() {
        let resp = ChatResponse {
            choices: vec![ChatChoice {
                message: ChatResponseMessage {
                    role: "assistant".into(),
                    content: Some("Hello!".into()),
                    tool_calls: None,
                },
                finish_reason: Some("stop".into()),
            }],
            usage: Some(Usage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
            }),
        };
        let output = resp.to_string();
        assert!(output.contains("Hello!"));
        assert!(output.contains("[finish_reason: stop]"));
        assert!(output.contains("[tokens: 10 prompt + 5 completion = 15 total]"));
    }

    #[test]
    fn display_chat_response_empty_choices() {
        let resp = ChatResponse {
            choices: vec![],
            usage: None,
        };
        assert_eq!(resp.to_string(), "");
    }

    #[test]
    fn display_chat_response_with_tool_calls() {
        let resp = ChatResponse {
            choices: vec![ChatChoice {
                message: ChatResponseMessage {
                    role: "assistant".into(),
                    content: None,
                    tool_calls: Some(vec![
                        serde_json::json!({"id": "call_1", "type": "function"}),
                    ]),
                },
                finish_reason: None,
            }],
            usage: None,
        };
        let output = resp.to_string();
        assert!(output.contains("Tool calls:"));
        assert!(output.contains("call_1"));
    }

    #[test]
    fn display_embedding_response_basic() {
        let resp = EmbeddingResponse {
            data: vec![EmbeddingData {
                embedding: vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6],
                index: 0,
            }],
            usage: Some(EmbeddingUsage {
                prompt_tokens: 8,
                total_tokens: 8,
            }),
        };
        let output = resp.to_string();
        assert!(output.contains("[0] dim=6"));
        assert!(output.contains("[tokens: 8 prompt, 8 total]"));
    }
}
