# Multi-stage build for User Management Service
FROM rust:1.75 as builder

WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY shared/ ./shared/
COPY services/user-management/ ./services/user-management/

# Build the application
RUN cargo build --release --bin user-management

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -r -s /bin/false appuser

# Copy the binary
COPY --from=builder /app/target/release/user-management /usr/local/bin/user-management

# Change ownership
RUN chown appuser:appuser /usr/local/bin/user-management

# Switch to app user
USER appuser

# Expose port
EXPOSE 8000

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8000/health || exit 1

# Run the application
CMD ["user-management"]