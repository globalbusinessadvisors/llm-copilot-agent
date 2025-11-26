# Multi-stage Dockerfile for LLM-CoPilot-Agent
# Stage 1: Chef planner - analyzes dependencies for better caching
FROM rust:1.80-slim as chef
WORKDIR /app
RUN cargo install cargo-chef --locked

# Stage 2: Recipe preparation - creates dependency list
FROM chef as planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Stage 3: Builder - compiles dependencies and application
FROM chef as builder
COPY --from=planner /app/recipe.json recipe.json

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

# Build dependencies - this layer is cached unless dependencies change
RUN cargo chef cook --release --recipe-path recipe.json

# Copy source code and build application
COPY . .
RUN cargo build --release --bin copilot-server

# Stage 4: Runtime - minimal production image
FROM debian:bookworm-slim as runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libpq5 \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN groupadd -r copilot && useradd -r -g copilot -u 1000 copilot

# Set up application directory
WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/copilot-server /usr/local/bin/copilot-server

# Set ownership
RUN chown -R copilot:copilot /app

# Switch to non-root user
USER copilot

# Expose ports
# 8080: HTTP API
# 50051: gRPC
# 9090: Metrics
EXPOSE 8080 50051 9090

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Set default environment variables
ENV RUST_LOG=info \
    RUST_BACKTRACE=1

# Run the application
ENTRYPOINT ["/usr/local/bin/copilot-server"]
CMD ["server"]
