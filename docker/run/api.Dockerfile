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
ENTRYPOINT ["/usr/bin/tini", "--"]
CMD ["cargo", "watch", "-i", "e2e/*", "-i", "e2e/**", "-x", "run -p dvd_catalog_api --features postgres,internet"]