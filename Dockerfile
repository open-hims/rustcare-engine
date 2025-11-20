# Multi-stage Dockerfile for RustCare Server
# Optimized for fast builds with proper layer caching
# Requires Docker BuildKit: DOCKER_BUILDKIT=1 docker build ...

# syntax=docker/dockerfile:1.4

# Builder stage
# Using latest stable Rust (1.85+) which supports edition2024 (stabilized)
# home 0.5.12 requires edition2024, which is available in stable Rust 1.85+
FROM rust:latest as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    cmake \
    build-essential \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy only Cargo files first for dependency caching
# If Cargo.lock version is incompatible, Cargo will regenerate it
COPY Cargo.toml ./
COPY Cargo.lock* ./

# Copy workspace member Cargo.toml files to build dependency graph
# This allows Cargo to resolve dependencies without copying all source code
COPY error-common/Cargo.toml ./error-common/
COPY logger-redacted/Cargo.toml ./logger-redacted/
COPY database-layer/Cargo.toml ./database-layer/
COPY auth-identity/Cargo.toml ./auth-identity/
COPY auth-oauth/Cargo.toml ./auth-oauth/
COPY auth-zanzibar/Cargo.toml ./auth-zanzibar/
COPY auth-gateway/Cargo.toml ./auth-gateway/
COPY auth-config.toml ./auth-config.toml
COPY config-engine/Cargo.toml ./config-engine/
COPY workflow-engine/Cargo.toml ./workflow-engine/
COPY audit-engine/Cargo.toml ./audit-engine/
COPY crypto/Cargo.toml ./crypto/
COPY object-governance/Cargo.toml ./object-governance/
COPY telemetry/Cargo.toml ./telemetry/
COPY ops-cli/Cargo.toml ./ops-cli/
COPY plugin-runtime-core/Cargo.toml ./plugin-runtime-core/
COPY plugins-registry-api/Cargo.toml ./plugins-registry-api/
COPY device-manager/Cargo.toml ./device-manager/
COPY voice-recognition-service/Cargo.toml ./voice-recognition-service/
COPY mcp-macros/Cargo.toml ./mcp-macros/
COPY mcp-server/Cargo.toml ./mcp-server/
COPY billing-service/Cargo.toml ./billing-service/
COPY insurance-service/Cargo.toml ./insurance-service/
COPY accounting-service/Cargo.toml ./accounting-service/
COPY external-services/events-bus/Cargo.toml ./external-services/events-bus/
COPY external-services/email-service/Cargo.toml ./external-services/email-service/
COPY external-services/secrets-service/Cargo.toml ./external-services/secrets-service/
COPY server/rustcare-server/Cargo.toml ./server/rustcare-server/
COPY server/rustcare-sync/Cargo.toml ./server/rustcare-sync/

# Create dummy source files to satisfy Cargo and build dependencies
# This layer will be cached unless Cargo.toml files change
RUN mkdir -p error-common/src logger-redacted/src database-layer/src \
    auth-identity/src auth-oauth/src auth-zanzibar/src auth-gateway/src \
    config-engine/src workflow-engine/src audit-engine/src crypto/src \
    object-governance/src telemetry/src ops-cli/src plugin-runtime-core/src \
    plugins-registry-api/src device-manager/src voice-recognition-service/src \
    mcp-macros/src mcp-server/src billing-service/src insurance-service/src \
    accounting-service/src external-services/events-bus/src \
    external-services/email-service/src external-services/secrets-service/src \
    server/rustcare-server/src server/rustcare-sync/src && \
    echo "fn main() {}" > error-common/src/lib.rs && \
    echo "fn main() {}" > logger-redacted/src/lib.rs && \
    echo "fn main() {}" > database-layer/src/lib.rs && \
    echo "fn main() {}" > auth-identity/src/lib.rs && \
    echo "fn main() {}" > auth-oauth/src/lib.rs && \
    echo "fn main() {}" > auth-zanzibar/src/lib.rs && \
    echo "fn main() {}" > auth-gateway/src/lib.rs && \
    echo "fn main() {}" > config-engine/src/lib.rs && \
    echo "fn main() {}" > workflow-engine/src/lib.rs && \
    echo "fn main() {}" > audit-engine/src/lib.rs && \
    echo "fn main() {}" > crypto/src/lib.rs && \
    echo "fn main() {}" > object-governance/src/lib.rs && \
    echo "fn main() {}" > telemetry/src/lib.rs && \
    echo "fn main() {}" > ops-cli/src/lib.rs && \
    echo "fn main() {}" > plugin-runtime-core/src/lib.rs && \
    echo "fn main() {}" > plugins-registry-api/src/lib.rs && \
    echo "fn main() {}" > device-manager/src/lib.rs && \
    echo "fn main() {}" > voice-recognition-service/src/lib.rs && \
    echo "fn main() {}" > mcp-macros/src/lib.rs && \
    echo "fn main() {}" > mcp-server/src/lib.rs && \
    echo "fn main() {}" > billing-service/src/lib.rs && \
    echo "fn main() {}" > insurance-service/src/lib.rs && \
    echo "fn main() {}" > accounting-service/src/lib.rs && \
    echo "fn main() {}" > external-services/events-bus/src/lib.rs && \
    echo "fn main() {}" > external-services/email-service/src/lib.rs && \
    echo "fn main() {}" > external-services/secrets-service/src/lib.rs && \
    echo "fn main() {}" > server/rustcare-server/src/main.rs && \
    echo "fn main() {}" > server/rustcare-sync/src/lib.rs

# Build dependencies only (this layer will be cached unless Cargo files change)
# Use BuildKit cache mounts for faster subsequent builds
# If Cargo.lock version is incompatible, Cargo will automatically regenerate it
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release --bin rustcare-server || true

# Now copy the actual source code
# This layer only invalidates when source code changes, not dependencies
COPY error-common ./error-common
COPY logger-redacted ./logger-redacted
COPY database-layer ./database-layer
COPY auth-identity ./auth-identity
COPY auth-oauth ./auth-oauth
COPY auth-zanzibar ./auth-zanzibar
COPY auth-gateway ./auth-gateway
COPY auth-config.toml ./auth-config.toml
COPY config-engine ./config-engine
COPY workflow-engine ./workflow-engine
COPY audit-engine ./audit-engine
COPY crypto ./crypto
COPY object-governance ./object-governance
COPY telemetry ./telemetry
COPY ops-cli ./ops-cli
COPY plugin-runtime-core ./plugin-runtime-core
COPY plugins-registry-api ./plugins-registry-api
COPY device-manager ./device-manager
COPY voice-recognition-service ./voice-recognition-service
COPY mcp-macros ./mcp-macros
COPY mcp-server ./mcp-server
COPY billing-service ./billing-service
COPY insurance-service ./insurance-service
COPY accounting-service ./accounting-service
COPY external-services ./external-services
COPY server/rustcare-server ./server/rustcare-server
COPY server/rustcare-sync ./server/rustcare-sync

# Build the actual application with cache mounts
# Cargo will automatically handle lock file version compatibility
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release --bin rustcare-server

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libpq5 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -m -u 1000 rustcare

# Set working directory
WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/rustcare-server /usr/local/bin/rustcare-server

# Copy configuration files
COPY config /app/config
COPY migrations /app/migrations
COPY Caddyfile /app/Caddyfile

# Set permissions
RUN chown -R rustcare:rustcare /app
RUN chmod +x /usr/local/bin/rustcare-server

# Switch to non-root user
USER rustcare

# Expose port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=40s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Run the server
CMD ["rustcare-server"]
