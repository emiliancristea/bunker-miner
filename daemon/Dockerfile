# BUNKER MINER Daemon - Multi-stage Docker Build
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
# - Read-only root filesystem
# - Explicit port and volume declarations

# Stage 1: Build Environment
FROM rust:1.90-bookworm as builder

# Set build-time metadata
LABEL stage=builder
LABEL description="BUNKER MINER Daemon build environment"

# Create application user for build
RUN groupadd -r bunker && useradd -r -g bunker bunker

# Install build dependencies
RUN apt-get update && apt-get install -y \
    protobuf-compiler \
    pkg-config \
    libssl-dev \
    cmake \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy dependency manifests first for better caching
COPY Cargo.toml Cargo.lock ./
COPY daemon/Cargo.toml daemon/
COPY libs/common-rust/Cargo.toml libs/common-rust/
COPY protos/ protos/

# Create dummy source files to build dependencies
RUN mkdir -p daemon/src libs/common-rust/src && \
    echo "fn main() {}" > daemon/src/main.rs && \
    echo "// dummy" > libs/common-rust/src/lib.rs

# Build dependencies (this layer will be cached)
RUN cargo build --release --bin bunker-miner-daemon

# Remove dummy files
RUN rm daemon/src/main.rs libs/common-rust/src/lib.rs

# Copy actual source code
COPY daemon/src/ daemon/src/
COPY daemon/build.rs daemon/
COPY libs/common-rust/src/ libs/common-rust/src/
COPY tools/poc/src/lib.rs tools/poc/src/
COPY tools/poc/src/hardware_detection.rs tools/poc/src/

# Build the actual application
RUN cargo build --release --bin bunker-miner-daemon

# Create directory structure for runtime
RUN mkdir -p /app/runtime/bin /app/runtime/etc /app/runtime/logs && \
    cp target/release/bunker-miner-daemon /app/runtime/bin/ && \
    chown -R bunker:bunker /app/runtime

# Stage 2: Runtime Environment
FROM gcr.io/distroless/cc-debian12:nonroot as runtime

# Set runtime metadata
LABEL maintainer="Emilian Cristea <emilian@bunkercorpo.com>"
LABEL description="BUNKER MINER Daemon - Secure cryptocurrency mining management daemon"
LABEL version="0.1.0"
LABEL security.policy="Runs as non-root user in distroless container"

# Copy application binary from builder
COPY --from=builder --chown=nonroot:nonroot /app/runtime/bin/bunker-miner-daemon /usr/local/bin/bunker-miner-daemon

# Create necessary runtime directories
COPY --from=builder --chown=nonroot:nonroot /app/runtime/etc /etc/bunker-miner
COPY --from=builder --chown=nonroot:nonroot /app/runtime/logs /var/log/bunker-miner

# Set environment variables
ENV RUST_LOG=info
ENV RUST_BACKTRACE=1
ENV BUNKER_MINER_CONFIG_PATH=/etc/bunker-miner
ENV BUNKER_MINER_LOG_PATH=/var/log/bunker-miner
ENV BUNKER_MINER_BIND_ADDRESS=0.0.0.0:50051

# Expose gRPC port
EXPOSE 50051

# Define volumes for persistent data
VOLUME ["/etc/bunker-miner", "/var/log/bunker-miner"]

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD ["/usr/local/bin/bunker-miner-daemon", "--health-check"]

# Security: Run as non-root user
USER nonroot:nonroot

# Security: Set read-only root filesystem
# This will be enforced at runtime with --read-only flag

# Default command
ENTRYPOINT ["/usr/local/bin/bunker-miner-daemon"]
CMD ["--config-path", "/etc/bunker-miner", "--log-level", "info"]

# Build-time security checks
# These are comments for documentation - actual checks happen in CI
# - Runs as non-root user (nonroot:nonroot)
# - Uses minimal distroless base image
# - No shell access or package managers in final image
# - Explicit port declaration for security scanning
# - Volume declarations for data persistence
# - Health check for container orchestration
# - Multi-stage build minimizes attack surface