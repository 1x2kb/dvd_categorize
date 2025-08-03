FROM rust:latest

ARG UID=1000
ARG GID=1000
ARG USERNAME=appuser
RUN groupadd -g ${GID} ${USERNAME} && useradd -m -u ${UID} -g ${USERNAME} ${USERNAME}

# Install diesel CLI
RUN cargo install diesel_cli --no-default-features --features postgres

USER ${USERNAME}
WORKDIR /app
