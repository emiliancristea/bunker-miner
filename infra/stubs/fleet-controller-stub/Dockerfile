# BUNKER MINER Fleet Controller Stub - Multi-stage Docker Build
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
LABEL description="BUNKER MINER Fleet Controller Stub build environment"

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
COPY infra/stubs/fleet-controller-stub/Cargo.toml infra/stubs/fleet-controller-stub/Cargo.lock ./

# Create dummy source file to build dependencies
RUN mkdir -p src && \
    echo "fn main() {}" > src/main.rs

# Build dependencies (this layer will be cached)
RUN cargo build --release

# Remove dummy file
RUN rm src/main.rs

# Copy actual source code
COPY infra/stubs/fleet-controller-stub/src/ src/

# Build the actual application
RUN cargo build --release

# Create directory structure for runtime
RUN mkdir -p /app/runtime/bin && \
    cp target/release/fleet-controller-stub /app/runtime/bin/ && \
    chown -R bunker:bunker /app/runtime

# Stage 2: Runtime Environment
FROM gcr.io/distroless/cc-debian12:nonroot as runtime

# Set runtime metadata
LABEL maintainer="Emilian Cristea <emilian@bunkercorpo.com>"
LABEL description="BUNKER MINER Fleet Controller Stub - Secure WebSocket fleet management stub"
LABEL version="0.1.0-stub"
LABEL security.policy="Runs as non-root user in distroless container"
LABEL org.opencontainers.image.source="https://github.com/emiliancristea/bunker-miner"
LABEL org.opencontainers.image.title="BUNKER MINER Fleet Controller Stub"
LABEL org.opencontainers.image.description="Development stub for Fleet Controller WebSocket API"

# Copy application binary from builder
COPY --from=builder --chown=nonroot:nonroot /app/runtime/bin/fleet-controller-stub /usr/local/bin/fleet-controller-stub

# Set environment variables
ENV RUST_LOG=info
ENV RUST_BACKTRACE=1
ENV FLEET_WS_BIND_ADDRESS=0.0.0.0:8081
ENV FLEET_HTTP_BIND_ADDRESS=0.0.0.0:8082

# Expose WebSocket and HTTP ports
EXPOSE 8081 8082

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD ["/usr/local/bin/fleet-controller-stub", "--health-check"] || exit 1

# Security: Run as non-root user
USER nonroot:nonroot

# Default command
ENTRYPOINT ["/usr/local/bin/fleet-controller-stub"]
CMD ["--ws-bind", "0.0.0.0:8081", "--http-bind", "0.0.0.0:8082"]

# Build-time security documentation
# - Runs as non-root user (nonroot:nonroot)
# - Uses minimal distroless base image
# - No shell access or package managers in final image
# - Explicit port declarations for security scanning
# - Health check for container orchestration
# - Multi-stage build minimizes attack surface
# - Compatible with read-only root filesystem
# - WebSocket and HTTP endpoints on separate ports for security