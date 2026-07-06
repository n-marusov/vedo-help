# RouterAI Model Catalog Reference

> Source: https://routerai.ru/models, https://routerai.ru/api/v1/models
> Created: 2026-07-06
> Updated: 2026-07-06

## Overview

RouterAI provides access to 401+ AI models from 78 providers (developers) via a unified OpenAI-compatible API. The catalog spans text LLMs, vision models, embeddings, audio/speech, rerankers, image generation, and video generation — all billed in RUB with local payment methods.

**API base:** `https://routerai.ru/api/v1`

**Model ID format:** `provider/model-name` (e.g. `anthropic/claude-sonnet-4.6`, `openai/gpt-5.2`, `deepseek/deepseek-v4-flash`)

**Pricing:** All prices below are in **RUB per 1M tokens** (prompt/completion) unless noted otherwise. Pricing is converted from per-token values returned by the API. Cache-read pricing is listed where available.

### Catalog Categories

| Category | Count | Description |
|----------|-------|-------------|
| Text | 311 | LLMs for chat, completion, reasoning |
| Images | 34 | Image generation & editing |
| Embeddings | 27 | Text/code embedding models |
| Audio | 4 | Speech recognition (ASR) |
| Video | 16 | Video generation |
| Speech | 9 | Text-to-speech (TTS) |
| Transcription | 10 | Audio-to-text |
| Rerank | 3 | Document re-ranking |

---

## Premium Frontier Models (Top Tier Coding & Reasoning)

These are the most capable models for complex software engineering, deep reasoning, and agentic workflows. Prices are per 1M tokens (prompt / completion) in RUB.

### Anthropic Claude

| Model ID | Context | Prompt ₽ | Completion ₽ | Key Features |
|----------|---------|-----------|---------------|--------------|
| `anthropic/claude-sonnet-5` | 1M | 200 | 1,004 | Best Sonnet tier, adaptive reasoning (low/med/high/max/xhigh) |
| `anthropic/claude-sonnet-4.6` | 1M | 301 | 1,506 | Strong coding & agentic, verbosity control |
| `anthropic/claude-sonnet-4.5` | 1M | 301 | 1,506 | Advanced Sonnet, SWE-bench leader |
| `anthropic/claude-sonnet-4` | 1M | 301 | 1,506 | 72.7% SWE-bench, 1M context |
| `anthropic/claude-opus-4.8` | 1M | 502 | 2,510 | Most powerful Opus, 1M ctx |
| `anthropic/claude-opus-4.7` | 1M | 502 | 2,510 | Async agent, long-horizon tasks |
| `anthropic/claude-opus-4.6` | 1M | 502 | 2,510 | Most powerful coding Opus |
| `anthropic/claude-opus-4.5` | 200K | 502 | 2,510 | Optimized for complex SWE |
| `anthropic/claude-opus-4.1` | 200K | 1,506 | 7,530 | 74.5% SWE-bench |
| `anthropic/claude-opus-4` | 200K | 1,506 | 7,530 | 72.5% SWE-bench |
| `anthropic/claude-haiku-4.5` | 200K | 100 | 502 | Fast, cheap, 73%+ SWE-bench |
| `anthropic/claude-fable-5` | 1M | 1,004 | 5,020 | Mythos-tier, autonomous knowledge work |
| `anthropic/claude-3-haiku` | 200K | 25 | 125 | Legacy fast model |

**Cache pricing:** Cache read is ~10% of prompt price. Cache write is ~80-125% of prompt price.

### OpenAI GPT

| Model ID | Context | Prompt ₽ | Completion ₽ | Key Features |
|----------|---------|-----------|---------------|--------------|
| `openai/gpt-5.5` | 1M+ | 502 | 3,012 | Newest frontier, 1M+ ctx, coding & multimodal |
| `openai/gpt-5.5-pro` | 1M+ | 3,614 | 21,685 | Deep analysis, precision, 1M+ ctx |
| `openai/gpt-5.4` | 1M+ | 251 | 1,506 | Unified Codex+GPT, 1M+ ctx |
| `openai/gpt-5.4-pro` | 1M+ | 3,614 | 21,685 | Most advanced reasoning |
| `openai/gpt-5.4-mini` | 400K | 75 | 452 | Balanced efficiency |
| `openai/gpt-5.4-nano` | 400K | 20 | 125 | Fast, cheap sub-agent |
| `openai/gpt-5.3-codex` | 400K | 176 | 1,406 | Advanced coding, SWE-Bench Pro |
| `openai/gpt-5.3-chat` | 128K | 176 | 1,406 | Smoother conversations |
| `openai/gpt-5.2` | 400K | 176 | 1,406 | Latest frontier, agentic |
| `openai/gpt-5.2-pro` | 400K | 2,108 | 16,866 | Premium reasoning |
| `openai/gpt-5.2-codex` | 400K | 176 | 1,406 | Coding-optimized |
| `openai/gpt-5.2-chat` | 128K | 176 | 1,406 | Low-latency chat |
| `openai/gpt-5.1` | 400K | 125 | 1,004 | Adaptive reasoning, improved instruction following |
| `openai/gpt-5.1-codex` | 400K | 125 | 1,004 | SWE-optimized |
| `openai/gpt-5.1-codex-mini` | 400K | 25 | 201 | Faster, cheaper coding |
| `openai/gpt-5.1-chat` | 128K | 125 | 1,004 | Fast chat variant |
| `openai/gpt-5-codex` | 400K | 125 | 1,004 | Coding agent model |
| `openai/gpt-5` | 400K | 125 | 1,004 | Base frontier model |
| `openai/gpt-5-mini` | 400K | 25 | 201 | Compact reasoning |
| `openai/gpt-5-nano` | 400K | 5 | 40 | Ultra-fast, ultra-cheap |
| `openai/gpt-5-pro` | 400K | 1,506 | 12,047 | Premium tier |
| `openai/gpt-5-chat` | 128K | 125 | 1,004 | Chat-optimized |
| `openai/gpt-4.1` | 1M | 201 | 803 | 1M ctx, 54.6% SWE-bench |
| `openai/gpt-4.1-mini` | 1M | 40 | 161 | Efficient, 1M ctx |
| `openai/gpt-4.1-nano` | 1M | 10 | 40 | Cheapest 1M ctx model |
| `openai/o3` | 200K | 201 | 803 | Strong reasoning, STEM |
| `openai/o3-mini` | 200K | 110 | 442 | Cost-efficient reasoning |
| `openai/o4-mini` | 200K | 110 | 442 | Compact reasoning, 99.5% AIME |
| `openai/o1` | 200K | 1,506 | 6,024 | Legacy reasoning |
| `openai/o1-pro` | 200K | 15,059 | 60,237 | Premium reasoning |
| `openai/o3-deep-research` | 200K | 1,004 | 4,016 | Multi-step research |
| `openai/gpt-4o` | 128K | 251 | 1,004 | Legacy multimodal |

**Cache:** Cache read is ~10% of prompt price.

### Google Gemini

| Model ID | Context | Prompt ₽ | Completion ₽ | Key Features |
|----------|---------|-----------|---------------|--------------|
| `google/gemini-3.5-flash` | 1M | 151 | 904 | Near-Pro coding at Flash speed |
| `google/gemini-3.1-pro-preview` | 1M | 201 | 1,205 | 1M ctx, SWE improvements |
| `google/gemini-3.1-flash-lite` | 1M | 25 | 151 | Half the cost of 3 Flash, full thinking levels |
| `google/gemini-3-flash-preview` | 1M | 50 | 301 | High speed, near-Pro reasoning |
| `google/gemini-2.5-pro` | 1M | 125 | 1,004 | Top ranked, 1M ctx, multimodal |
| `google/gemini-2.5-flash` | 1M | 30 | 251 | Strong reasoning, fast |
| `google/gemini-2.5-flash-lite` | 1M | 10 | 40 | Ultra cheap, 1M ctx |

**Multimodal pricing:** Image input costs 0.03-0.13 ₽/1K tokens extra. Audio input costs 0.03-0.13 ₽/1K tokens extra.

### DeepSeek

| Model ID | Context | Prompt ₽ | Completion ₽ | Key Features |
|----------|---------|-----------|---------------|--------------|
| `deepseek/deepseek-v4-pro` | 1M | 61 | 122 | 1.6T params, 49B active, 1M ctx |
| `deepseek/deepseek-v4-flash` | 1M | 9 | 18 | 284B params, 13B active, efficient |
| `deepseek/deepseek-v3.2` | 131K | 23 | 34 | SOTA open, IMO gold level |
| `deepseek/deepseek-v3.1-terminus` | 164K | 27 | 95 | Improved V3.1, agent fix |
| `deepseek/deepseek-chat-v3.1` | 164K | 21 | 79 | Hybrid reasoning (thinking/non-thinking) |
| `deepseek/deepseek-r1-0528` | 164K | 50 | 216 | Open reasoning model |
| `deepseek/deepseek-r1` | 164K | 70 | 251 | Original open reasoning |

**Cache:** Cache read is ~50-65% of prompt price for V3/V4 models.

---

## Best Value Models (Balanced Price/Performance)

These models offer strong capabilities at significantly lower prices than the frontier tier.

| Model ID | Context | Prompt ₽ | Completion ₽ | Notes |
|----------|---------|-----------|---------------|-------|
| `qwen/qwen3.7-plus` | 1M | 32 | 129 | Mid-range Qwen, 1M ctx, multimodal |
| `qwen/qwen3.6-plus` | 1M | 33 | 196 | Strong coding, 1M ctx |
| `qwen/qwen3.6-flash` | 1M | 19 | 113 | Fast Qwen, 1M ctx |
| `qwen/qwen3.5-plus` | 1M | 26 | 157 | Good all-rounder, 1M ctx |
| `qwen/qwen3.5-flash` | 1M | 7 | 26 | Very cheap, 1M ctx |
| `qwen/qwen3-coder-plus` | 1M | 65 | 326 | Proprietary coding model |
| `qwen/qwen3-coder-flash` | 1M | 20 | 98 | Fast coder |
| `qwen/qwen3-coder` | 1M | 30 | 100 | Open source 480B MoE coder |
| `qwen/qwen3-plus` | 1M | 26 | 78 | Balanced performance |
| `qwen/qwen3-max` | 262K | 78 | 392 | Top open Qwen |
| `qwen/qwen3-max-thinking` | 262K | 78 | 392 | Reasoning variant |
| `qwen/qwen3-235b-a22b` | 131K | 46 | 183 | Strong open MoE |
| `qwen/qwen3.7-max` | 1M | 125 | 376 | Flagship, agent-optimized |
| `google/gemma-4-31b-it` | 262K | 12 | 35 | Apache 2.0, multimodal (image+video) |
| `google/gemma-4-26b-a4b-it` | 262K | 6 | 33 | MoE, efficient, Apache 2.0 |
| `meta-llama/llama-4-maverick` | 1M | 15 | 60 | 400B MoE, 17B active, 1M ctx |
| `meta-llama/llama-4-scout` | 10M | 10 | 30 | 10M context window! |
| `meta-llama/llama-3.3-70b-instruct` | 131K | 10 | 32 | Reliable open model |
| `mistralai/mistral-large-3-2512` | 262K | 50 | 151 | 675B MoE, Apache 2.0 |
| `mistralai/mistral-medium-3.5` | 262K | 151 | 753 | 128B dense, open weights |
| `mistralai/mistral-small-4` | 262K | 15 | 60 | Multi-capability small model |
| `mistralai/mistral-small-3.2-24b` | 128K | 8 | 20 | 24B, image input |
| `mistralai/codestral-2508` | 256K | 30 | 90 | Code-specialized |
| `z-ai/glm-5.2` | 1M | 69 | 216 | 1M ctx, reasoning (high/xhigh) |
| `z-ai/glm-5` | 203K | 60 | 193 | Open, system design |
| `z-ai/glm-4.7` | 203K | 40 | 176 | Improved coding |
| `minimax/minimax-m3` | 1M | 30 | 120 | Multimodal (text+image+video), 1M ctx |
| `minimax/minimax-m2.7` | 205K | 18 | 72 | Agent-optimized, multi-user |
| `minimax/minimax-m2.5` | 205K | 12 | 48 | 80.2% SWE-Bench |
| `minimax/minimax-m2.1` | 205K | 30 | 120 | Lightweight, 10B active |
| `cohere/command-a` | 256K | 251 | 1,004 | Open weights, agentic |
| `nvidia/nemotron-3-super` | 1M | 9 | 40 | 120B MoE, 12B active, 1M ctx |
| `nvidia/nemotron-3-ultra` | 1M | 50 | 221 | 550B MoE, orchestrator |
| `inception/mercury-2` | 128K | 25 | 75 | Diffusion LLM, 1000+ tok/s |
| `stepfun/step-3.7-flash` | 256K | 20 | 115 | Multimodal MoE |
| `bytedance-seed/seed-2.0-mini` | 262K | 10 | 40 | Fast, 4 reasoning levels |
| `cohere/command-r-08-2024` | 128K | 15 | 60 | Good for RAG & tools |
| `poolside/laguna-xs-2.1` | 262K | 6 | 12 | Coding agent, 33B, very cheap |
| `poolside/laguna-m.1` | 262K | 20 | 40 | Flagship coding agent |
| `xiaomi/mimo-v2.5-pro` | 1M | 66 | 131 | Flagship, 1M ctx |
| `xiaomi/mimo-v2.5` | 1M | 11 | 28 | Omni-modal, cheap, 1M ctx |
| `nex-agi/nex-n2-pro` | 262K | 25 | 100 | Agent MoE, coding |

---

## Budget & Open Models (Low Cost)

Ideal for high-volume, simple tasks, classification, extraction, and sub-agent roles.

| Model ID | Context | Prompt ₽ | Completion ₽ | Parameters |
|----------|---------|-----------|---------------|------------|
| `qwen/qwen3-8b` | 131K | 12 | 46 | 8B dense |
| `qwen/qwen3-14b` | 132K | 10 | 24 | 14B dense |
| `qwen/qwen3-32b` | 131K | 8 | 28 | 32B dense |
| `qwen/qwen3-30b-a3b-instruct` | 131K | 5 | 19 | 30B MoE, 3B active |
| `qwen/qwen3-235b-a22b-2507` | 262K | 9 | 10 | 235B MoE, very cheap output! |
| `qwen/qwen3-coder-30b-a3b-instruct` | 160K | 7 | 27 | Coding MoE |
| `qwen/qwen3.5-9b` | 262K | 10 | 15 | Multimodal 9B |
| `qwen/qwen3-vl-8b-instruct` | 256K | 12 | 46 | Vision-Language 8B |
| `qwen/qwen3-embedding-8b` | 32K | 1 | 0 | Embeddings only |
| `qwen/qwen3-embedding-4b` | 33K | 2 | 0 | Smaller embeddings |
| `google/gemma-3-27b-it` | 131K | 8 | 16 | Open, Apache 2.0 |
| `google/gemma-3-12b-it` | 131K | 5 | 15 | Mid-range |
| `google/gemma-3-4b-it` | 131K | 5 | 10 | Tiny |
| `google/gemma-3n-4b-it` | 33K | 6 | 12 | Mobile-optimized |
| `meta-llama/llama-3.1-8b-instruct` | 131K | 2 | 5 | Classic small model |
| `meta-llama/llama-3.1-70b-instruct` | 131K | 40 | 40 | 70B, solid |
| `meta-llama/llama-3.2-3b-instruct` | 131K | 5 | 34 | Tiny but capable |
| `meta-llama/llama-3.2-1b-instruct` | 131K | 3 | 20 | Ultra-tiny |
| `mistralai/mistral-small-3` | 33K | 5 | 8 | 24B, Apache 2.0 |
| `mistralai/mistral-nemo` | 131K | 2 | 3 | 12B, multilingual |
| `mistralai/ministral-3-8b-2512` | 262K | 15 | 15 | 8B, image input |
| `mistralai/ministral-3-3b-2512` | 131K | 10 | 10 | 3B, image input |
| `openai/gpt-oss-120b` | 131K | 4 | 14 | Open weights MoE, 120B |
| `openai/gpt-oss-20b` | 131K | 3 | 13 | Open 20B, Apache 2.0 |
| `microsoft/phi-4` | 16K | 7 | 14 | 14B, reasoning |
| `cohere/command-r7b` | 128K | 4 | 15 | Tiny, fast, RAG |
| `amazon/nova-micro-v1` | 128K | 4 | 14 | Fast text only |
| `amazon/nova-lite-v1` | 300K | 6 | 24 | Multimodal |
| `nvidia/nemotron-3-nano-30b-a3b` | 262K | 5 | 20 | Open MoE |
| `liquid/lfm-2-24b-a2b` | 128K | 3 | 12 | 2B active, runs on laptop |
| `inclusionai/ling-2.6-flash` | 262K | 1 | 3 | Ultra-cheap |
| `inclusionai/ling-2.6-1t` | 262K | 8 | 63 | 1T params, fast thinking |
| `inclusionai/ring-2.6-1t` | 262K | 8 | 63 | Thinking variant |
| `arcee-ai/trinity-mini` | 131K | 5 | 15 | 26B MoE, 3B active |

---

## Embedding Models

For vector search, RAG, and semantic similarity. Prices are per 1M prompt tokens (completion is 0).

| Model ID | Dimensions | Context | Price ₽/1M | Notes |
|----------|-----------|---------|-------------|-------|
| `openai/text-embedding-3-small` | 512-1536 | 8K | 2 | Efficient, adjustable dims |
| `openai/text-embedding-3-large` | 256-3072 | 8K | 13 | Highest quality |
| `qwen/qwen3-embedding-8b` | varies | 32K | 1 | Multilingual, long ctx |
| `qwen/qwen3-embedding-4b` | varies | 33K | 2 | Smaller, cheaper |
| `google/gemini-embedding-001` | 768 | 20K | 15 | Top MTEB ranking |
| `google/gemini-embedding-2` | 128-3072 | 8K | 20 | Multimodal (text+image) |
| `google/gemini-embedding-2-preview` | 128-3072 | 8K | 20 | Preview of v2 |
| `mistralai/mistral-embed-2312` | 1024 | 8K | 10 | Mistral quality |
| `mistralai/codestral-embed-2505` | 1024 | 8K | 15 | Code-optimized |
| `baai/bge-m3` | 1024 | 8K | 1 | Multilingual, open |
| `baai/bge-large-en-v1.5` | 1024 | 8K | 1 | English optimized |
| `baai/bge-base-en-v1.5` | 768 | 8K | 0.5 | Lightweight English |
| `intfloat/e5-large-v2` | 1024 | 8K | 1 | Strong search |
| `intfloat/e5-base-v2` | 768 | 8K | 0.5 | Balanced |
| `intfloat/multilingual-e5-large` | 1024 | 8K | 1 | 90+ languages |
| `sentence-transformers/all-minilm-l6-v2` | 384 | 8K | 0.5 | Classic, tiny |
| `sentence-transformers/all-mpnet-base-v2` | 768 | 8K | 0.5 | Reliable |
| `thenlper/gte-base` | 768 | 8K | 0.5 | Efficient |
| `thenlper/gte-large` | 1024 | 8K | 1 | High quality |
| `perplexity/pplx-embed-v1-4b` | varies | 32K | 3 | Web-scale search |
| `perplexity/pplx-embed-v1-0.6b` | varies | 32K | 0.4 | Ultra-cheap |

---

## Image Generation Models

| Model ID | Type | Price | Notes |
|----------|------|-------|-------|
| `openai/gpt-image-2` | text+image→image | 0.80 ₽ prompt + 3.01 ₽ image | Latest, high quality |
| `openai/gpt-image-1` | text+image→image | 1.00 ₽ prompt + 4.02 ₽ image | Precise text rendering |
| `openai/gpt-image-1-mini` | text+image→image | 0.25 ₽ prompt + 0.80 ₽ image | Cheap variant |
| `openai/gpt-5-image` | text+image→text+image | 1,004 ₽/1M tokens + 4.02 ₽ image | Multimodal reasoning + image |
| `openai/gpt-5-image-mini` | text+image→text+image | 251 ₽/1M tokens + 0.80 ₽ image | Efficient multimodal |
| `google/gemini-3.1-flash-image` | text+image→text+image | 6.02 ₽/image | Flash speed, Pro quality |
| `google/gemini-3-pro-image` | text+image→text+image | 12.05 ₽/image | Best Google quality |
| `black-forest-labs/flux.2-pro` | text+image→image | 0.0007 ₽/MP | Per-megapixel billing |
| `black-forest-labs/flux.2-max` | text+image→image | 0.0017 ₽/MP | Best quality FLUX |
| `black-forest-labs/flux.2-klein-4b` | text+image→image | 0.0003 ₽/MP | Fastest FLUX |

---

## Speech & Audio

### Text-to-Speech (TTS)

| Model ID | Languages | Price ₽/char | Notes |
|----------|-----------|--------------|-------|
| `openai/gpt-audio` | multi | 6.43 ₽/1M audio out | GPT-powered speech |
| `openai/gpt-audio-mini` | multi | 0.24 ₽/1M audio out | Cheap GPT audio |
| `google/gemini-3.1-flash-tts-preview` | 70+ | 0.10 ₽ prompt + 2.01 ₽ completion | 200+ audio tags |
| `x-ai/grok-voice-tts-1.0` | 20+ | 1.51 ₽/1M chars | 5 voices |
| `mistralai/voxtral-mini-tts` | multi | 1.61 ₽/1M chars | Voice cloning |
| `microsoft/mai-voice-2` | 10+ | 2.21 ₽/1M chars | SSML styles |

### Transcription (STT / ASR)

| Model ID | Languages | Price | Notes |
|----------|-----------|-------|-------|
| `openai/gpt-4o-transcribe` | multi | 251 ₽/1M in + 1,004 ₽/1M out | High quality |
| `openai/gpt-4o-mini-transcribe` | multi | 125 ₽/1M in + 502 ₽/1M out | Cheaper |
| `openai/whisper-large-v3-turbo` | 99+ | 0.0011 ₽/sec | 216x realtime |
| `openai/whisper-1` | 50+ | 0.010 ₽/sec | Legacy |
| `qwen/qwen3-asr-flash` | 11 | 0.0035 ₽/sec | Music/noise resilient |
| `mistralai/voxtral-mini-transcribe` | multi | 0.005 ₽/sec | Mistral quality |
| `google/chirp-3` | 24 GA + 77 preview | 0.027 ₽/sec | Google accuracy |
| `nvidia/parakeet-tdt-0.6b-v3` | EU langs | 0.003 ₽/sec | 6.34% WER |

---

## Video Generation

| Model ID | Max Duration | Max Resolution | Price ₽/sec | Notes |
|----------|-------------|----------------|-------------|-------|
| `openai/sora-2-pro` | 20s | 1080p | 30.12 | Production quality, audio |
| `google/veo-3.1` | 8s | 4K | 40.16 | Best Google quality, audio |
| `google/veo-3.1-fast` | 8s | 4K | 10.04 | Fast variant |
| `google/veo-3.1-lite` | 8s | 1080p | 5.02 | Cheapest Google |
| `alibaba/wan-2.7` | 10s | 1080p | 10.04 | Good quality |
| `alibaba/wan-2.6` | 10s | 1080p | 4.02 | Latest Alibaba |
| `bytedance/seedance-2.0` | 15s | 4K | 15.18 | High-end ByteDance |
| `bytedance/seedance-2.0-fast` | 15s | 720p | 12.14 | Fast ByteDance |
| `minimax/hailuo-2.3` | 10s | 1080p | 8.20 | Realistic motion |
| `kwaivgi/kling-v3.0-pro` | 15s | 720p | 16.87 | Premium Kuaishou |
| `kwaivgi/kling-v3.0-std` | 15s | 720p | 12.65 | Standard Kuaishou |
| `x-ai/grok-imagine-video` | 15s | 720p | 5.02 | Fast xAI video |

---

## Rerank Models

| Model ID | Context | Price | Notes |
|----------|---------|-------|-------|
| `cohere/rerank-4-pro` | 32K | 0.25 ₽/search unit | Best quality, 100+ languages |
| `cohere/rerank-4-fast` | 32K | 0.20 ₽/search unit | Low latency |
| `cohere/rerank-v3.5` | 4K | 0.10 ₽/search unit | Legacy |

---

## Reasoning & Thinking Mode Support

Many models support configurable reasoning effort via the `reasoning` parameter. Options vary by model:

| Effort Level | Description |
|-------------|-------------|
| `none` / `disabled` | Direct answer, no chain-of-thought |
| `low` / `minimal` | Light reasoning for simple tasks |
| `medium` | Balanced reasoning (default for many) |
| `high` | Deep reasoning for complex problems |
| `xhigh` / `max` | Maximum reasoning budget (Claude, DeepSeek, GLM) |

**Key models with reasoning support:**
- **OpenAI GPT-5.x series:** `reasoning` parameter with effort levels
- **Anthropic Claude:** `reasoning` + `verbosity` (low/medium/high/max/xhigh for Sonnet 5)
- **DeepSeek V3/V4:** `reasoning` boolean toggle + `reasoning_effort`
- **Google Gemini:** `reasoning` parameter with effort levels
- **Qwen3:** `reasoning` boolean toggle for thinking mode
- **GLM 4.5+:** `reasoning` boolean toggle
- **Nous Hermes 4:** `reasoning` boolean for `<think>` blocks

---

## Context Window Size Distribution

| Size | Example Models |
|------|---------------|
| **10M** | `llama-4-scout` |
| **1M+** | `claude-sonnet-4+`, `gpt-5.4+`, `gpt-5.5`, `gemini-3*`, `deepseek-v4*`, `qwen-*-plus`, `qwen3.6-*`, `minimax-m3`, `glm-5.2`, `command-a`, `llama-4-maverick`, `nemotron-3-*`, `xiaomi-mimo-*` |
| **400K** | `gpt-5*` (non-Pro), `gpt-4.1*` |
| **256-300K** | `qwen3-coder`, `qwen3-235b*`, `codestral`, `laguna`, `poolside`, `kimi-k2*`, `glm-4.7`, `mistral-large-3`, `gemma-4` |
| **200K** | `claude-opus-4*`, `claude-sonnet-4`, `claude-haiku`, `o1`, `o3`, `o4` |
| **131K** | `qwen3-*` (most), `llama-3*`, `mistral-medium*`, `gemma-3`, `deepseek-v3*` |
| **128K** | `gpt-4o`, `gpt-4.1-chat`, `mistral-small-3.2` |
| **65K** | `mixtral`, `gemini-3-pro-image`, `olmo-3` |
| **32K** | `mistral-small-3`, `gemma-3n`, `qwen3-embedding` |

---

## Use Case Selection Guide

### For Coding / SWE Agents
| Priority | Recommended Models |
|----------|-------------------|
| Best quality | `claude-sonnet-5`, `claude-opus-4.8`, `gpt-5.5`, `gpt-5.4`, `gpt-5.3-codex` |
| Best open | `deepseek-v4-pro`, `qwen3.7-max`, `kimi-k2.6`, `glm-5.2`, `mistral-large-3` |
| Best value | `deepseek-v4-flash`, `qwen3-coder-plus`, `qwen3.6-plus`, `poolside/laguna-m.1` |
| Cheap coding | `poolside/laguna-xs-2.1`, `qwen3-coder-flash`, `codestral` |
| Sub-agent | `gpt-5.4-nano`, `gpt-oss-20b`, `ministral-3-3b` |

### For Chat / General Assistant
| Priority | Recommended Models |
|----------|-------------------|
| Best quality | `claude-sonnet-5`, `gpt-5.4`, `gemini-3.1-pro-preview` |
| Balanced | `gpt-5.2-chat`, `claude-sonnet-4.6`, `gemini-3-flash-preview` |
| Budget | `qwen3.5-flash`, `gemma-4-26b-a4b-it`, `llama-4-maverick` |

### For RAG / Retrieval
| Component | Recommended Models |
|-----------|-------------------|
| Embedding | `text-embedding-3-large` (quality), `qwen3-embedding-8b` (multilingual/cheap), `bge-m3` (open) |
| Reranking | `cohere/rerank-4-pro` |
| Generation | `gpt-4.1-mini` (1M ctx), `command-r-08-2024` (RAG-optimized), `claude-sonnet-4` (1M ctx) |

### For Vision / Image Analysis
| Priority | Recommended Models |
|----------|-------------------|
| Best quality | `gpt-5.4`, `claude-sonnet-5`, `gemini-3.1-pro-preview` |
| Good value | `qwen3-vl-235b-a22b`, `gemini-3-flash-preview`, `mistral-small-3.2` |
| Lightweight | `qwen3-vl-8b-instruct`, `ministral-3-8b` |

### For Tool Calling / Function Calling
| Priority | Recommended Models |
|----------|-------------------|
| Best | `claude-sonnet-5`, `gpt-5.4`, `gemini-3.1-pro-preview-customtools` |
| Optimized | `command-a`, `command-r-08-2024`, `kimi-k2.6` |
| Open | `qwen3.7-max`, `deepseek-v3.2`, `mistral-large-3`, `glm-5.2` |

---

## Input / Output Modalities

| Modality Pattern | Example Models | Description |
|-----------------|----------------|-------------|
| `text→text` | Most LLMs | Classic text in, text out |
| `text+image→text` | GPT-4o, Claude, Gemini, Qwen-VL | Image understanding |
| `text+image+file→text` | Claude, GPT-4.1+, GPT-5 | PDF, code files, images |
| `text+image+file+audio+video→text` | Gemini 2.5+ | Full multimodal |
| `text+image→text+image` | GPT-5 Image, Nano Banana | Generate images |
| `text+audio→text+audio` | GPT Audio | Voice conversation |
| `text→speech` | TTS models | Text-to-speech |
| `audio→transcription` | Whisper, Chirp | Speech-to-text |
| `text→embeddings` | Embedding models | Vector representations |
| `text→video` | Sora 2 Pro | Video generation |

---

## Supported Parameters Reference

Commonly supported parameters across text models:

| Parameter | Type | Range | Default | Description |
|-----------|------|-------|---------|-------------|
| `temperature` | float | 0.0–2.0 | 1.0 | Response randomness |
| `top_p` | float | 0.0–1.0 | 1.0 | Nucleus sampling |
| `top_k` | integer | 0+ | 0 (off) | Top-K token selection |
| `max_tokens` | integer | 1+ | varies | Max output tokens |
| `max_completion_tokens` | integer | 1+ | varies | Alternative max tokens field |
| `frequency_penalty` | float | -2.0–2.0 | 0.0 | Penalize frequent tokens |
| `presence_penalty` | float | -2.0–2.0 | 0.0 | Penalize repeated topics |
| `repetition_penalty` | float | 0.0–2.0 | 1.0 | Reduce token repetition |
| `stop` | array | — | — | Stop sequences |
| `seed` | integer | — | — | Deterministic output |
| `response_format` | map | — | — | `json_object` or `json_schema` |
| `structured_outputs` | boolean | — | false | Ensure schema adherence |
| `tools` | array | — | — | Tool/function definitions |
| `tool_choice` | string/object | — | — | Tool selection mode |
| `reasoning` | bool/object | — | — | Enable thinking mode |
| `include_reasoning` | boolean | — | — | Return reasoning in response |
| `min_p` | float | 0.0–1.0 | 0.0 | Min probability threshold |
| `logprobs` | boolean | — | false | Return log probabilities |
| `top_logprobs` | integer | 0–20 | — | Top-N logprobs |
| `logit_bias` | map | -100–100 | — | Token bias map |
| `web_search_options` | object | — | — | Enable web grounding |
| `parallel_tool_calls` | boolean | — | true | Parallel function calling |
| `verbosity` | enum | low/med/high | medium | Response effort (Claude) |
| `reasoning_effort` | enum | varies | — | Reasoning budget level |

**Note:** The exact set of supported parameters varies per model. Check the API response for `supported_parameters` on each model to know what's available.

---

## Providers (Developers)

| Provider | Slug | Notable Models |
|----------|------|---------------|
| Anthropic | `anthropic` | Claude Sonnet, Opus, Haiku, Fable |
| OpenAI | `openai` | GPT-4/5 series, o1/o3/o4, GPT Image |
| Google | `google` | Gemini 2.5/3.x, Gemma 3/4, Gemma 3n, Veo |
| DeepSeek | `deepseek` | V3, V3.1, V3.2, V4, R1 |
| Alibaba (Qwen) | `qwen` | Qwen3, Qwen3.5, Qwen3.6, Qwen3.7, Qwen-VL |
| Meta | `meta-llama` | Llama 3.1/3.2/3.3/4, Llama Guard |
| Mistral AI | `mistralai` | Mistral Large, Medium, Small, Codestral |
| Z.ai | `z-ai` | GLM 4.5/4.6/4.7/5/5.1/5.2, GLM-4V/5V |
| MiniMax | `minimax` | MiniMax M1/M2/M2.1/M2.5/M2.7/M3 |
| Moonshot AI | `moonshotai` | Kimi K2/K2.5/K2.6/K2.7 |
| ByteDance | `bytedance` / `bytedance-seed` | Seed 1.6/2.0, Seedance, UI-TARS |
| Cohere | `cohere` | Command R/R+, Rerank 4 |
| NVIDIA | `nvidia` | Nemotron 3 Nano/Super/Ultra, Parakeet |
| Mistral (code) | `mistralai` | Codestral, Devstral |
| xAI | `x-ai` | Grok 4.20/4.3, Grok Imagine |
| Perplexity | `perplexity` | Sonar, Sonar Pro, Sonar Deep Research |
| Microsoft | `microsoft` | Phi 4, MAI-Voice, MAI-Image |
| Amazon | `amazon` | Nova Lite/Micro/Pro/Premier |
| Tencent | `tencent` | Hunyuan A13B |
| Baidu | `baidu` | ERNIE 4.5 VL |
| StepFun | `stepfun` | Step 3.5/3.7 Flash |
| Xiaomi | `xiaomi` | MiMo V2.5 |
| Yandex | `yandex` | YandexGPT Pro 5/5.1, Lite 5 |
| Writer | `writer` | Palmyra X5 |
| IBM | `ibm-granite` | Granite 4.0/4.1 |
| Poolside | `poolside` | Laguna XS/M |
| Black Forest Labs | `black-forest-labs` | FLUX.2 Pro/Max/Flex/Klein |
| Recraft | `recraft` | Recraft V4/V4.1 image gen |
