# BUNKER POOL - Production-Grade Multi-Stage Dockerfile
# Optimized for security, performance, and minimal attack surface

#==============================================================================
# Stage 1: Build Environment
#==============================================================================
FROM rust:1.82-bookworm as builder

# Set up build environment
WORKDIR /build
ENV DEBIAN_FRONTEND=noninteractive

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Cache dependencies by building them first
COPY Cargo.toml ./
COPY Cargo.loc[k] ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release && rm -rf src target/release/deps/bunker_pool*

# Copy source code and build
COPY src/ ./src/
RUN cargo build --release

# Verify the binary was built
RUN ls -la target/release/ && \
    ldd target/release/bunker-pool || echo "Static binary (no dynamic deps)" && \
    file target/release/bunker-pool

#==============================================================================
# Stage 2: Runtime Environment
#==============================================================================
FROM debian:bookworm-slim as runtime

# Create non-root user for security
RUN groupadd -r bunker && useradd -r -g bunker -d /opt/bunker -s /bin/false bunker

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

# Create directories with proper permissions
RUN mkdir -p /opt/bunker/config /opt/bunker/logs /opt/bunker/data && \
    chown -R bunker:bunker /opt/bunker

# Copy binary from builder stage
COPY --from=builder /build/target/release/bunker-pool /usr/local/bin/bunker-pool
RUN chmod +x /usr/local/bin/bunker-pool

# Copy configuration template
COPY --chown=bunker:bunker bunker-pool.example.toml /opt/bunker/config/bunker-pool.toml

# Switch to non-root user
USER bunker
WORKDIR /opt/bunker

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD /usr/local/bin/bunker-pool --help > /dev/null || exit 1

# Environment variables
ENV RUST_LOG=info
ENV BUNKER_POOL_CONFIG=/opt/bunker/config/bunker-pool.toml

# Expose ports
EXPOSE 3333/tcp 9090/tcp

# Labels for metadata
LABEL maintainer="BUNKER CORPORATION <engineering@bunker-corporation.com>"
LABEL version="0.1.0"
LABEL description="BUNKER POOL - High-performance multi-algorithm mining pool"
LABEL org.opencontainers.image.source="https://github.com/emiliancristea/bunker-miner"
LABEL org.opencontainers.image.documentation="https://github.com/emiliancristea/bunker-miner/pool"
LABEL org.opencontainers.image.vendor="BUNKER CORPORATION"
LABEL org.opencontainers.image.title="BUNKER POOL"

# Security labels
LABEL security.scan.enabled="true"
LABEL security.no-new-privileges="true"

# Default command
ENTRYPOINT ["/usr/local/bin/bunker-pool"]
CMD ["--config", "/opt/bunker/config/bunker-pool.toml"]