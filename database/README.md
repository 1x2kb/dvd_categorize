# Database Module

Database abstraction layer supporting PostgreSQL (via Diesel async) and MongoDB (via the official MongoDB driver).

## Overview

This crate provides all database operations for the DVD Categorizer application, including:

- **CRUD Operations**: Create, read, update, delete for movies and related entities
- **Vector Search**: Semantic similarity search using PostgreSQL's pgvector extension or Qdrant, depending on the active backend
- **Structured Search**: Type-safe dynamic query building from structured criteria (PostgreSQL backend)
- **Batch Operations**: Optimized bulk inserts and queries
- **Repository Pattern**: Clean abstractions for database access

## Architecture

### Modules

- **`actors.rs`**: Actor entity operations
- **`directors.rs`**: Director entity operations
- **`genres.rs`**: Genre operations
- **`movies.rs`**: Base movie operations
- **`full_movies.rs`**: Complete movie objects with all relationships
- **`embedding.rs`**: Vector embedding helpers for Qdrant
- **`structured_search.rs`**: Dynamic Diesel query building from structured criteria
- **`postgres.rs`**: PostgreSQL repository implementation
- **`mongodb.rs`**: MongoDB repository implementation
- **`traits.rs`**: Trait definitions for database operations
- **`mocks.rs`**: Mock implementations for testing

## Key Features

### Vector Similarity Search

Embeddings are generated with `nomic-embed-text` and stored in the active backend's vector store:

- **PostgreSQL** — stored in a `pgvector` column and queried with cosine distance.
- **MongoDB** — stored in Qdrant and queried by cosine similarity.

```rust
use database::search_movies;

let embedding = vec![0.1; 768]; // Example 768-dim vector
let similar_movies = search_movies(embedding, 10).await?;
```

### Structured Search (PostgreSQL)

Build type-safe Diesel queries from structured criteria:

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

### PostgreSQL

The PostgreSQL schema is managed by Diesel migrations in the `migrations/` directory.

### Core Tables

- **movie**: Main movie table
- **actor**: Actor entities
- **director**: Director entities
- **movie_actor**: Many-to-many relationship between movies and actors
- **movie_genre**: Movie genre associations

See `TRAIT_USAGE.md` for detailed trait documentation and `Documentation/Diagrams/ER_DIAGRAM.md` for the PostgreSQL entity-relationship diagram.

### MongoDB

MongoDB stores movie documents in a single collection. Vector embeddings are stored separately in Qdrant.

## Configuration

### PostgreSQL

Requires the `DATABASE_URL` environment variable:

```bash
DATABASE_URL=postgresql://user:password@host:port/database
```

### MongoDB

Requires the `MONGODB_URI` and Qdrant variables:

```bash
MONGODB_URI=mongodb://user:password@mongodb:27017/dvd_catalog?authSource=admin
QDRANT_URL=http://qdrant:6334
QDRANT_COLLECTION=movies
```

## Features

- **postgres**: PostgreSQL repository implementation
- **mongodb**: MongoDB repository implementation
- **testing**: Enables mock implementations and test utilities

## Running Migrations

Migrations are automatically run by the `diesel_migrate` service in Docker Compose when the PostgreSQL profile is active.

For manual migration:

```bash
diesel migration run
```

## Testing

```bash
cargo test -p database
```

## Dependencies

- `diesel`: ORM with async support (PostgreSQL)
- `diesel-async`: Async query execution (PostgreSQL)
- `mongodb`: MongoDB driver
- `qdrant-client`: Vector search client (MongoDB backend)
- `tokio`: Async runtime
