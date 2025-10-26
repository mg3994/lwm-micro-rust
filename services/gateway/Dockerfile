# Multi-stage build for Gateway Service
FROM rust:1.75 as builder

WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY shared/ ./shared/
COPY services/gateway/ ./services/gateway/

# Build the application
RUN cargo build --release --bin gateway

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

# Copy the binary
COPY --from=builder /app/target/release/gateway /usr/local/bin/gateway

# Change ownership
RUN chown appuser:appuser /usr/local/bin/gateway

# Switch to app user
USER appuser

# Expose port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Run the application
CMD ["gateway"]