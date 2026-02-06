# Multi-stage build for production CSV Reader
FROM rust:latest as builder

WORKDIR /app

# Copy all source code
COPY . .

# Build the CSV reader binary
RUN cargo build --release --bin csv_reader

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y \
        ca-certificates \
        libssl3 \
        libpq5 \
        tini \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN groupadd -r appuser && useradd -r -g appuser appuser

# Copy the binary from builder stage
COPY --from=builder /app/target/release/csv_reader /usr/local/bin/csv_reader

# Set ownership
RUN chown appuser:appuser /usr/local/bin/csv_reader

USER appuser

ENTRYPOINT ["/usr/bin/tini", "--"]
CMD ["csv_reader"]
