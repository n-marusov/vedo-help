# RouterAI API Reference

> Source: https://routerai.ru/docs, https://routerai.ru/docs/reference, https://routerai.ru/api/openapi.json
> Created: 2026-06-21
> Updated: 2026-06-21

## Overview

RouterAI is a unified API gateway providing access to 100+ AI models from OpenAI, Anthropic, Google, DeepSeek, xAI, and other providers. It is designed for users in Russia and CIS, offering RUB-based billing, local payment methods (Russian bank cards, SBP, wire transfer for legal entities), and no VPN requirements.

The API is compatible with the OpenAI SDK format, making migration straightforward. It also provides an Anthropic Messages-compatible endpoint for tools like Claude Code.

**Base endpoint:** `https://routerai.ru/api/v1`

## Core Concepts

### API Endpoint

All requests go to `https://routerai.ru/api/v1` with an OpenAI-compatible format. The API supports both streaming (SSE) and non-streaming modes.

### Authentication

Two types of keys:

| Key Type | Purpose | Created via |
|----------|---------|-------------|
| **API key** (`sk-...`) | AI model calls (chat completions, completions, embeddings, etc.) | UI or API |
| **Master key** | Programmatic API key management (CRUD on `/keys/*`) | UI only |

Authentication is via `Authorization: Bearer <API_KEY>` header.

### Provider Selection

RouterAI distributes load across providers balancing stability (no failures in last 30s), price (cheapest first), and fallbacks. Customize via the `provider` object in request body:

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `order` | `Array<String>` | — | Preferred provider slug order (e.g. `["openai","anthropic"]`) |
| `only` | `Array<String>` | — | Whitelist of providers |
| `ignore` | `Array<String>` | — | Blacklist of providers |
| `allow_fallbacks` | `Boolean` | `true` | Retry on fallback provider on error |
| `country` | `String` | — | Two-letter country code for geo-filtering (e.g. `"ru"`) |

Provider slugs: `openai`, `anthropic`, `google`, `deepseek`, `yandex`, `x-ai`, `mistralai`, `qwen`, etc.

Note: `order/only/ignore` only apply when routing through routerAI's multi-provider mesh. Direct/static routes bypass these fields.

### Session Tracking

Optional `session_id` field (max 256 chars) groups related requests. Can also be passed via `X-Session-Id` header. Body takes precedence.

## API Endpoints

All endpoints are under `https://routerai.ru/api/v1`. Authentication: `Authorization: Bearer <API_KEY>`.

### Chat Completions

**`POST /v1/chat/completions`**

OpenAI Chat Completions-compatible endpoint for conversational AI. Supports streaming and non-streaming.

**Required params:** `model`, `messages`

**Request body key fields:**

| Field | Type | Description |
|-------|------|-------------|
| `model` | `string` | Model ID from catalog (e.g. `openai/gpt-5.2`, `anthropic/claude-sonnet-4.6`) |
| `messages` | `array` | Array of `{role, content}` objects. Roles: `system`, `developer`, `user`, `assistant`, `tool` |
| `stream` | `boolean` | Enable streaming (SSE) |
| `temperature` | `float` | 0.0–2.0, default 1.0 |
| `session_id` | `string` | Optional session grouping identifier |
| `provider` | `object` | Provider selection control |

**Response:** Standard OpenAI ChatCompletion object with `id`, `choices[]`, `usage`, etc.

### Completions (Legacy)

**`POST /v1/completions`**

Legacy text completions endpoint.

**Required params:** `model`, `prompt`

### Responses (OpenAI Responses API)

**`POST /v1/responses`**

New OpenAI Responses API format. Supports streaming.

**Required params:** `model`, `input`

### Anthropic Messages API

**`POST /v1/messages`**

Anthropic-compatible endpoint for Claude Code and Anthropic SDK. Accepts Anthropic message format with content blocks (text, tool_use, thinking).

**Required params:** `model`, `max_tokens`, `messages`

**Key fields:**

| Field | Type | Description |
|-------|------|-------------|
| `model` | `string` | Model ID (recommended: `anthropic/claude-*`) |
| `max_tokens` | `integer` | Maximum output tokens |
| `messages` | `array` | Array of `{role, content}` |
| `system` | `string` | System prompt (string or content block array) |
| `stream` | `boolean` | SSE streaming |
| `thinking` | `object` | Extended thinking config `{type:"enabled", budget_tokens}` |
| `tools` | `array` | Tools in Anthropic format `{name, description, input_schema}` |
| `provider` | `object` | Provider selection control |

Non-Anthropic models are auto-translated via Chat Completions format transparently.

### Models

**`GET /v1/models`**

Returns list of available models. No auth required.

**Response:** Array of models with `id`, `name`, `context_length`, `pricing`, `supported_parameters`, etc.

### API Keys Management

Requires Master Key auth.

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/v1/keys` | List all API keys |
| `POST` | `/v1/keys` | Create new key `{name, limit?}` |
| `GET` | `/v1/keys/{hash}` | Get key info by hash |
| `PATCH` | `/v1/keys/{hash}` | Update key `{name?, disabled?, limit?}` |
| `DELETE` | `/v1/keys/{hash}` | Delete key |
| `GET` | `/v1/key` | Get current API key info |

### Embeddings

**`POST /v1/embeddings`**

**Required params:** `model`, `input`
**Optional:** `encoding_format` (`float` or `base64`)

### Rerank

**`POST /v1/rerank`**

Re-rank documents by relevance to query.

**Required params:** `model`, `query`, `documents`
**Optional:** `top_n`, `return_documents`

**Response:** Array of `{index, relevance_score}`

### Speech (TTS)

**`POST /v1/audio/speech`**

Text-to-speech. Returns binary audio.

**Required params:** `model`, `input`, `voice`
**Optional:** `response_format` (`mp3`, `pcm`), `speed`

### Transcription (STT)

**`POST /v1/audio/transcriptions`**

Speech-to-text. Accepts base64 audio.

**Required params:** `model`, `input_audio` `{data, format}`
**Optional:** `language` (ISO-639-1), `temperature`

**Response:** `{text, usage: {seconds}}`

### Credits

**`GET /v1/credits`**

Returns current balance: `{credits: number}`

### Generation Info

**`GET /v1/generation?id={id}`**

Get details of a past request by `X-Generation-Id` header or body `id`.

**Response:** `{id, created_at, model, api, source, total_cost, latency_ms, provider, usage, has_web_search}`

## Parameters

| Parameter | Type | Range | Default | Description |
|-----------|------|-------|---------|-------------|
| `temperature` | float | 0.0–2.0 | 1.0 | Response randomness |
| `top_p` | float | 0.0–1.0 | 1.0 | Nucleus sampling threshold |
| `top_k` | integer | 0+ | 0 (off) | Top-K token selection |
| `frequency_penalty` | float | -2.0–2.0 | 0.0 | Penalize frequent tokens (scales with count) |
| `presence_penalty` | float | -2.0–2.0 | 0.0 | Penalize repeated tokens (does not scale) |
| `repetition_penalty` | float | 0.0–2.0 | 1.0 | Reduce token repetition |
| `min_p` | float | 0.0–1.0 | 0.0 | Min probability relative to best token |
| `top_a` | float | 0.0–1.0 | 0.0 | Dynamic Top-P based on best token |
| `seed` | integer | — | — | Deterministic sampling (not guaranteed for all models) |
| `max_tokens` | integer | 1+ | — | Max output tokens |
| `max_completion_tokens` | integer | 1+ | — | Alternative max tokens field |
| `logit_bias` | map | -100–100 | — | Token ID → bias value map |
| `logprobs` | boolean | — | false | Return log probabilities |
| `top_logprobs` | integer | 0–20 | — | Number of top logprobs (requires `logprobs=true`) |
| `response_format` | map | — | — | `{"type": "json_object"}` or `json_schema` |
| `structured_outputs` | boolean | — | — | Enable structured output with json_schema |
| `stop` | array | — | — | Stop sequences |
| `tools` | array | — | — | Tool definitions (OpenAI format) |
| `tool_choice` | string/object | — | — | `"none"`, `"auto"`, `"required"`, or `{type:"function", function:{name:...}}` |
| `parallel_tool_calls` | boolean | — | true | Allow parallel function calling |
| `verbosity` | enum | low/medium/high/xhigh/max | medium | Response verbosity (maps to Anthropic `output_config.effort`) |

## Error Codes

| Code | Meaning |
|------|---------|
| `401` | Invalid or missing API key |
| `402` | Insufficient balance |
| `400` | Invalid request parameters |
| `500`/`502`/`503` | Provider error (with fallbacks: retried; without: passed through) |
| `503` | No available provider for requested model |
| `404` | Resource not found |

## Agent Integration Guides

### Generic Configuration Pattern

All integrations use: `base_url: https://routerai.ru/api/v1` + API key. Most use "OpenAI Compatible" provider type.

### VS Code — Cline

1. Install Cline from VS Code Marketplace
2. Select **"Bring my own API key"** → **API Provider:** `OpenAI Compatible`
3. Set **Base URL:** `https://routerai.ru/api/v1`, **API Key:** your `sk-...` key
4. Set **Model ID:** e.g. `anthropic/claude-sonnet-4.6`
5. Rules stored in `.clinerules/rules.md`

### VS Code — Roo Code

1. Install Roo Code from VS Code Marketplace
2. Select **"Use another provider"** → **API Provider:** `OpenAI Compatible`
3. Set **Base URL:** `https://routerai.ru/api/v1`, **API Key:** your key
4. Supports **Custom Modes** — assign different models per role (Architect, Code, Ask)
5. Rules stored in `.roo/rules/rules.md`

### VS Code — Kilo Code

1. Install Kilo Code from VS Code Marketplace
2. Click gear icon → **Providers** → **Custom Provider** → **Connect**
3. Fill: **Provider ID:** `routerai`, **Display name:** `RouterAI`, **Base URL:** `https://routerai.ru/api/v1`, **API Key:** your key
4. Add models with model-id and display name
5. Supports **Roles** with per-role model assignment
6. Rules stored in `.kilocode/rules/rules.md`

### VS Code — Continue

1. Install Continue from VS Code Marketplace
2. Edit `config.yaml` (via Settings → Configs → Local Config)
3. Add models with `provider: openai`, `apiBase: https://routerai.ru/api/v1`, `apiKey: YOUR_KEY`

```yaml
models:
  - name: Claude Sonnet 4.6
    provider: openai
    model: anthropic/claude-sonnet-4.6
    apiKey: YOUR_KEY
    apiBase: https://routerai.ru/api/v1
```

4. Rules stored in `.continue/rules/`

### IntelliJ IDEA — Cline

Same as VS Code Cline setup. Install from JetBrains Marketplace, configure OpenAI Compatible provider.

### IntelliJ IDEA — Kilo Code

Same as VS Code Kilo Code setup. Install from JetBrains Marketplace.

### Zed Editor

Edit `settings.json` (Zed → Settings → Open Settings):

```json
{
  "agent": {
    "default_model": {
      "provider": "RouterAI",
      "model": "anthropic/claude-sonnet-4.6"
    }
  },
  "language_models": {
    "openai_compatible": {
      "RouterAI": {
        "api_url": "https://routerai.ru/api/v1",
        "available_models": [
          {
            "name": "anthropic/claude-sonnet-4.6",
            "display_name": "Claude Sonnet 4.6 (RouterAI)",
            "max_tokens": 200000,
            "max_output_tokens": 32000,
            "capabilities": {
              "tools": true,
              "images": true
            }
          }
        ]
      }
    }
  }
}
```

API key set via environment variable `ROUTERAI_API_KEY` or UI. Rules via `.rules` file in project root.

### Claude Code (CLI)

Configure via environment variables:

```bash
export ANTHROPIC_BASE_URL="https://routerai.ru/api"
export ANTHROPIC_AUTH_TOKEN="your-api-key"
export ANTHROPIC_API_KEY=""  # Must be empty string
```

Or via `~/.claude/settings.json`:

```json
{
  "env": {
    "ANTHROPIC_BASE_URL": "https://routerai.ru/api",
    "ANTHROPIC_AUTH_TOKEN": "your-api-key",
    "ANTHROPIC_API_KEY": ""
  }
}
```

Set model overrides:
- `ANTHROPIC_DEFAULT_SONNET_MODEL` — main tasks
- `ANTHROPIC_DEFAULT_OPUS_MODEL` — heavy tasks
- `ANTHROPIC_DEFAULT_HAIKU_MODEL` — fast operations
- `CLAUDE_CODE_SUBAGENT_MODEL` — sub-agents

A status bar script is available at `https://routerai.ru/scripts/claude-statusline.sh` (bash/macOS/Linux/WSL) or `https://routerai.ru/scripts/claude-statusline-win.mjs` (Windows native).

### OpenCode (CLI)

Configure `~/.config/opencode/opencode.json`:

```json
{
  "provider": {
    "routerai": {
      "npm": "@ai-sdk/openai-compatible",
      "name": "RouterAI",
      "options": { "baseURL": "https://routerai.ru/api/v1" },
      "models": {
        "anthropic/claude-sonnet-4.6": { "name": "Claude Sonnet 4.6" },
        "deepseek/deepseek-v3.2": { "name": "DeepSeek V3.2" }
      }
    }
  }
}
```

Enter API key via UI (`Ctrl+P` → Connect Provider → RouterAI). Rules via `AGENTS.md` (generated with `/init`).

### NeoVIM — Avante

Configure avante.nvim:

```lua
{
  "yetone/avante.nvim",
  opts = {
    provider = "routerai",
    providers = {
      routerai = {
        __inherited_from = "openai",
        endpoint = "https://routerai.ru/api/v1/",
        model = "anthropic/claude-sonnet-4.6",
        api_key_name = 'ROUTERAI_API_KEY',
      }
    }
  }
}
```

Rules via `avante.md` in project root.

### n8n

1. In AI Agent node, set **Chat Model** → **OpenAI Chat Model**
2. Create credential: **Base URL:** `https://routerai.ru/api/v1`, **API Key:** from RouterAI
3. Select any model from RouterAI catalog

### Dify

1. Go to **Providers** → Search "OpenAI-API-compatible" → **Install**
2. Click **Add Model**: fill **Model Name** (e.g. `anthropic/claude-4.6-sonnet`), **Model Type:** LLM, **API Key:** from RouterAI, **API Endpoint URL:** `https://routerai.ru/api/v1`
3. Recommend temperature ranges: code 0.1–0.3, dialogue 0.5–0.7, ideas 0.8–1.0, classification 0.0–0.2

### OpenClaw

1. Install via `curl -fsSL https://openclaw.ai/install.sh | bash` or Docker
2. Set auth: `openclaw models auth paste-token --provider routerai`
3. Configure provider in RAW config:
```json
{
  "providers": {
    "routerai": {
      "baseUrl": "https://routerai.ru/api/v1",
      "api": "openai-completions",
      "models": [{ "id": "deepseek/deepseek-v3.2", "name": "deepseek/deepseek-v3.2" }]
    }
  }
}
```
4. Supports Telegram integration, web search (Tavily), skills, and scheduled tasks

## Best Practices

1. **Use Short-lived Sessions** — Start a new chat/thread for each distinct task. Long sessions waste tokens on repeated context and degrade response quality.
2. **Match Model to Task** — Use `anthropic/claude-sonnet-4.6` or `openai/gpt-5.2` for architecture/complex logic; use `deepseek/deepseek-v3.2` or `anthropic/claude-haiku-4.5` for routine code and simple tasks.
3. **Set Provider Country** — Use `provider: {country: "ru"}` when data must stay in Russia.
4. **Disable Fallbacks** — Use `provider: {allow_fallbacks: false}` when you need the original provider's error.
5. **Monitor Balance** — Use the `/v1/credits` endpoint or the Claude Code status bar script to track spending.
6. **Use Project Rules** — Define agent behavior in `.clinerules/`, `.roo/rules/`, `.rules`, `AGENTS.md`, or `avante.md` for consistent behavior across sessions.
7. **Environment Variables for Keys** — Never hardcode API keys; use env vars or `.env` files.
8. **Separate Keys per Application** — Create different API keys for different apps to isolate costs.

## Common Pitfalls

- **ANTHROPIC_API_KEY must be empty string** for Claude Code — if unset, it connects to Anthropic directly instead of RouterAI.
- **Windows Claude Code** — Run from `cmd.exe`, not PowerShell, for better terminal compatibility. Use full paths to `node.exe` in status bar config.
- **Provider fields `order/only/ignore`** only apply during multi-provider routing, not for direct/static model routes. Use `country` for geo-policy enforcement.
- **403/401 with Master Key** — Master keys only work for `/v1/keys/*` endpoints, not for model inference.
- **Balance exhaustion** — The `/v1/credits` endpoint requires no auth and returns `{credits: number}`. Check before long agent sessions.

## Version Notes

- **OpenAPI spec version:** 1.0.0 (OAS 3.0.0)
- **API base:** `https://routerai.ru/api/v1`
- **Anthropic Messages endpoint:** `POST /v1/messages` (Anthropic-compatible)
- **Responses API:** New recommended interface replacing Chat Completions for advanced apps
