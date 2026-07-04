# Environment Variables

This document lists all environment variables used by the DVD Categorizer application.

## Database Configuration

### `DATABASE_URL`
- **Required**: Yes (PostgreSQL backend)
- **Default**: None
- **Description**: PostgreSQL connection string for the database
- **Format**: `postgresql://user:password@host:port/database`
- **Example**: `postgresql://dvd_user:password@postgres:5432/dvd_catalog`
- **Used by**: `database`, `dvd_catalog_api`

### `MONGODB_URI`
- **Required**: Yes (MongoDB backend)
- **Default**: None
- **Description**: MongoDB connection string for the database
- **Format**: `mongodb://user:password@host:port/database?authSource=admin`
- **Example**: `mongodb://admin:mongopassword@mongodb:27017/dvd_catalog?authSource=admin`
- **Used by**: `database`, `dvd_catalog_api`

### `QDRANT_URL`
- **Required**: Yes (MongoDB backend)
- **Default**: None
- **Description**: Qdrant gRPC URL for vector search
- **Example**: `http://qdrant:6334`
- **Used by**: `database`, `dvd_catalog_api`

### `QDRANT_COLLECTION`
- **Required**: Yes (MongoDB backend)
- **Default**: `movies`
- **Description**: Name of the Qdrant collection that stores movie embeddings
- **Used by**: `database`, `dvd_catalog_api`

## Ollama Configuration

### `OLLAMA_HOST`
- **Required**: No
- **Default**: `ollama`
- **Description**: Hostname or IP address of the Ollama service
- **Used by**: `ai_chat`, `dvd_catalog_api`

### `OLLAMA_PORT`
- **Required**: No
- **Default**: `11434`
- **Description**: Port number for the Ollama service
- **Used by**: `ai_chat`, `dvd_catalog_api`

### `OLLAMA_NUM_CTX`
- **Required**: No
- **Default**: `8000`
- **Description**: Context window size for Ollama models (number of tokens)
- **Used by**: `ai_chat`

## API Server Configuration

### `SERVER_HOST`
- **Required**: No
- **Default**: `0.0.0.0`
- **Description**: Host address for the API server to bind to
- **Used by**: `dvd_catalog_api`

### `SERVER_PORT`
- **Required**: No
- **Default**: `3000`
- **Description**: Port number for the API server
- **Used by**: `dvd_catalog_api`

## Docker Build Configuration

These variables are used during Docker container builds and are typically set in the `.env` file.

### `UID`
- **Required**: No (for development)
- **Default**: Current user's UID
- **Description**: User ID for file permission mapping in development containers
- **Purpose**: Ensures files created by containers have correct ownership on the host

### `GID`
- **Required**: No (for development)
- **Default**: Current user's GID
- **Description**: Group ID for file permission mapping in development containers
- **Purpose**: Ensures files created by containers have correct ownership on the host

### `USERNAME`
- **Required**: No (for development)
- **Default**: Current username
- **Description**: Username for the container user in development
- **Purpose**: Creates a user inside the container matching the host user

### `POSTGRES_PASSWORD`
- **Required**: No
- **Default**: `strongpassword`
- **Description**: Password for the PostgreSQL database
- **Used by**: `postgres` service in Docker Compose

### `MONGO_PASSWORD`
- **Required**: No
- **Default**: `mongopassword`
- **Description**: Password for the MongoDB root user
- **Used by**: `mongodb` service in Docker Compose

## Example .env File

```bash
# PostgreSQL backend
DATABASE_URL=postgresql://dvd_user:secretpassword@postgres:5432/dvd_catalog
POSTGRES_PASSWORD=secretpassword

# MongoDB backend
MONGODB_URI=mongodb://admin:secretpassword@mongodb:27017/dvd_catalog?authSource=admin
MONGO_PASSWORD=secretpassword
QDRANT_URL=http://qdrant:6334
QDRANT_COLLECTION=movies

# Ollama
OLLAMA_HOST=ollama
OLLAMA_PORT=11434
OLLAMA_NUM_CTX=8000

# API Server
SERVER_HOST=0.0.0.0
SERVER_PORT=3000
```

## Notes

- Environment variables can be set in the `.env` file at the project root
- The `default.env` file provides a template with default values
- Docker Compose automatically loads variables from `.env`
- For production deployments, use secure methods to manage secrets (e.g., Docker secrets, environment variable injection)
- Never commit `.env` files with real credentials to version control
