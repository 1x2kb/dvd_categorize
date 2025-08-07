FROM rust:latest

ARG UID=1000
ARG GID=1000
ARG USERNAME=appuser
RUN groupadd -g ${GID} ${USERNAME} && useradd -m -u ${UID} -g ${USERNAME} ${USERNAME}

USER ${USERNAME}
WORKDIR /app

CMD ["cargo", "run", "--bin", "csv_reader"]