# Multi-stage Dockerfile for BUNKER Fleet Controller
FROM rust:1.75-slim as builder

# Install system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy Cargo files
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to cache dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy source code
COPY src ./src
COPY migrations ./migrations
COPY static ./static

# Build the application
RUN touch src/main.rs && \
    cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libpq5 \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -m -u 1001 bunker

# Create app directory
WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/bunker-fleet-controller /app/
COPY --from=builder /app/migrations /app/migrations
COPY --from=builder /app/static /app/static

# Change ownership
RUN chown -R bunker:bunker /app

# Switch to app user
USER bunker

# Expose port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/api/health || exit 1

# Set entrypoint
ENTRYPOINT ["./bunker-fleet-controller"]