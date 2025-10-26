# Build stage
FROM rust:1.75-slim as builder

# Install system dependencies including FFmpeg
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ffmpeg \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY shared/ shared/
COPY services/video-lectures/ services/video-lectures/

# Build the application
RUN cargo build --release --bin video-lectures

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies including FFmpeg
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    ffmpeg \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -r -s /bin/false appuser

# Set working directory
WORKDIR /app

# Create necessary directories
RUN mkdir -p /app/uploads /app/processed /app/thumbnails && \
    chown -R appuser:appuser /app

# Copy binary from builder stage
COPY --from=builder /app/target/release/video-lectures /app/video-lectures

# Change ownership
RUN chown -R appuser:appuser /app

# Switch to app user
USER appuser

# Expose port
EXPOSE 8009

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8009/health || exit 1

# Run the application
CMD ["./video-lectures"]