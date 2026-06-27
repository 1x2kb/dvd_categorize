FROM rust:latest

# Install tini for proper signal handling
RUN apt-get update && apt-get install -y tini && rm -rf /var/lib/apt/lists/*

WORKDIR /app

RUN rustup default stable

RUN cargo install --locked dioxus-cli --version 0.7.9

# Copy source code into the image
COPY . .

# Use tini to ensure proper signal forwarding to dx serve
ARG FEATURES=postgres-api
ENV WEB_FEATURES=${FEATURES}

# Echo features for debugging
RUN echo "========================================" && \
    echo "Building Web Frontend with features:" && \
    echo "  ${FEATURES}" && \
    echo "========================================"

ENTRYPOINT ["/usr/bin/tini", "--"]
CMD ["sh", "-c", "echo 'Starting dx serve with features: ${WEB_FEATURES}' && dx serve --platform web -p dvd_categorizer_web --addr 0.0.0.0 --port 8080 --no-default-features --features ${WEB_FEATURES}"]