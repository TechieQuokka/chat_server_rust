# ============================================
# Multi-stage Dockerfile for Rust Chat Server
# Optimized for production deployment
# ============================================

# ============================================
# Stage 1: Builder - Compile the application
# ============================================
FROM rust:1.83-slim AS builder

# Install build dependencies
RUN apt-get update && \
    apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create a new empty shell project
WORKDIR /app

# Copy manifests first to cache dependencies
COPY Cargo.toml Cargo.lock ./
COPY rust-toolchain.toml ./

# Create a dummy main.rs to build dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    echo "// dummy" > src/lib.rs

# Build and cache dependencies only (this layer will be cached)
RUN cargo build --release && \
    rm -rf src target/release/.fingerprint/chat-server-*

# Copy the actual source code
COPY src ./src
COPY migrations ./migrations
COPY config ./config

# Build the actual application
# The dependency cache will be reused here
RUN cargo build --release --bin chat-server

# Verify the binary was built
RUN test -f /app/target/release/chat-server || (echo "Binary not found!" && exit 1)

# ============================================
# Stage 2: Runtime - Minimal production image
# ============================================
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies only
RUN apt-get update && \
    apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create a non-root user for security
RUN groupadd -r chatserver && \
    useradd -r -g chatserver -s /bin/false chatserver

# Create necessary directories
RUN mkdir -p /app/config /app/migrations /app/uploads && \
    chown -R chatserver:chatserver /app

# Set working directory
WORKDIR /app

# Copy the compiled binary from builder
COPY --from=builder --chown=chatserver:chatserver /app/target/release/chat-server /app/chat-server

# Copy migrations for SQLx runtime
COPY --chown=chatserver:chatserver migrations ./migrations

# Copy configuration files
COPY --chown=chatserver:chatserver config ./config

# Switch to non-root user
USER chatserver

# Expose the application port
EXPOSE 3000

# Expose metrics port (Prometheus)
EXPOSE 9100

# Set environment variables
ENV RUST_LOG=info \
    RUST_BACKTRACE=1 \
    SERVER_HOST=0.0.0.0 \
    SERVER_PORT=3000

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=40s --retries=3 \
    CMD ["/bin/sh", "-c", "timeout 5 bash -c '</dev/tcp/localhost/3000' || exit 1"]

# Run the binary
ENTRYPOINT ["/app/chat-server"]
