# Changelog

## 0.2.0 — 2026-05-11

### Breaking
- Default chat model changed from `grok-4.20-experimental-beta-0304` to
  `grok-4.3`. The previous default is not present in xAI's live
  `/v1/models` response as of 2026-05-11; callers relying on the default
  may have been silently aliased or broken.

### Changed
- `model` parameter descriptions on `chat`, `chat_with_vision`, and
  `chat_with_search` no longer enumerate model IDs. Call the
  `list_models` tool for the current set of available models.
- `reasoning_effort` documentation updated: on `grok-4.3` the value
  controls native reasoning depth (`low`/`medium`/`high`). On multi-agent
  models it still controls agent count (`low`/`medium` = 4 agents,
  `high`/`xhigh` = 16 agents). `xhigh` is multi-agent-only.

### Notes
- xAI is retiring `grok-3`, `grok-4-0709`, `grok-4-1-fast-reasoning`,
  `grok-4-1-fast-non-reasoning`, `grok-4-fast-reasoning`,
  `grok-4-fast-non-reasoning`, and `grok-code-fast-1` on 2026-05-15.
  None of these were the default; this server requires no further code
  changes for the retirements themselves.
- The default embedding model (`grok-2-text-embedding`) is not visible
  in `/v1/models` as of 2026-05-11. Status unverified — confirm with a
  direct `/v1/embeddings` call before relying on it.
