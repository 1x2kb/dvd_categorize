# Multi-stage build for production Diesel migrations
FROM rust:bookworm as builder

WORKDIR /app

# Install diesel CLI
RUN cargo install diesel_cli --no-default-features --features postgres

# Copy migration files
COPY database/ ./database/

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y \
        ca-certificates \
        libssl3 \
        libpq5 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN groupadd -r appuser && useradd -r -g appuser appuser

# Copy diesel CLI from builder stage
COPY --from=builder /usr/local/cargo/bin/diesel /usr/local/bin/diesel

# Copy database files
COPY --from=builder /app/database /app/database

# Set ownership
RUN chown -R appuser:appuser /app && \
    chown appuser:appuser /usr/local/bin/diesel

USER appuser
WORKDIR /app/database

CMD ["diesel", "migration", "run"]
