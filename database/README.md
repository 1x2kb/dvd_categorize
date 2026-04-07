# Database Module

Async PostgreSQL database layer using Diesel ORM with pgvector support.

## Overview

This crate provides all database operations for the DVD Categorizer application, including:

- **CRUD Operations**: Create, read, update, delete for movies and related entities
- **Vector Search**: Semantic similarity search using PostgreSQL's pgvector extension
- **Structured Search**: Type-safe dynamic query building from structured criteria
- **Batch Operations**: Optimized bulk inserts and queries
- **Repository Pattern**: Clean abstractions for database access

## Architecture

### Modules

- **`actors.rs`**: Actor entity operations
- **`directors.rs`**: Director entity operations
- **`genres.rs`**: Genre operations
- **`movies.rs`**: Base movie operations
- **`full_movies.rs`**: Complete movie objects with all relationships
- **`embedding.rs`**: Vector embedding storage and retrieval
- **`structured_search.rs`**: Dynamic Diesel query building from structured criteria
- **`postgres.rs`**: Repository pattern implementations
- **`traits.rs`**: Trait definitions for database operations
- **`mocks.rs`**: Mock implementations for testing

## Key Features

### Vector Similarity Search

Uses PostgreSQL's pgvector extension for semantic search:

```rust
use database::search_movies;

let embedding = vec![0.1; 768]; // Example 768-dim vector
let similar_movies = search_movies(embedding, 10).await?;
```

### Structured Search

Build type-safe queries from structured criteria:

```rust
use database::structured_search::search_movies_structured;
use models::StructuredQuery;

let query = StructuredQuery {
    actors: vec!["brad pitt".to_string()],
    genres: vec!["action".to_string()],
    ..Default::default()
};

let results = search_movies_structured(&query, 50).await?;
```

### Batch Operations

Optimized for bulk operations:

```rust
use database::insert_full_movies;

let movies = vec![/* ... */];
insert_full_movies(movies).await?;
```

## Database Schema

The schema is managed by Diesel migrations in the `migrations/` directory.

### Core Tables

- **movie**: Main movie table with vector embeddings
- **actor**: Actor entities
- **director**: Director entities
- **movie_actor**: Many-to-many relationship between movies and actors
- **movie_genre**: Movie genre associations

See `TRAIT_USAGE.md` for detailed trait documentation and `Documentation/Diagrams/ER_DIAGRAM.md` for the entity-relationship diagram.

## Configuration

Requires the `DATABASE_URL` environment variable:

```bash
DATABASE_URL=postgresql://user:password@host:port/database
```

## Features

- **testing**: Enables mock implementations and test utilities

## Running Migrations

Migrations are automatically run by the `diesel_migrate` service in Docker Compose.

For manual migration:

```bash
diesel migration run
```

## Testing

```bash
cargo test -p database
```

## Dependencies

- `diesel`: ORM with async support
- `diesel-async`: Async query execution
- `pgvector`: Vector similarity search
- `tokio`: Async runtime
