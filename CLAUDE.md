# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Is

An MCP (Model Context Protocol) server that wraps the xAI Grok API, exposing chat completions, vision, web/X search, embeddings, and model listing as MCP tools over stdio.

## Build & Run

```bash
cargo build --release    # Production build (binary: target/release/grok-chat)
cargo build              # Debug build
cargo run                # Run in dev mode
RUST_LOG=debug cargo run # Run with debug logging
```

No test suite exists. No linter config beyond default `cargo check`/`cargo clippy`.

## Configuration

Config file: `~/.config/mcp-server-grok-chat/config.toml`

```toml
api_key = "xai-..."
```

Loaded at startup in `src/config.rs`. The server will fail immediately if this file is missing or malformed.

## Architecture

Five source files, no sub-crates:

- **`main.rs`** — Entry point. Loads config, creates `XaiClient`, creates `GrokServer`, starts rmcp stdio transport.
- **`config.rs`** — TOML config loading from `~/.config/mcp-server-grok-chat/config.toml`. Single field: `api_key`.
- **`server.rs`** — MCP tool definitions using rmcp `#[tool]` / `#[tool_router]` / `#[tool_handler]` macros. Contains 5 tools: `chat`, `chat_with_vision`, `chat_with_search`, `embedding`, `list_models`. Shared helpers (`validate_temperature`, `build_messages`, `build_chat_request`, `do_chat`, `search_tools`) keep tool methods DRY.
- **`api.rs`** — `XaiClient` HTTP client wrapping reqwest against `https://api.x.ai/v1`. Generic `request<Req, Resp>()` method handles GET/POST with auth. Also contains all xAI API types (`ChatRequest`, `ChatResponse`, `EmbeddingRequest`, etc.) and response formatters.
- **`params.rs`** — Serde + JsonSchema parameter structs for each tool (`ChatParams`, `VisionParams`, `SearchParams`, `EmbeddingParams`). The `#[schemars(description)]` attributes become the tool parameter descriptions exposed to MCP clients.

## Key Constants

- Default chat model: `grok-4-1-fast-non-reasoning` (in `server.rs`)
- Default embedding model: `grok-2-text-embedding` (in `server.rs`)
- API base URL: `https://api.x.ai/v1` (in `api.rs`)
- HTTP timeout: 30 seconds (in `api.rs`)

## Adding a New Tool

1. Add parameter struct to `params.rs` with `Deserialize` + `JsonSchema` derives
2. Add the `#[tool(description = "...")]` method inside the `#[tool_router] impl GrokServer` block in `server.rs`
3. Add any new API types to `api.rs` if calling a new endpoint

## Dependencies

Uses `rmcp` crate (v0.8) for MCP protocol handling. Rust edition 2024.
