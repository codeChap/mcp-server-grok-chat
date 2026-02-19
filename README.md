# mcp-server-grok-chat

An MCP (Model Context Protocol) server for the xAI Grok API. Built in Rust, exposes chat completions, vision, web/X search, embeddings, and model listing as MCP tools.

Communicates via stdio using JSON-RPC 2.0, like all MCP servers.

## Tools

| Tool | Description |
|------|-------------|
| `chat` | Send a chat completion request to Grok with optional multi-turn history, system prompt, structured output (JSON schema), and model selection |
| `chat_with_vision` | Analyse an image with Grok's vision capabilities given an image URL and text prompt |
| `chat_with_search` | Chat with Grok using live web search and/or X (Twitter) search to ground responses |
| `embedding` | Generate text embeddings using Grok's embedding model |
| `list_models` | List all available Grok models and their IDs (cached for 5 minutes) |

### chat

Send a chat completion request. Supports multi-turn conversations via a JSON message history array, system prompts, structured output via JSON schema, temperature control, and model selection.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `prompt` | string | yes | The user message to send |
| `model` | string | no | Model to use (default: `grok-4-1-fast-non-reasoning`) |
| `system_prompt` | string | no | System prompt to set context |
| `messages` | string | no | Full conversation history as JSON array of `{role, content}` objects |
| `temperature` | float | no | Sampling temperature (0.0 - 2.0) |
| `max_tokens` | integer | no | Maximum tokens to generate |
| `response_schema` | string | no | JSON schema string to enforce structured output |

### chat_with_vision

Analyse an image using Grok's vision capabilities.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `prompt` | string | yes | Text prompt describing what to analyse |
| `image_url` | string | yes | URL of the image (must be http:// or https://) |
| `model` | string | no | Model to use (default: `grok-4-1-fast-non-reasoning`) |
| `detail` | string | no | Image detail level: `low` or `high` (default: `high`) |
| `temperature` | float | no | Sampling temperature (0.0 - 2.0) |
| `max_tokens` | integer | no | Maximum tokens to generate |

### chat_with_search

Chat with Grok using live web search and/or X (Twitter) search. The model automatically searches the internet to ground its response.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `prompt` | string | yes | The user message to send |
| `search_type` | string | no | Search type: `web`, `x`, or `both` (default: `both`) |
| `model` | string | no | Model to use (default: `grok-4-1-fast-non-reasoning`) |
| `system_prompt` | string | no | System prompt to set context |
| `temperature` | float | no | Sampling temperature (0.0 - 2.0) |
| `max_tokens` | integer | no | Maximum tokens to generate |

### embedding

Generate text embeddings.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `input` | string | yes | Text to embed as JSON: a single string or array of strings |
| `model` | string | no | Embedding model to use (default: `grok-2-text-embedding`) |

### list_models

List all available Grok models. No parameters. Results are cached for 5 minutes.

## Prerequisites

- Rust (edition 2024)
- An xAI API key from [console.x.ai](https://console.x.ai)

## Setup

Create the config file:

```bash
mkdir -p ~/.config/mcp-server-grok-chat
```

Create `~/.config/mcp-server-grok-chat/config.toml`:

```toml
api_key = "xai-..."
```

## Build

```bash
cargo build --release
```

This produces `target/release/grok-chat`.

For development:

```bash
cargo build              # debug build
cargo run                # run in dev mode
RUST_LOG=debug cargo run # run with debug logging
```

## MCP Configuration

Add to your Claude Desktop config (`~/.config/Claude/claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "grok-chat": {
      "command": "/path/to/grok-chat"
    }
  }
}
```

## Project Structure

```
src/
  main.rs    - entry point, config loading, stdio transport setup
  server.rs  - MCP tool definitions (chat, chat_with_vision, chat_with_search, embedding, list_models)
  api.rs     - xAI HTTP client, request/response types, response formatters
  params.rs  - tool parameter types with serde and JSON Schema derives
  config.rs  - TOML config loading
```

## License

MIT
