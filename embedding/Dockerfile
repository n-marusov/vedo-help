# syntax=docker/dockerfile:1
# Multi-stage: builder → development → production
# pyproject.toml requires Python >=3.11

# --- Builder (dependencies + wheel build) ---
FROM ghcr.io/astral-sh/uv:python3.11-bookworm-slim AS builder

WORKDIR /app

# Install project dependencies only (frozen from uv.lock, no dev deps)
# --mount=type=cache persists uv's package cache across Docker builds,
# avoiding re-download of large packages like torch and sentence-transformers
RUN --mount=type=cache,target=/root/.cache/uv \
    --mount=type=bind,source=uv.lock,target=uv.lock \
    --mount=type=bind,source=pyproject.toml,target=pyproject.toml \
    uv sync --frozen --no-dev --no-install-project

# Copy and install the project itself
COPY src/ ./src/

RUN --mount=type=cache,target=/root/.cache/uv \
    --mount=type=bind,source=uv.lock,target=uv.lock \
    --mount=type=bind,source=pyproject.toml,target=pyproject.toml \
    uv sync --frozen --no-dev

# Pre-cache the sentence-transformers model (avoids download on first start)
ENV EMBEDDING_MODEL=all-MiniLM-L6-v2
RUN uv run python -c "from sentence_transformers import SentenceTransformer; SentenceTransformer('${EMBEDDING_MODEL}')"

# --- Development ---
FROM python:3.11-slim AS development

RUN groupadd -r vedo && useradd -r -g vedo -d /home/vedo -m vedo

WORKDIR /app

# Install uv for dev dependency management
RUN pip install --no-cache-dir uv

# Sync with dev dependencies
COPY pyproject.toml uv.lock ./
RUN --mount=type=cache,target=/root/.cache/uv \
    uv sync --frozen

COPY src/ ./src/
RUN mkdir -p /data/cache && chown -R vedo:vedo /app /data /home/vedo

USER vedo

# Pre-cache the sentence-transformers model for development (as the vedo user,
# so the cache lands in ~/.cache/huggingface where the runtime user can find it)
ENV EMBEDDING_MODEL=all-MiniLM-L6-v2
RUN uv run python -c "from sentence_transformers import SentenceTransformer; SentenceTransformer('${EMBEDDING_MODEL}')"

EXPOSE 8001

CMD ["uv", "run", "uvicorn", "src.main:app", "--host", "0.0.0.0", "--port", "8001", "--reload"]

# --- Production ---
FROM python:3.11-slim AS production

RUN groupadd -r vedo && useradd -r -g vedo -d /home/vedo -m vedo

WORKDIR /app

# Runtime dependencies only (libgcc for torch, etc.)
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        curl \
        && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/.venv /app/.venv
ENV PATH=/app/.venv/bin:$PATH

COPY src/ ./src/
RUN mkdir -p /data/cache && chown -R vedo:vedo /data

# Copy pre-cached HuggingFace model from builder
COPY --from=builder /root/.cache/huggingface /root/.cache/huggingface

USER vedo

EXPOSE 8001

HEALTHCHECK --interval=30s --timeout=5s --start-period=30s --retries=3 \
    CMD curl -f http://localhost:8001/health || exit 1

CMD ["uvicorn", "src.main:app", "--host", "0.0.0.0", "--port", "8001"]
