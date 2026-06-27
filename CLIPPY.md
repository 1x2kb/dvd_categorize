# Clippy Strategy for Feature-Gated Code

This workspace uses feature flags extensively, with mutually exclusive backends (`postgres` vs `mongodb`). Standard `cargo clippy --workspace` will fail because some code only exists when specific features are enabled.

## Quick Start

Run clippy for each backend separately:

```bash
# PostgreSQL backend (most common)
make clippy-postgres
# or
./clippy-postgres.sh

# MongoDB backend
make clippy-mongodb
# or
./clippy-mongodb.sh
```

Both commands automatically commit any fixes applied by clippy.

## Feature Structure

### Database Backends (Mutually Exclusive)
- **postgres**: PostgreSQL with Diesel ORM
- **mongodb**: MongoDB with Qdrant vector store

### Per-Crate Features

#### `models`
- `postgres` - PostgreSQL-specific types and Diesel derives
- `postgres-types` - Postgres field structure without Diesel (frontend)
- `mongodb` - MongoDB-specific types (includes `vector-similarity`)
- `mongodb-types` - MongoDB field structure without dependencies (frontend)
- `vector-similarity` - Shared vector search types
- `ai` - Ollama integration types
- `testing` - Test utilities
- `text-matching` - Text search functionality

#### `database`
- `postgres` - PostgreSQL backend (enables `models/postgres`)
- `mongodb` - MongoDB backend (enables `models/mongodb`)
- `ai` - AI-related database operations
- `testing` - Test utilities and examples

#### `ai_chat`
- `internet` - Wikipedia RAG and web scraping support

#### `dvd_catalog_api`
- `postgres` - Use PostgreSQL backend
- `internet` - Enable internet features in AI chat

#### `csv_utils`
- `postgres` - PostgreSQL model support
- `vector-similarity` - Vector similarity types

#### `prompts`
- `conversions` - Ollama type conversions

#### `dvd_categorizer_web`
- `web` - Web/WASM target (default)
- `desktop` - Desktop application target
- `ai-backend` - Enable AI-powered UI pages (Chat, Live, Generator, Models) and search modes. When disabled, these routes show error messages and are hidden from the navbar.

## Why Two Scripts?

The workspace has mutually exclusive backends:

### `clippy-postgres.sh`
- Checks entire workspace with PostgreSQL backend enabled
- Enables: `database/postgres`, `dvd_catalog_api/postgres`, `dvd_catalog_api/ai`, `csv_utils/postgres`, `ai_chat/internet`, `dvd_categorizer_web/web`, `dvd_categorizer_web/ai-backend`
- Use for production code (PostgreSQL is the primary backend)

### `clippy-mongodb.sh`
- Checks entire workspace with MongoDB backend enabled
- Enables: `database/mongodb`, `dvd_catalog_api/mongodb`, `dvd_catalog_api/ai`, `ai_chat/internet`, `dvd_categorizer_web/web`, `dvd_categorizer_web/ai-backend`, `csv_utils/vector-similarity`
- Use when working on MongoDB backend features

**Run both scripts** to ensure all code paths are checked, since the backends are mutually exclusive.

## CI Integration

Add to your CI pipeline to check both backends:

```yaml
- name: Clippy (PostgreSQL)
  run: make clippy-postgres

- name: Clippy (MongoDB)
  run: make clippy-mongodb
```

## Common Issues

### "Cannot find X in this scope"
- Code is behind a feature flag that isn't enabled
- Run both `clippy-postgres.sh` and `clippy-mongodb.sh` to check all backend code paths

### "Unused import" warnings
- Import may only be used in certain feature configurations
- Use `#[cfg(feature = "...")]` on the import

### Mutually exclusive features
- Never enable both `postgres` and `mongodb` in the same build
- The scripts handle this automatically by running them separately
