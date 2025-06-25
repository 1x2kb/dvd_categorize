# Builder stage
FROM rust:latest AS builder

WORKDIR /app

# Copy entire workspace including all crates
COPY . .

# Install Dioxus CLI
RUN cargo install --locked dioxus-cli

# Build web crate
WORKDIR /app/dvd_categorizer_web
RUN dx bundle -p dvd_categorizer_web --platform web --release

# Runtime stage
FROM nginx:alpine
COPY --from=builder /app/target/dx/dvd_categorizer_web/release/web/public /usr/share/nginx/html
COPY docker/nginx.conf /etc/nginx/conf.d/default.conf
