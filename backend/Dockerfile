# syntax=docker/dockerfile:1
# Multi-stage build: deps → builder → development / production

# ====================================================================
# Deps stage – resolve & cache dependencies using a dummy main.rs
# ====================================================================
FROM rust:1.96-slim-bookworm AS deps

WORKDIR /app

# Install build dependencies for sqlx (links against libpq)
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        pkg-config \
        libpq-dev \
        libssl-dev \
        && rm -rf /var/lib/apt/lists/*

# Copy manifests first for dependency caching
COPY Cargo.toml Cargo.lock ./

# Create a dummy entry point to let Cargo resolve depenencies
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies only – cached via BuildKit mount
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release 2>/dev/null || true

# ====================================================================
# Builder stage – compile the real binary
# ====================================================================
FROM deps AS builder

# Copy the actual source code and compile-time SQL migrations
COPY src/ ./src/
COPY migrations/ ./migrations/

# Full release build (reuses cached dependencies from /app/target)
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release && \
    cp target/release/vedo-backend /app/vedo-backend

# ====================================================================
# Development stage – hot-reload with cargo-watch
# ====================================================================
FROM deps AS development

# Install cargo-watch for file-change-triggered rebuilds
RUN cargo install cargo-watch

# Expose the default backend port
EXPOSE 3000

# Rebuild and restart on every source change
CMD ["cargo", "watch", "-x", "run"]

# ====================================================================
# Production stage – minimal runtime image
# ====================================================================
FROM debian:bookworm-slim AS production

# Create a non-root user
RUN groupadd -r vedo && useradd -r -g vedo -d /home/vedo -m vedo

# Runtime libraries needed by the binary
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        ca-certificates \
            libpq5 \
            curl \
            && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the pre-compiled binary from the builder stage
COPY --from=builder /app/vedo-backend /app/vedo-backend

USER vedo

EXPOSE 3000

HEALTHCHECK --interval=30s --timeout=3s --start-period=15s --retries=3 \
  CMD curl -f http://localhost:3000/health || exit 1

CMD ["/app/vedo-backend"]
