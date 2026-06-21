FROM rust:latest

# Install tini for proper signal handling
RUN apt-get update && apt-get install -y tini && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Install cargo-watch for auto-reloading
RUN cargo install cargo-watch

# Copy source code into the image
COPY . .

# Use tini to ensure proper signal forwarding to cargo watch
ARG FEATURES=postgres,internet
ENV API_FEATURES=${FEATURES}

ENTRYPOINT ["/usr/bin/tini", "--"]
CMD ["sh", "-c", "set -f && cargo clean -p dvd_catalog_api && cargo watch -i 'e2e/*' -i 'e2e/**' -x \"run -p dvd_catalog_api --no-default-features --features ${API_FEATURES}\""]