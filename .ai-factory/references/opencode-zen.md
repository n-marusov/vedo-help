# OpenCode Zen Reference

> Source: https://opencode.ai/docs/ru/zen/
> Created: 2026-07-10
> Updated: 2026-07-10

## Overview

OpenCode Zen is a curated list of tested and verified AI models provided by the OpenCode team. It acts as an AI gateway and an optional provider within OpenCode, giving users access to a selection of models (from various providers) that have been specifically tested for coding agent performance. It operates on a pay-as-you-go model and allows access via the OpenCode TUI or API endpoints.

## Core Concepts

**Curated Models**: A selection of models tested and verified for optimal performance in coding tasks.
**Provider Agnostic**: Zen acts as a gateway, routing requests to the best combinations of models and providers.
**Pay-As-You-Go**: Users are charged per request based on token usage, with the ability to top up account balances.
**Auto-Top-Up**: Automatically reloads the balance (e.g., by $20) when it falls below a threshold ($5), though this can be configured or disabled.
**Workspaces & Teams**: Supports team collaboration with roles (Admin, Member), model access control, and usage limits.
**Bring Your Own Key (BYOK)**: Users can use their own OpenAI or Anthropic API keys while still accessing other models through Zen.

## API / Interface

Models can be accessed via specific API endpoints. The model ID format in the OpenCode config is `opencode/<model-id>`.

Available endpoints based on model provider types:
- `https://opencode.ai/zen/v1/responses` (OpenAI format)
- `https://opencode.ai/zen/v1/messages` (Anthropic format)
- `https://opencode.ai/zen/v1/models/<model-id>` (Google Gemini format)
- `https://opencode.ai/zen/v1/chat/completions` (OpenAI Compatible format)
- Full model list metadata: `https://opencode.ai/zen/v1/models`

### Supported Models & Endpoints

| Model | Model ID | Endpoint | AI SDK Package |
| --- | --- | --- | --- |
| GPT 5.5 / 5.4 / 5.3 / 5.2 / 5.1 / 5 (inc. Pro/Mini/Nano/Codex) | e.g., `gpt-5.5` | `https://opencode.ai/zen/v1/responses` | `@ai-sdk/openai` |
| Claude Fable/Opus/Sonnet/Haiku (various versions) | e.g., `claude-sonnet-5` | `https://opencode.ai/zen/v1/messages` | `@ai-sdk/anthropic` |
| Gemini 3.5 / 3.1 / 3 (Flash/Pro) | e.g., `gemini-3.5-flash` | `https://opencode.ai/zen/v1/models/<model-id>` | `@ai-sdk/google` |
| Qwen3.7 / 3.6 / 3.5 (Max/Plus) | e.g., `qwen3.7-max` | `https://opencode.ai/zen/v1/messages` | `@ai-sdk/anthropic` |
| DeepSeek V4 (Pro/Flash) | e.g., `deepseek-v4-pro` | `https://opencode.ai/zen/v1/chat/completions` | `@ai-sdk/openai-compatible` |
| MiniMax, GLM, Kimi, Grok, Big Pickle, MiMo, North Mini, Nemotron | e.g., `glm-5.2` | `https://opencode.ai/zen/v1/chat/completions` | `@ai-sdk/openai-compatible` |

## Usage Patterns

**Connecting in OpenCode TUI:**
1. Run `/connect` in the TUI.
2. Select **OpenCode Zen**.
3. Paste the API key.
4. Run `/models` to view available recommended models.

**Configuration:**
To use GPT 5.5, set the model ID in the OpenCode config as:
`opencode/gpt-5.5`

## Pricing

Zen passes provider costs directly without markups (excluding credit card processing fees of 4.4% + $0.30). Pricing is per 1M tokens (Input / Output / Cached Read / Cached Write). Examples include:
- DeepSeek V4 Pro: $1.74 / $3.48 / $0.145 / -
- Claude Sonnet 5: $2.00 / $10.00 / $0.20 / $2.50
- GPT 5.5 (≤ 272K): $5.00 / $30.00 / $0.50 / -

*Note: Several free tier/trial models are available for a limited time (e.g., DeepSeek V4 Flash Free, Big Pickle).*

## Privacy and Data

- Models are hosted in the US.
- Zero-retention policy is standard; data is not used for training.
- **Exceptions**: Free tier models (Big Pickle, DeepSeek V4 Flash Free, MiMo-V2.5 Free, North Mini Code Free, Nemotron 3 Ultra Free) may log or use data to improve models. Do not send sensitive data to free models.
- OpenAI and Anthropic APIs retain requests for 30 days as per their respective policies.

## Version Notes

- Certain legacy models (e.g., GPT 5.2 Codex, Claude Sonnet 4) have specific deprecation dates ranging from February to August 2026.
