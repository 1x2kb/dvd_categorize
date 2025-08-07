# Multi-stage build for production Web
FROM rust:latest as builder

WORKDIR /app

# Install dioxus CLI
RUN cargo install --locked dioxus-cli

# Copy all source code
COPY . .

# Build the web application for production
RUN dx build --release --package dvd_categorizer_web

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
