# RouterAI Model Catalog Reference

> Source: https://routerai.ru/models (web) + https://routerai.ru/api/v1/models (API)
> Created: 2026-07-08
> Updated: 2026-07-08

## Overview

RouterAI provides access to 400+ AI models via a single OpenAI-compatible API at `https://routerai.ru/api/v1`. Pricing is in RUB. The full model catalog (IDs, names, context lengths, pricing, supported parameters) is available at `GET https://routerai.ru/api/v1/models` — returns JSON with all models in a single response (no pagination).

**Total models in catalog:** 403
- Text models: 313
- Image generation: 34
- Embeddings: 27
- Video: 16
- Rerank: 3
- Speech: 9
- Transcription: 10
- Audio: 4

**API Base URL:** `https://routerai.ru/api/v1`
**Compatible with:** OpenAI SDK format

## Key Model Families

### Anthropic Claude

| ID | Context | prompt ₽/tok | completion ₽/tok | prompt ₽/1M | completion ₽/1M |
|---|---|---|---|---|---|
| `anthropic/claude-sonnet-5` | 1M | 0.00019792708 | 0.0009896354 | 198 | 989 |
| `anthropic/claude-sonnet-4.6` | 1M | 0.00029689062 | 0.0014844531 | 297 | 1,484 |
| `anthropic/claude-sonnet-4.5` | 1M | 0.00029689062 | 0.0014844531 | 297 | 1,484 |
| `anthropic/claude-sonnet-4` | 1M | 0.00029689062 | 0.0014844531 | 297 | 1,484 |
| `anthropic/claude-opus-4.8` | 1M | 0.0004948177 | 0.0024740885 | 495 | 2,474 |
| `anthropic/claude-opus-4.7` | 1M | 0.0004948177 | 0.0024740885 | 495 | 2,474 |
| `anthropic/claude-opus-4.6` | 1M | 0.0004948177 | 0.0024740885 | 495 | 2,474 |
| `anthropic/claude-opus-4.5` | 200K | 0.0004948177 | 0.0024740885 | 495 | 2,474 |
| `anthropic/claude-opus-4.1` | 200K | 0.0014844531 | 0.0074222655 | 1,484 | 7,422 |
| `anthropic/claude-opus-4` | 200K | 0.0014844531 | 0.0074222655 | 1,484 | 7,422 |
| `anthropic/claude-haiku-4.5` | 200K | 0.00009896354 | 0.0004948177 | 99 | 495 |
| `anthropic/claude-fable-5` | 1M | 0.0009896354 | 0.004948177 | 989 | 4,948 |
| `anthropic/claude-3-haiku` | 200K | 0.000024740885 | 0.000123704425 | 25 | 124 |

### OpenAI GPT

| ID | Context | prompt ₽/1M | completion ₽/1M |
|---|---|---|---|
| `openai/gpt-5.5` | 1M+ | 495 | 2,969 |
| `openai/gpt-5.5-pro` | 1M+ | 3,563 | 21,376 |
| `openai/gpt-5.4` | 1M+ | 247 | 1,484 |
| `openai/gpt-5.4-mini` | 400K | 74 | 445 |
| `openai/gpt-5.4-nano` | 400K | 20 | 124 |
| `openai/gpt-5.3-codex` | 400K | 173 | 1,385 |
| `openai/gpt-5.2-chat` | 128K | 173 | 1,385 |
| `openai/gpt-5.2` | 400K | 173 | 1,385 |
| `openai/gpt-5.1` | 400K | 124 | 989 |
| `openai/gpt-5` | 400K | 124 | 989 |
| `openai/gpt-5-mini` | 400K | 25 | 198 |
| `openai/gpt-5-nano` | 400K | 5 | 40 |
| `openai/gpt-5-pro` | 400K | 1,484 | 11,876 |
| `openai/gpt-4o` | 128K | 247 | 989 |
| `openai/gpt-4o-mini` | 128K | 15 | 59 |
| `openai/gpt-4.1` | 1M | 198 | 792 |
| `openai/gpt-4.1-mini` | 1M | 40 | 158 |
| `openai/gpt-4.1-nano` | 1M | 10 | 40 |
| `openai/o4-mini` | 200K | 109 | 435 |
| `openai/o3` | 200K | 198 | 792 |
| `openai/o3-mini` | 200K | 109 | 435 |
| `openai/o1` | 200K | 1,484 | 5,938 |
| `openai/o1-pro` | 200K | 14,845 | 59,378 |
| `openai/gpt-4-turbo` | 128K | 989 | 2,969 |
| `openai/gpt-4o-mini-search-preview` | 128K | 15 | 59 |
| `openai/gpt-4o-search-preview` | 128K | 247 | 989 |

### Google Gemini

| ID | Context | prompt ₽/1M | completion ₽/1M |
|---|---|---|---|
| `google/gemini-3.5-flash` | 1M | 148 | 891 |
| `google/gemini-3.1-pro-preview` | 1M | 198 | 1,188 |
| `google/gemini-3.1-flash-lite` | 1M | 25 | 148 |
| `google/gemini-3-flash-preview` | 1M | 49 | 297 |
| `google/gemini-2.5-pro` | 1M | 124 | 989 |
| `google/gemini-2.5-flash` | 1M | 30 | 247 |
| `google/gemini-2.5-flash-lite` | 1M | 10 | 40 |

### DeepSeek

| ID | Context | prompt ₽/1M | completion ₽/1M |
|---|---|---|---|
| `deepseek/deepseek-v4-pro` | 1M | 60 | 121 |
| `deepseek/deepseek-v4-flash` | 1M | 9 | 18 |
| `deepseek/deepseek-v3.2` | 131K | 23 | 34 |
| `deepseek/deepseek-v3.1-terminus` | 164K | 27 | 94 |
| `deepseek/deepseek-chat-v3.1` | 164K | 21 | 78 |
| `deepseek/deepseek-r1-0528` | 164K | 49 | 213 |
| `deepseek/deepseek-r1` | 164K | 69 | 247 |

### Qwen (Alibaba)

| ID | Context | prompt ₽/1M | completion ₽/1M |
|---|---|---|---|
| `qwen/qwen3.7-plus` | 1M | 32 | 127 |
| `qwen/qwen3.7-max` | 1M | 124 | 371 |
| `qwen/qwen3.6-plus` | 1M | 32 | 193 |
| `qwen/qwen3.6-flash` | 1M | 19 | 111 |
| `qwen/qwen3.5-plus` | 1M | 26 | 154 |
| `qwen/qwen3.5-flash` | 1M | 6 | 26 |
| `qwen/qwen3-coder-plus` | 1M | 64 | 322 |
| `qwen/qwen3-coder-flash` | 1M | 19 | 96 |
| `qwen/qwen3-plus` | 131K | 26 | 78 |
| `qwen/qwen3-max` | 262K | 77 | 386 |
| `qwen/qwen3-max-thinking` | 262K | 77 | 386 |
| `qwen/qwen3-235b-a22b` | 131K | 45 | 180 |
| `qwen/qwen3-32b` | 131K | 8 | 28 |
| `qwen/qwen3-14b` | 132K | 10 | 24 |
| `qwen/qwen3-8b` | 131K | 12 | 45 |

## API Pricing Response Format

The `GET https://routerai.ru/api/v1/models` endpoint returns pricing per model as RUB per token:

```json
{
  "data": [
    {
      "id": "anthropic/claude-sonnet-4.6",
      "pricing": {
        "prompt": 0.00029689062,
        "completion": 0.0014844531,
        "input_cache_read": 0.000029689062,
        "input_cache_write": 0.000371113275
      },
      "context_length": 1000000,
      "architecture": {
        "modality": "text+image+file->text"
      },
      "supported_parameters": [
        "include_reasoning", "max_tokens", "reasoning", "stop",
        "temperature", "tool_choice", "tools", "top_p"
      ]
    }
  ]
}
```

### Pricing Field Types

- **Token-based models** (`text->text`, `text+image->text`): `prompt` and `completion` in RUB per token
- **Embedding models** (`text->embeddings`): `prompt` only, `completion: 0.0`
- **Rerank models** (`text->rerank`): `search_units` in RUB per search unit (e.g., `0.24740885`)
- **Image generation** (`text+image->image`): `image_output` in RUB per image
- **Audio/Transcription**: `seconds` in RUB per second of audio, `audio` in RUB per token
- **Video generation**: `seconds` in RUB per second of video
- **Gemini models**: extra fields `audio`, `image`, `internal_reasoning`, `web_search`, `input_cache_read`, `input_audio_cache`, `input_cache_write`
- **Image output models** (Gemini, GPT Image): `image_output` in RUB per image

### Cache Pricing

Many models support prompt caching with discounts:

- `input_cache_read`: ~10% of prompt price (cache hit)
- `input_cache_write`: ~80-125% of prompt price (cache creation)
- Cache discounts apply to Claude, GPT, Gemini, Qwen, DeepSeek, Mistral models

## Rerank Models

Dedicated reranking models have per-search-unit pricing (not per-token):

| ID | Context | Price ₽/search unit |
|---|---|---|
| `cohere/rerank-4-pro` | 32K | 0.25 |
| `cohere/rerank-4-fast` | 32K | 0.20 |
| `cohere/rerank-v3.5` | 4K | 0.10 |

## Embedding Models

| ID | Dimensions | Price ₽/1M input |
|---|---|---|
| `sentence-transformers/all-minilm-l6-v2` | 384 | 0.5 |
| `sentence-transformers/all-mpnet-base-v2` | 768 | 0.5 |
| `openai/text-embedding-3-small` | 512-1536 | 2 |
| `openai/text-embedding-3-large` | 256-3072 | 13 |
| `qwen/qwen3-embedding-8b` | varies | 1 |
| `qwen/qwen3-embedding-4b` | varies | 2 |
| `baai/bge-m3` | 1024 | 1 |
| `baai/bge-large-en-v1.5` | 1024 | 1 |
| `baai/bge-base-en-v1.5` | 768 | 0.5 |
| `intfloat/e5-large-v2` | 1024 | 1 |
| `intfloat/e5-base-v2` | 768 | 0.5 |
| `intfloat/multilingual-e5-large` | 1024 | 1 |
| `mistralai/mistral-embed-2312` | 1024 | 10 |
| `mistralai/codestral-embed-2505` | 1024 | 15 |
| `thenlper/gte-base` | 768 | 0.5 |
| `thenlper/gte-large` | 1024 | 1 |
| `perplexity/pplx-embed-v1-4b` | varies | 3 |
| `perplexity/pplx-embed-v1-0.6b` | varies | 0.4 |

## Supported Parameters

Models support varying subsets of these parameters:

- `temperature`, `top_p`, `top_k` — sampling control
- `max_tokens`, `max_completion_tokens` — output length
- `stop` — stop sequences
- `frequency_penalty`, `presence_penalty`, `repetition_penalty` — penalty controls
- `min_p`, `top_a` — advanced sampling (few models)
- `seed` — deterministic output
- `response_format` — JSON mode
- `structured_outputs` — schema-constrained JSON
- `tool_choice`, `tools` — function calling
- `include_reasoning`, `reasoning` — thinking/reasoning mode (DeepSeek, Qwen, Gemini, Claude)
- `logprobs`, `top_logprobs` — token probabilities
- `logit_bias` — token bias
- `web_search_options` — web search (Perplexity, GPT-4o search previews)
- `parallel_tool_calls` — parallel function calling (MiniMax M2.5+)
- `reasoning_effort` — reasoning depth (o3-mini, o4-mini)
- `verbosity` — Claude output verbosity control

## Use Case Selection

### For Coding / SWE Agents
- Claude Opus 4.8, 4.7, 4.6 — strongest coding
- GPT-5.3-Codex, GPT-5.2-Codex — OpenAI coding agents
- DeepSeek V4 Pro / Flash — open source MoE
- Qwen3.6 Plus — strongest Chinese model for coding
- MiniMax M2.7 — 56.2% SWE-Pro

### For Chat / General Assistant
- Claude Sonnet 5, 4.6 — balanced quality/speed
- GPT-5.4, GPT-5.5 — frontier general models
- Gemini 3.5 Flash — fast, near-Pro quality
- DeepSeek V4 Flash — 9 ₽/1M input, very cheap

### For RAG / Retrieval
- Embedding models (see table above)
- Rerank: Cohere Rerank 4 Pro / Fast
- LLM reranking: Claude Haiku 4.5, DeepSeek V4 Flash — cheap prompt-based

### For Budget
- GPT-5 Nano (5/40 ₽/1M)
- DeepSeek V4 Flash (9/18 ₽/1M)
- Gemini 2.5 Flash Lite (10/40 ₽/1M)
- InclusionAI Ling-2.6-flash (~1/3 ₽/1M)
- Qwen Qwen3.5 Flash (7/26 ₽/1M)

## Providers

| Provider | Models |
|---|---|
| Anthropic | Claude Sonnet, Opus, Haiku, Fable |
| OpenAI | GPT-5.x, GPT-4o, o-series, Codex, Image |
| Google | Gemini, Gemma, Veo, Lyria |
| DeepSeek | V4, V3.2, V3.1, R1 |
| Meta Llama | Llama 4 Maverick/Scout, Llama 3.x |
| Mistral AI | Mistral Large/Small/Medium, Ministral, Codestral |
| Qwen (Alibaba) | Qwen3.x, Qwen2.5 |
| MiniMax | M3, M2.7, M2.5, Hailuo |
| Moonshot AI | Kimi K2.7, K2.6, K2.5 |
| xAI | Grok 4.x, Grok Build |
| NVIDIA | Nemotron 3 Super/Ultra/Nano |
| Z AI (GLM) | GLM-5.x, GLM-4.x |
| Perplexity | Sonar, Sonar Pro, Sonar Reasoning |
| Cohere | Command A/R, Rerank |
| ByteDance | Seed 2.x, Seedance |
| Amazon | Nova 2 Lite, Nova Pro |
