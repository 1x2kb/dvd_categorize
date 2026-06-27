# Models Module

Shared domain types and data structures for the DVD Categorizer application.

## Overview

This crate contains all shared data models used across the application workspace. It uses feature flags to conditionally compile functionality based on the needs of each consuming crate.

## Feature Flags

### `postgres`
Enables Diesel ORM integration with PostgreSQL-specific types:
- Database schema definitions
- `Queryable`, `Insertable` derives
- `pgvector::Vector` support
- `chrono::NaiveDateTime` for timestamps

### `ai`
Enables AI-related types:
- `AiAction`, `AiState`
- Chat message types
- Model configuration

### `vector-similarity`
Enables vector similarity scoring:
- `VectorSimilarity` trait
- Cosine similarity calculations

### `text-matching`
Enables text-based search scoring:
- `TextMatchScoring` trait
- Keyword matching logic

### `testing`
Enables test utilities:
- `Random` trait for generating test data
- `FullMovie::create_test_movies()` helper
- Mock data generation

## Core Types

### Movie Types

```rust
pub struct FullMovie {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub actors: Vec<Actor>,
    pub director: Option<Director>,
    pub genres: Vec<String>,
    pub embedding: Option<Vec<f32>>,  // Feature-gated
    pub added_on: Option<String>,
    pub location: Option<String>,
}

pub struct Actor {
    pub id: i32,
    pub name: String,
}

pub struct Director {
    pub id: i32,
    pub name: String,
}
```

### Search Types

```rust
pub enum SearchMode {
    Text,       // Keyword-based entity extraction
    Vector,     // Semantic similarity search
    Both,       // Hybrid RRF fusion (default)
    Structured, // AI-parsed structured queries
}

pub struct SearchRequest {
    pub query: String,
    pub disable_enhancement: bool,
    pub search_mode: SearchMode,
    pub model: Option<String>,
}

pub struct SearchResponse {
    pub results: Vec<ScoredMovie>,
    pub original_query: String,
    pub enhanced_query: String,
}

pub struct StructuredQuery {
    pub actors: Vec<String>,
    pub directors: Vec<String>,
    pub genres: Vec<String>,
    pub title_keywords: Vec<String>,
    pub description_keywords: Vec<String>,
}
```

### API Types

```rust
pub struct ChatRequest {
    pub messages: Vec<RoledMessage>,
    pub model: Option<String>,
}

pub struct UpdateLocationRequest {
    pub movie_id: i32,
    pub location: String,
}

pub struct AvailableModel {
    pub name: String,
    pub size: u64,
}
```

## Traits

### VectorSimilarity (feature: `vector-similarity`)

```rust
pub trait VectorSimilarity {
    fn cosine_similarity(&self, query_embedding: &[f32]) -> Option<f32>;
}
```

Implemented for `FullMovie` to calculate cosine similarity between movie embeddings and query embeddings.

### TextMatchScoring (feature: `text-matching`)

```rust
pub trait TextMatchScoring {
    fn text_match_score(&self, titles: &[String], actors: &[String], genres: &[String]) -> usize;
}
```

Implemented for `FullMovie` to score movies based on keyword matches with weighted scoring:
- Title matches: weight 3
- Actor matches: weight 2
- Genre matches: weight 1

### Random (feature: `testing`)

```rust
pub trait Random {
    fn random() -> Self;
}
```

Generates random test data for `Actor`, `Director`, and `FullMovie`.

## Usage Examples

### Basic Usage

```rust
use models::{FullMovie, SearchMode, SearchRequest};

let request = SearchRequest {
    query: "science fiction".to_string(),
    disable_enhancement: false,
    search_mode: SearchMode::Text,
    model: None,
};
```

### With Testing Feature

```rust
#[cfg(test)]
use models::FullMovie;

let test_movies = FullMovie::create_test_movies();
// Returns well-known movies: The Matrix, Inception, Interstellar
```

### With Vector Similarity

```rust
use models::{FullMovie, VectorSimilarity};

let movie: FullMovie = /* ... */;
let query_embedding = vec![0.1; 768];

if let Some(score) = movie.cosine_similarity(&query_embedding) {
    println!("Similarity: {}", score);
}
```

## Dependencies

- `serde`: Serialization/deserialization
- `diesel`: ORM (optional, with `postgres` feature)
- `pgvector`: Vector types (optional, with `postgres` feature)
- `chrono`: Date/time (optional, with `postgres` feature)
- `rand`: Random generation (optional, with `testing` feature)

## Schema

The database schema is defined in `schema.rs` (when `postgres` feature is enabled) and managed by Diesel migrations.
