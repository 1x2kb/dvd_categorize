FROM rust:latest

ARG UID=1000
ARG GID=1000
ARG USERNAME=appuser
RUN groupadd -g ${GID} ${USERNAME} && useradd -m -u ${UID} -g ${USERNAME} ${USERNAME}

# Install tini for proper signal handling
RUN apt-get update && apt-get install -y tini && rm -rf /var/lib/apt/lists/*

USER ${USERNAME}
WORKDIR /app

# Install cargo-watch for auto-reloading
RUN cargo install cargo-watch

# Use tini to ensure proper signal forwarding to cargo watch
ARG FEATURES=postgres,internet
ENV API_FEATURES=${FEATURES}

ENTRYPOINT ["/usr/bin/tini", "--"]
CMD ["sh", "-c", "set -f && cargo watch -i 'e2e/*' -i 'e2e/**' -x \"run -p dvd_catalog_api --no-default-features --features ${API_FEATURES}\""]