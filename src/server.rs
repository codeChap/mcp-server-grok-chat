use moka::future::Cache;
use reqwest::Method;
use rmcp::{
    ErrorData as McpError, ServerHandler, handler::server::tool::ToolRouter,
    handler::server::wrapper::Parameters, model::*, tool, tool_handler, tool_router,
};
use serde_json::Value;
use std::time::Duration;
use tracing::debug;

use crate::api::{
    ChatMessage, ChatRequest, ChatResponse, EmbeddingRequest, EmbeddingResponse, ModelsResponse,
    ResponsesMessage, ResponsesRequest, ResponsesResponse, XaiClient,
};
use crate::params::{ChatParams, EmbeddingParams, SearchParams, SearchType, VisionParams};

const DEFAULT_MODEL: &str = "grok-4-1-fast-non-reasoning";
const DEFAULT_EMBEDDING_MODEL: &str = "grok-2-text-embedding";

/// Valid roles for chat messages.
const VALID_ROLES: &[&str] = &["system", "user", "assistant", "tool"];

/// The MCP server wrapping the xAI Grok API.
#[derive(Clone)]
pub struct GrokServer {
    client: std::sync::Arc<XaiClient>,
    models_cache: Cache<(), String>,
    tool_router: ToolRouter<Self>,
}

// ---------------------------------------------------------------------------
// Shared helpers — keep tool methods DRY
// ---------------------------------------------------------------------------

impl GrokServer {
    /// Validate temperature is within the allowed range and is a finite number.
    fn validate_temperature(temp: Option<f32>) -> Result<(), McpError> {
        if let Some(t) = temp
            && (!t.is_finite() || !(0.0..=2.0).contains(&t))
        {
            return Err(McpError::invalid_params(
                format!("temperature must be a finite number between 0.0 and 2.0, got {t}"),
                None,
            ));
        }
        Ok(())
    }

    /// Build the messages vec from optional system prompt, optional history, and current prompt.
    fn build_messages(
        system_prompt: Option<&str>,
        history_json: Option<&str>,
        prompt: &str,
    ) -> Result<Vec<ChatMessage>, String> {
        let mut messages = Vec::new();

        if let Some(sys) = system_prompt {
            messages.push(ChatMessage::system(sys));
        }

        if let Some(json) = history_json {
            let parsed: Vec<ChatMessage> =
                serde_json::from_str(json).map_err(|e| format!("Invalid messages JSON: {e}"))?;
            for msg in &parsed {
                if !VALID_ROLES.contains(&msg.role.as_str()) {
                    return Err(format!(
                        "Invalid role '{}' in messages — must be one of: {}",
                        msg.role,
                        VALID_ROLES.join(", ")
                    ));
                }
            }
            messages.extend(parsed);
        }

        messages.push(ChatMessage::user(prompt));
        Ok(messages)
    }

    /// Build a ChatRequest with shared optional fields applied.
    fn build_chat_request(
        model: Option<&str>,
        messages: Vec<ChatMessage>,
        temperature: Option<f32>,
        max_tokens: Option<u32>,
        response_schema: Option<&str>,
        tools: Option<Vec<Value>>,
    ) -> Result<ChatRequest, String> {
        let mut req = ChatRequest::new(model.unwrap_or(DEFAULT_MODEL), messages);
        req.temperature = temperature;
        req.max_tokens = max_tokens;
        req.tools = tools;

        if let Some(schema_str) = response_schema {
            let schema: Value = serde_json::from_str(schema_str)
                .map_err(|e| format!("Invalid response_schema JSON: {e}"))?;
            req.response_format = Some(serde_json::json!({
                "type": "json_schema",
                "json_schema": {
                    "name": "structured_output",
                    "strict": true,
                    "schema": schema
                }
            }));
        }

        Ok(req)
    }

    /// Send a chat request and return the formatted result.
    async fn do_chat(&self, req: &ChatRequest) -> Result<CallToolResult, McpError> {
        match self
            .client
            .request::<_, ChatResponse>(Method::POST, "/chat/completions", Some(req))
            .await
        {
            Ok(resp) => Ok(CallToolResult::success(vec![Content::text(
                resp.to_string(),
            )])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(e.to_string())])),
        }
    }

    /// Build search tool definitions for the xAI agent tools API.
    fn search_tools(search_type: SearchType) -> Vec<Value> {
        let mut tools = Vec::new();
        if matches!(search_type, SearchType::Web | SearchType::Both) {
            tools.push(serde_json::json!({ "type": "web_search" }));
        }
        if matches!(search_type, SearchType::X | SearchType::Both) {
            tools.push(serde_json::json!({ "type": "x_search" }));
        }
        tools
    }
}

// ---------------------------------------------------------------------------
// Tool definitions
// ---------------------------------------------------------------------------

#[tool_router]
impl GrokServer {
    pub fn new(client: XaiClient) -> Self {
        let models_cache = Cache::builder()
            .max_capacity(1)
            .time_to_live(Duration::from_secs(300))
            .build();

        Self {
            client: std::sync::Arc::new(client),
            models_cache,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        description = "Send a chat completion request to Grok. Supports multi-turn conversations, \
                           structured output via JSON schema, and model selection."
    )]
    async fn chat(
        &self,
        Parameters(p): Parameters<ChatParams>,
    ) -> Result<CallToolResult, McpError> {
        debug!(model = ?p.model, "chat tool called");
        Self::validate_temperature(p.temperature)?;

        let messages =
            Self::build_messages(p.system_prompt.as_deref(), p.messages.as_deref(), &p.prompt)
                .map_err(|e| McpError::invalid_params(e, None))?;

        let req = Self::build_chat_request(
            p.model.as_deref(),
            messages,
            p.temperature,
            p.max_tokens,
            p.response_schema.as_deref(),
            None,
        )
        .map_err(|e| McpError::invalid_params(e, None))?;

        self.do_chat(&req).await
    }

    #[tool(description = "Analyse an image with Grok's vision capabilities. \
                           Provide an image URL and a text prompt.")]
    async fn chat_with_vision(
        &self,
        Parameters(p): Parameters<VisionParams>,
    ) -> Result<CallToolResult, McpError> {
        debug!(model = ?p.model, "chat_with_vision tool called");
        if !p.image_url.starts_with("http://") && !p.image_url.starts_with("https://") {
            return Err(McpError::invalid_params(
                "image_url must start with http:// or https://",
                None,
            ));
        }
        Self::validate_temperature(p.temperature)?;

        let detail = p.detail.unwrap_or_default();
        let messages = vec![ChatMessage::user_with_image(
            &p.prompt,
            &p.image_url,
            detail.as_str(),
        )];

        let req = Self::build_chat_request(
            p.model.as_deref(),
            messages,
            p.temperature,
            p.max_tokens,
            None,
            None,
        )
        .map_err(|e| McpError::invalid_params(e, None))?;

        self.do_chat(&req).await
    }

    #[tool(
        description = "Chat with Grok using live web search and/or X (Twitter) search. \
                           The model will automatically search the internet to ground its response."
    )]
    async fn chat_with_search(
        &self,
        Parameters(p): Parameters<SearchParams>,
    ) -> Result<CallToolResult, McpError> {
        debug!(model = ?p.model, search_type = ?p.search_type, "chat_with_search tool called");
        Self::validate_temperature(p.temperature)?;

        let search_type = p.search_type.unwrap_or_default();

        let mut input = Vec::new();
        if let Some(sys) = &p.system_prompt {
            input.push(ResponsesMessage::system(sys));
        }
        input.push(ResponsesMessage::user(&p.prompt));

        let tools = Self::search_tools(search_type);

        let req = ResponsesRequest {
            model: p.model.unwrap_or_else(|| DEFAULT_MODEL.into()),
            input,
            temperature: p.temperature,
            max_output_tokens: p.max_tokens,
            tools: Some(tools),
        };

        match self
            .client
            .request::<_, ResponsesResponse>(Method::POST, "/responses", Some(&req))
            .await
        {
            Ok(resp) => Ok(CallToolResult::success(vec![Content::text(
                resp.to_string(),
            )])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(e.to_string())])),
        }
    }

    #[tool(description = "Generate text embeddings using Grok's embedding model.")]
    async fn embedding(
        &self,
        Parameters(p): Parameters<EmbeddingParams>,
    ) -> Result<CallToolResult, McpError> {
        debug!(model = ?p.model, "embedding tool called");
        let input: Value = serde_json::from_str(&p.input).map_err(|e| {
            McpError::invalid_params(
                format!("Invalid input JSON (must be a quoted string or array of strings): {e}"),
                None,
            )
        })?;

        let req = EmbeddingRequest {
            model: p.model.unwrap_or_else(|| DEFAULT_EMBEDDING_MODEL.into()),
            input,
        };

        match self
            .client
            .request::<_, EmbeddingResponse>(Method::POST, "/embeddings", Some(&req))
            .await
        {
            Ok(resp) => Ok(CallToolResult::success(vec![Content::text(
                resp.to_string(),
            )])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(e.to_string())])),
        }
    }

    #[tool(description = "List all available Grok models and their IDs.")]
    async fn list_models(&self) -> Result<CallToolResult, McpError> {
        // Check cache first
        if let Some(cached) = self.models_cache.get(&()).await {
            debug!("list_models: returning cached result");
            return Ok(CallToolResult::success(vec![Content::text(cached.clone())]));
        }

        debug!("list_models: fetching from API");
        match self
            .client
            .request::<(), ModelsResponse>(Method::GET, "/models", None)
            .await
        {
            Ok(resp) => {
                let lines: Vec<String> = resp
                    .data
                    .iter()
                    .map(|m| {
                        let owner = m.owned_by.as_deref().unwrap_or("xai");
                        format!("- {} ({})", m.id, owner)
                    })
                    .collect();
                let result = lines.join("\n");
                self.models_cache.insert((), result.clone()).await;
                Ok(CallToolResult::success(vec![Content::text(result)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(e.to_string())])),
        }
    }
}

// ---------------------------------------------------------------------------
// MCP ServerHandler
// ---------------------------------------------------------------------------

#[tool_handler]
impl ServerHandler for GrokServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::default(),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "grok-chat".into(),
                title: None,
                version: env!("CARGO_PKG_VERSION").into(),
                icons: None,
                website_url: None,
            },
            instructions: Some(
                "xAI Grok MCP server. Tools: chat, chat_with_vision, chat_with_search, \
                 embedding, list_models."
                    .into(),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- validate_temperature -------------------------------------------------

    #[test]
    fn validate_temperature_none_is_ok() {
        assert!(GrokServer::validate_temperature(None).is_ok());
    }

    #[test]
    fn validate_temperature_valid_range() {
        assert!(GrokServer::validate_temperature(Some(0.0)).is_ok());
        assert!(GrokServer::validate_temperature(Some(1.0)).is_ok());
        assert!(GrokServer::validate_temperature(Some(2.0)).is_ok());
    }

    #[test]
    fn validate_temperature_out_of_range() {
        assert!(GrokServer::validate_temperature(Some(-0.1)).is_err());
        assert!(GrokServer::validate_temperature(Some(2.1)).is_err());
    }

    #[test]
    fn validate_temperature_nan() {
        assert!(GrokServer::validate_temperature(Some(f32::NAN)).is_err());
    }

    #[test]
    fn validate_temperature_infinity() {
        assert!(GrokServer::validate_temperature(Some(f32::INFINITY)).is_err());
        assert!(GrokServer::validate_temperature(Some(f32::NEG_INFINITY)).is_err());
    }

    // -- build_messages -------------------------------------------------------

    #[test]
    fn build_messages_basic() {
        let msgs = GrokServer::build_messages(None, None, "hello").unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].role, "user");
    }

    #[test]
    fn build_messages_with_system_prompt() {
        let msgs = GrokServer::build_messages(Some("be helpful"), None, "hello").unwrap();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].role, "system");
        assert_eq!(msgs[1].role, "user");
    }

    #[test]
    fn build_messages_with_history() {
        let history =
            r#"[{"role": "user", "content": "hi"}, {"role": "assistant", "content": "hey"}]"#;
        let msgs = GrokServer::build_messages(None, Some(history), "next").unwrap();
        assert_eq!(msgs.len(), 3);
        assert_eq!(msgs[0].role, "user");
        assert_eq!(msgs[1].role, "assistant");
        assert_eq!(msgs[2].role, "user");
    }

    #[test]
    fn build_messages_invalid_json() {
        let result = GrokServer::build_messages(None, Some("not json"), "hello");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid messages JSON"));
    }

    #[test]
    fn build_messages_invalid_role() {
        let history = r#"[{"role": "hacker", "content": "hi"}]"#;
        let result = GrokServer::build_messages(None, Some(history), "hello");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid role 'hacker'"));
    }

    // -- build_chat_request ---------------------------------------------------

    #[test]
    fn build_chat_request_defaults() {
        let msgs = vec![ChatMessage::user("hello")];
        let req = GrokServer::build_chat_request(None, msgs, None, None, None, None).unwrap();
        assert_eq!(req.model, DEFAULT_MODEL);
        assert!(req.temperature.is_none());
        assert!(req.response_format.is_none());
    }

    #[test]
    fn build_chat_request_with_schema() {
        let msgs = vec![ChatMessage::user("hello")];
        let schema = r#"{"type": "object", "properties": {"name": {"type": "string"}}}"#;
        let req =
            GrokServer::build_chat_request(None, msgs, None, None, Some(schema), None).unwrap();
        assert!(req.response_format.is_some());
    }

    #[test]
    fn build_chat_request_invalid_schema() {
        let msgs = vec![ChatMessage::user("hello")];
        let result = GrokServer::build_chat_request(None, msgs, None, None, Some("not json"), None);
        assert!(result.is_err());
    }

    // -- search_tools ---------------------------------------------------------

    #[test]
    fn search_tools_web_only() {
        let tools = GrokServer::search_tools(SearchType::Web);
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["type"], "web_search");
    }

    #[test]
    fn search_tools_x_only() {
        let tools = GrokServer::search_tools(SearchType::X);
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["type"], "x_search");
    }

    #[test]
    fn search_tools_both() {
        let tools = GrokServer::search_tools(SearchType::Both);
        assert_eq!(tools.len(), 2);
    }
}
