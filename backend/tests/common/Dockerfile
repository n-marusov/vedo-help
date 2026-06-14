# syntax=docker/dockerfile:1
# Multi-stage build: builder → distroless runtime

# ---- Builder stage ----
FROM rust:1.95-slim-bookworm AS builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Cache dependencies by copying Cargo.toml first
COPY Cargo.toml Cargo.lock* ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release 2>/dev/null || true
RUN rm -rf src

# Build the actual application
COPY . .
RUN cargo build --release --bin vedo-backend

# ---- Runtime stage ----
FROM debian:12-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create /data directory (owned by root — Docker volumes mount as root)
RUN mkdir -p /data

COPY --from=builder /app/target/release/vedo-backend /usr/local/bin/vedo-backend

EXPOSE 3000

HEALTHCHECK --interval=30s --timeout=3s --start-period=15s --retries=3 \
    CMD curl -f http://localhost:3000/health || exit 1

ENTRYPOINT ["vedo-backend"]
