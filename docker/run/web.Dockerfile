FROM rust:latest

ARG UID=1000
ARG GID=1000
RUN groupadd -g ${GID} appuser && useradd -m -u ${UID} -g appuser appuser

USER appuser
WORKDIR /app

ENV CARGO_HOME="/home/appuser/.cargo"
ENV RUSTUP_HOME="/home/appuser/.rustup"
ENV PATH="/home/appuser/.cargo/bin:${PATH}"

RUN rustup default stable

RUN cargo install --locked dioxus-cli

# Configure dx serve with production features
CMD ["dx", "serve", "-p", "dvd_categorizer_web"]