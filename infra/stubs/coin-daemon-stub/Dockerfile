# BUNKER MINER Coin Daemon Stub - Multi-stage Docker Build
# Security-hardened container image with minimal attack surface
#
# Build stages:
# 1. Builder: Compile Rust application with all dependencies
# 2. Runtime: Minimal distroless image with only necessary runtime components
#
# Security features:
# - Non-root user execution
# - Minimal base image (Google distroless)
# - No shell or unnecessary binaries
# - Read-only root filesystem compatible
# - Explicit port and volume declarations

# Stage 1: Build Environment
FROM rust:1.75-bookworm as builder

# Set build-time metadata
LABEL stage=builder
LABEL description="BUNKER MINER Coin Daemon Stub build environment"

# Create application user for build
RUN groupadd -r bunker && useradd -r -g bunker bunker

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    cmake \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy dependency manifests first for better caching
COPY infra/stubs/coin-daemon-stub/Cargo.toml infra/stubs/coin-daemon-stub/Cargo.lock ./

# Create dummy source file to build dependencies
RUN mkdir -p src && \
    echo "fn main() {}" > src/main.rs

# Build dependencies (this layer will be cached)
RUN cargo build --release

# Remove dummy file
RUN rm src/main.rs

# Copy actual source code
COPY infra/stubs/coin-daemon-stub/src/ src/

# Build the actual application
RUN cargo build --release

# Create directory structure for runtime
RUN mkdir -p /app/runtime/bin && \
    cp target/release/coin-daemon-stub /app/runtime/bin/ && \
    chown -R bunker:bunker /app/runtime

# Stage 2: Runtime Environment
FROM gcr.io/distroless/cc-debian12:nonroot as runtime

# Set runtime metadata
LABEL maintainer="Emilian Cristea <emilian@bunkercorpo.com>"
LABEL description="BUNKER MINER Coin Daemon Stub - Secure JSON-RPC cryptocurrency daemon stub"
LABEL version="0.1.0-stub"
LABEL security.policy="Runs as non-root user in distroless container"
LABEL org.opencontainers.image.source="https://github.com/emiliancristea/bunker-miner"
LABEL org.opencontainers.image.title="BUNKER MINER Coin Daemon Stub"
LABEL org.opencontainers.image.description="Development stub for cryptocurrency daemon JSON-RPC API"

# Copy application binary from builder
COPY --from=builder --chown=nonroot:nonroot /app/runtime/bin/coin-daemon-stub /usr/local/bin/coin-daemon-stub

# Set environment variables
ENV RUST_LOG=info
ENV RUST_BACKTRACE=1
ENV COIN_DAEMON_BIND_ADDRESS=0.0.0.0:18081
ENV COIN_DAEMON_DIFFICULTY=300000000000

# Expose JSON-RPC port
EXPOSE 18081

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD ["/usr/local/bin/coin-daemon-stub", "--health-check"] || exit 1

# Security: Run as non-root user
USER nonroot:nonroot

# Default command
ENTRYPOINT ["/usr/local/bin/coin-daemon-stub"]
CMD ["--bind", "0.0.0.0:18081", "--difficulty", "300000000000"]

# Build-time security documentation
# - Runs as non-root user (nonroot:nonroot)
# - Uses minimal distroless base image
# - No shell access or package managers in final image
# - Explicit port declaration for security scanning
# - Health check for container orchestration
# - Multi-stage build minimizes attack surface
# - Compatible with read-only root filesystem
# - JSON-RPC endpoint for cryptocurrency operations