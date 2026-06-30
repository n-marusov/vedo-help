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

# Pre-download the embedding model at build time so it is cached in the image.
# Without this, SentenceTransformer downloads the model on first startup,
# which can hang the lifespan phase (especially without HF_TOKEN).
ENV HF_HOME=/tmp/hf-cache
RUN uv run python <<EOF
from sentence_transformers import SentenceTransformer
SentenceTransformer('BAAI/bge-small-en-v1.5')
EOF

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

# Pre-download the embedding model at build time for development.
ENV HF_HOME=/app/hf-cache
RUN uv run python <<EOF
from sentence_transformers import SentenceTransformer
SentenceTransformer('BAAI/bge-small-en-v1.5')
EOF

COPY src/ ./src/
RUN mkdir -p /data/cache && chown -R vedo:vedo /app /data

USER vedo

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
COPY --from=builder /tmp/hf-cache /app/hf-cache
ENV PATH=/app/.venv/bin:$PATH
ENV HF_HOME=/app/hf-cache

COPY src/ ./src/
RUN mkdir -p /data/cache && chown -R vedo:vedo /data

USER vedo

EXPOSE 8001

HEALTHCHECK --interval=30s --timeout=5s --start-period=30s --retries=3 \
    CMD curl -f http://localhost:8001/health || exit 1

CMD ["uvicorn", "src.main:app", "--host", "0.0.0.0", "--port", "8001"]
