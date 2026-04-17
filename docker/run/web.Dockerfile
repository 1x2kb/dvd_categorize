FROM rust:latest

ARG UID=1000
ARG GID=1000
RUN groupadd -g ${GID} appuser && useradd -m -u ${UID} -g appuser appuser

# Install tini for proper signal handling
RUN apt-get update && apt-get install -y tini && rm -rf /var/lib/apt/lists/*

USER appuser
WORKDIR /app

ENV CARGO_HOME="/home/appuser/.cargo"
ENV RUSTUP_HOME="/home/appuser/.rustup"
ENV PATH="/home/appuser/.cargo/bin:${PATH}"

RUN rustup default stable

RUN cargo install --locked dioxus-cli --version 0.7.5

# Use tini to ensure proper signal forwarding to dx serve
ENTRYPOINT ["/usr/bin/tini", "--"]
CMD ["dx", "serve", "-p", "dvd_categorizer_web", "--addr", "0.0.0.0", "--port", "8080"]