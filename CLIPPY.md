# Clippy Strategy for Feature-Gated Code

This workspace uses feature flags extensively, with mutually exclusive backends (`postgres` vs `mongodb`). Standard `cargo clippy --workspace` will fail because some code only exists when specific features are enabled.

## Quick Start

```bash
# Fast check covering main feature combinations (recommended for CI/pre-commit)
make clippy-quick
# or
./clippy-quick.sh

# Comprehensive check of all feature combinations (slower but thorough)
make clippy-all
# or
./clippy-all.sh
```

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

## Why Two Scripts?

### `clippy-quick.sh` (Recommended)
- Checks workspace with both `postgres` and `mongodb` backends
- Covers most common feature combinations
- Faster execution (~2-3 minutes)
- Ideal for CI pipelines and pre-commit hooks

### `clippy-all.sh` (Comprehensive)
- Checks every crate individually with all feature permutations
- Ensures 100% code coverage across all feature gates
- Slower execution (~5-10 minutes)
- Use before major releases or when modifying feature-gated code

## CI Integration

Add to your CI pipeline:

```yaml
- name: Clippy (all features)
  run: make clippy-quick
```

For thorough checks on release branches:

```yaml
- name: Clippy (comprehensive)
  run: make clippy-all
```

## Common Issues

### "Cannot find X in this scope"
- Code is behind a feature flag that isn't enabled
- Run `clippy-all.sh` to check all feature combinations

### "Unused import" warnings
- Import may only be used in certain feature configurations
- Use `#[cfg(feature = "...")]` on the import

### Mutually exclusive features
- Never enable both `postgres` and `mongodb` in the same build
- The scripts handle this automatically
