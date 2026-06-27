# Multi-stage build for production API
FROM rust:bookworm AS builder

WORKDIR /app

# Copy all source code
COPY . .

# Build the API application
ARG FEATURES=internet,ai,postgres
RUN echo "========================================" && \
    echo "Building API (PRODUCTION) with features:" && \
    echo "  ${FEATURES}" && \
    echo "========================================" && \
    cargo build --release --package dvd_catalog_api --no-default-features --features ${FEATURES}

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
COPY --from=builder /app/target/release/dvd_catalog_api /usr/local/bin/dvd_catalog_api

# Set ownership
RUN chown appuser:appuser /usr/local/bin/dvd_catalog_api

USER appuser

EXPOSE 3000

ENTRYPOINT ["/usr/bin/tini", "--"]
CMD ["dvd_catalog_api"]
