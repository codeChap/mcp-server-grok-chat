use schemars::JsonSchema;
use serde::Deserialize;

/// Search type for the `chat_with_search` tool.
#[derive(Debug, Clone, Default, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum SearchType {
    /// Search the web only.
    Web,
    /// Search X (Twitter) only.
    X,
    /// Search both web and X (default).
    #[default]
    Both,
}

/// Image detail level for the `chat_with_vision` tool.
#[derive(Debug, Clone, Default, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ImageDetail {
    /// Low detail — faster, fewer tokens.
    Low,
    /// High detail — more accurate (default).
    #[default]
    High,
    /// Let the model decide.
    Auto,
}

impl ImageDetail {
    /// Return the string representation used by the xAI API.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::High => "high",
            Self::Auto => "auto",
        }
    }
}

/// Parameters for the `chat` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ChatParams {
    #[schemars(description = "The user message / prompt to send to Grok")]
    pub prompt: String,

    #[schemars(description = "Optional system prompt to set context/behaviour")]
    pub system_prompt: Option<String>,

    #[schemars(
        description = "Full conversation history as JSON array of {role, content} objects. \
                        When provided, 'prompt' is appended as the final user message."
    )]
    pub messages: Option<String>,

    #[schemars(
        description = "Model to use. Defaults to grok-4-1-fast-non-reasoning. \
                        Options: grok-4-1-fast-reasoning, grok-4-1-fast-non-reasoning, \
                        grok-4-fast-reasoning, grok-4-0709, grok-3, grok-3-mini, grok-code-fast-1"
    )]
    pub model: Option<String>,

    #[schemars(description = "Sampling temperature (0.0 - 2.0)")]
    pub temperature: Option<f32>,

    #[schemars(description = "Maximum tokens to generate")]
    pub max_tokens: Option<u32>,

    #[schemars(
        description = "Optional JSON schema string to enforce structured output. \
                        The model response will conform to this schema."
    )]
    pub response_schema: Option<String>,
}

/// Parameters for the `chat_with_vision` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct VisionParams {
    #[schemars(description = "Text prompt describing what to analyse in the image")]
    pub prompt: String,

    #[schemars(description = "URL of the image to analyse (must be http:// or https://)")]
    pub image_url: String,

    #[schemars(description = "Image detail level: \"low\" or \"high\" (default: \"high\")")]
    pub detail: Option<ImageDetail>,

    #[schemars(
        description = "Model to use. Defaults to grok-4-1-fast-non-reasoning. \
                        Must be a vision-capable model."
    )]
    pub model: Option<String>,

    #[schemars(description = "Sampling temperature (0.0 - 2.0)")]
    pub temperature: Option<f32>,

    #[schemars(description = "Maximum tokens to generate")]
    pub max_tokens: Option<u32>,
}

/// Parameters for the `chat_with_search` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchParams {
    #[schemars(description = "The user message / prompt")]
    pub prompt: String,

    #[schemars(description = "Optional system prompt")]
    pub system_prompt: Option<String>,

    #[schemars(
        description = "Search type to enable: \"web\", \"x\" (X/Twitter), or \"both\" (default: \"both\")"
    )]
    pub search_type: Option<SearchType>,

    #[schemars(description = "Model to use. Defaults to grok-4-1-fast-non-reasoning.")]
    pub model: Option<String>,

    #[schemars(description = "Sampling temperature (0.0 - 2.0)")]
    pub temperature: Option<f32>,

    #[schemars(description = "Maximum tokens to generate")]
    pub max_tokens: Option<u32>,
}

/// Parameters for the `embedding` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct EmbeddingParams {
    #[schemars(description = "Text to embed as JSON: a single string or array of strings.")]
    pub input: String,

    #[schemars(description = "Embedding model to use (default: grok-2-text-embedding)")]
    pub model: Option<String>,
}
