# Multi-stage build for production Web
FROM rust:bookworm AS builder

WORKDIR /app

# Install dioxus CLI (match runtime dioxus version in Cargo.lock)
RUN cargo install --locked dioxus-cli --version 0.7.9

# Copy all source code
COPY . .

# Build the web application for production
ARG FEATURES=postgres-api
RUN dx build --release --package dvd_categorizer_web --features ${FEATURES}

# Runtime stage with nginx
FROM nginx:alpine

# Copy built assets from builder stage
COPY --from=builder /app/target/dx/dvd_categorizer_web/release/web/public/ /usr/share/nginx/html/

# Copy nginx configuration
COPY docker/run/nginx.conf /etc/nginx/conf.d/default.conf

# Set proper permissions for nginx content
RUN chown -R nginx:nginx /usr/share/nginx/html

EXPOSE 8080

CMD ["nginx", "-g", "daemon off;"]
