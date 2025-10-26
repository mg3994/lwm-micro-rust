# Build stage
FROM rust:1.75-slim as builder

# Install system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY shared/ shared/
COPY services/safety-moderation/ services/safety-moderation/

# Build the application
RUN cargo build --release --bin safety-moderation

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -r -s /bin/false appuser

# Set working directory
WORKDIR /app

# Create directories for ML models
RUN mkdir -p /app/models && chown -R appuser:appuser /app

# Copy binary from builder stage
COPY --from=builder /app/target/release/safety-moderation /app/safety-moderation

# Change ownership
RUN chown -R appuser:appuser /app

# Switch to app user
USER appuser

# Expose port
EXPOSE 8006

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8006/health || exit 1

# Run the application
CMD ["./safety-moderation"]