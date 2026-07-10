# OpenCode Go Reference

> Source: https://opencode.ai/docs/ru/go/
> Created: 2026-07-10
> Updated: 2026-07-10

## Overview

OpenCode Go is a low-cost subscription service ($5 for the first month, $10/month thereafter) providing reliable access to popular open-source coding models. Designed primarily for a global audience, it hosts models in the US, EU, and Singapore for low-latency access. It functions as an optional provider within OpenCode.

## Core Concepts

**Subscription Model**: Flat monthly fee ($5 first month, then $10) for access to a tier of open models.
**Global Hosting**: Models hosted across the US, EU, and Singapore for reliable, low-latency global access.
**Usage Limits**: Go imposes spending-equivalent limits to manage fair use:
- 5-hour limit: $12 value
- Weekly limit: $30 value
- Monthly limit: $60 value
*(Actual request volume varies significantly depending on the specific model's cost).*
**Balance Fallback**: If usage limits are reached, users can opt to use their regular Zen balance to continue making requests.
**Workspace Limitation**: Only one member per workspace can subscribe to OpenCode Go.

## API / Interface

Models can be accessed via specific API endpoints. The model ID format in the OpenCode config is `opencode-go/<model-id>`.

Available endpoints:
- OpenAI Compatible: `https://opencode.ai/zen/go/v1/chat/completions` (GLM, Kimi, DeepSeek, MiMo)
- Anthropic Compatible: `https://opencode.ai/zen/go/v1/messages` (MiniMax, Qwen)
- Full model list metadata: `https://opencode.ai/zen/go/v1/models`

### Supported Models

- **GLM**: GLM-5.2, GLM-5.1
- **Kimi**: Kimi K2.7 Code, Kimi K2.6
- **MiMo**: MiMo-V2.5, MiMo-V2.5-Pro
- **MiniMax**: MiniMax M3, MiniMax M2.7, MiniMax M2.5
- **Qwen**: Qwen3.7 Max, Qwen3.7 Plus, Qwen3.6 Plus
- **DeepSeek**: DeepSeek V4 Pro, DeepSeek V4 Flash

## Usage Patterns

**Connecting in OpenCode TUI:**
1. Run `/connect` in the TUI.
2. Select **OpenCode Go**.
3. Paste the API key.
4. Run `/models` to view models available through the Go subscription.

**Configuration:**
To use Kimi K2.7 Code, set the model ID in the OpenCode config as:
`opencode-go/kimi-k2.7-code`

## Privacy and Data

- Targeted at international users with global hosting.
- Zero-retention policy is standard; data is not used for model training.

## Configuration

**Overage Handling**: Enable "Use balance" in the console to allow OpenCode Go to pull from the OpenCode Zen prepaid balance once the Go subscription usage limits are exhausted.
