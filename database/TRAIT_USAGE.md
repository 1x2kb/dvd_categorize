# Database Trait-Based Architecture

This document explains the new trait-based architecture for the database layer, designed to enable easy testing and dependency injection.

## Overview

The database layer has been refactored into **fine-grained, single-purpose traits** following the Interface Segregation Principle. Each database operation is its own trait that can be implemented independently by different backends (PostgreSQL, MongoDB, Redis, mocks, etc.). This enables:

- **Maximum flexibility** - Implement only the traits you need for each backend
- **Easy unit testing** - Mock implementations for individual operations
- **Dependency injection** - Functions accept specific trait bounds, not large interfaces
- **Backend flexibility** - Each database system can implement traits differently
- **No vendor lock-in** - Switch between PostgreSQL, document databases, etc.
- **Testability without database connections** - No Ollama or Postgres required for unit tests

## Available Traits

### Movie Traits

#### `GetAllMovies`
```rust
use database::{GetAllMovies, PostgresMovieRepository};

let mut repo = PostgresMovieRepository::new(conn);
let movies = repo.get_all().await?;
```

#### `GetMovieById`
```rust
use database::{GetMovieById, PostgresMovieRepository};

let mut repo = PostgresMovieRepository::new(conn);
let movie = repo.get_by_id(42).await?;
```

#### `GetMoviesByIds`
```rust
use database::{GetMoviesByIds, PostgresMovieRepository};

let mut repo = PostgresMovieRepository::new(conn);
let movies = repo.get_by_ids(vec![1, 2, 3]).await?;
```

#### `InsertMovie`
```rust
use database::{InsertMovie, PostgresMovieRepository};

let mut repo = PostgresMovieRepository::new(conn);
let inserted = repo.insert(full_movie).await?;
```

#### `InsertMovies`
```rust
use database::{InsertMovies, PostgresMovieRepository};

let mut repo = PostgresMovieRepository::new(conn);
let results = repo.insert_batch(&new_movies).await?;
```

#### `UpdateMovieLocation`
```rust
use database::{UpdateMovieLocation, PostgresMovieRepository};

let mut repo = PostgresMovieRepository::new(conn);
repo.update_location(movie_id, "A1".to_string()).await?;
```

#### `SearchMoviesByEmbedding`
```rust
use database::{SearchMoviesByEmbedding, PostgresMovieRepository};

let mut repo = PostgresMovieRepository::new(conn);
let movies = repo.search_by_embedding(embedding_vec, 10).await?;
```

#### `GetRecentMovies`
```rust
use database::{GetRecentMovies, PostgresMovieRepository};

let mut repo = PostgresMovieRepository::new(conn);
let recent = repo.get_recent(20).await?;
```

#### `SearchMoviesStructured`
```rust
use database::{SearchMoviesStructured, PostgresMovieRepository};

let mut repo = PostgresMovieRepository::new(conn);
let movies = repo.search_structured(&query).await?;
```

### Actor Traits

#### `InsertActors`
```rust
use database::{InsertActors, PostgresActorRepository};

let mut repo = PostgresActorRepository::new(conn);
let actors = repo.insert_batch(&new_actors).await?;
```

#### `GetActorsForMovie`
```rust
use database::{GetActorsForMovie, PostgresActorRepository};

let mut repo = PostgresActorRepository::new(conn);
let actors = repo.get_for_movie(&movie).await?;
```

#### `InsertMovieActorAssociations`
```rust
use database::{InsertMovieActorAssociations, PostgresActorRepository};

let mut repo = PostgresActorRepository::new(conn);
repo.insert_movie_associations(&associations).await?;
```

### Director Traits

#### `InsertDirectors`
```rust
use database::{InsertDirectors, PostgresDirectorRepository};

let mut repo = PostgresDirectorRepository::new(conn);
let directors = repo.insert_batch(&new_directors).await?;
```

### Genre Traits

#### `GetAllGenres`
```rust
use database::{GetAllGenres, PostgresGenreRepository};

let mut repo = PostgresGenreRepository::new(conn);
let all_genres = repo.get_all().await?;
```

#### `GetGenresForMovie`
```rust
use database::{GetGenresForMovie, PostgresGenreRepository};

let mut repo = PostgresGenreRepository::new(conn);
let genres = repo.get_for_movie(&movie).await?;
```

#### `InsertMovieGenreAssociations`
```rust
use database::{InsertMovieGenreAssociations, PostgresGenreRepository};

let mut repo = PostgresGenreRepository::new(conn);
repo.insert_movie_associations(&associations).await?;
```

### Embedding Traits

#### `GenerateEmbedding`
```rust
use database::{GenerateEmbedding, OllamaEmbeddingProvider};

let provider = OllamaEmbeddingProvider;
let embedding = provider.generate_embedding("some text").await?;
```

#### `GenerateEmbeddings`
```rust
use database::{GenerateEmbeddings, OllamaEmbeddingProvider};

let provider = OllamaEmbeddingProvider;
let embeddings = provider.generate_embeddings(texts).await?;
```

## Testing with Mocks

The real power of this architecture is in testing. Here's how to use mock implementations:

### Example: Testing Movie Search Logic

```rust
use database::{MockMovieRepository, GetMovieById, GetAllMovies};
use models::FullMovie;

#[tokio::test]
async fn test_movie_search() {
    // Create a mock repository with test data
    let test_movies = vec![
        FullMovie {
            id: 1,
            name: "The Matrix".to_string(),
            director: None,
            description: None,
            actors: vec![],
            genres: vec![],
            embedding: None,
            added_on: None,
            location: None,
        },
    ];
    
    let mut repo = MockMovieRepository::with_movies(test_movies);
    
    // Test your logic without database connection
    let movie = repo.get_by_id(1).await.unwrap();
    assert_eq!(movie.name, "The Matrix");
    
    let all = repo.get_all().await.unwrap();
    assert_eq!(all.len(), 1);
}
```

### Example: Testing with Mock Embedding Provider

```rust
use database::{MockEmbeddingProvider, GenerateEmbedding, GenerateEmbeddings};

#[tokio::test]
async fn test_embedding_generation() {
    let provider = MockEmbeddingProvider::new()
        .with_embedding("test movie".to_string(), vec![1.0, 2.0, 3.0]);
    
    let embedding = provider.generate_embedding("test movie").await.unwrap();
    assert_eq!(embedding, vec![1.0, 2.0, 3.0]);
    
    // Test batch generation
    let embeddings = provider.generate_embeddings(
        vec!["test movie".to_string(), "another".to_string()]
    ).await.unwrap();
    assert_eq!(embeddings.len(), 2);
}
```

## Using Traits with Generic Functions

You can write functions that accept any implementation of a specific trait. This is the key benefit of fine-grained traits - your functions only require what they actually need:

```rust
use database::{GetRecentMovies, GetMovieById, DatabaseError};
use models::FullMovie;

// Function only needs GetRecentMovies - works with ANY implementation
async fn process_recent_movies<R: GetRecentMovies>(
    repo: &mut R,
    limit: i64,
) -> Result<Vec<FullMovie>, DatabaseError> {
    repo.get_recent(limit).await
}

// Function needs both traits - use trait bounds
async fn get_and_display<R>(repo: &mut R, id: i32) -> Result<(), DatabaseError>
where
    R: GetMovieById + GetRecentMovies,
{
    let movie = repo.get_by_id(id).await?;
    println!("Movie: {}", movie.name);
    
    let recent = repo.get_recent(10).await?;
    println!("Found {} recent movies", recent.len());
    Ok(())
}

// Use with production PostgreSQL
let conn = get_database_connection().await?;
let mut pg_repo = PostgresMovieRepository::new(conn);
let movies = process_recent_movies(&mut pg_repo, 10).await?;

// Use with mock in tests - same interface!
let mut mock_repo = MockMovieRepository::new();
let movies = process_recent_movies(&mut mock_repo, 10).await?;
```

## Migrating Existing Code

### Before (Direct Function Calls)
```rust
use database::get_movies;

let movies = get_movies().await?;
```

### After (Using Fine-Grained Traits)
```rust
use database::{get_database_connection, PostgresMovieRepository, GetAllMovies};

let conn = get_database_connection().await?;
let mut repo = PostgresMovieRepository::new(conn);
let movies = repo.get_all().await?;
```

### Key Difference: Interface Segregation

The old approach would have been a single large `MovieRepository` trait with all methods:
```rust
// ❌ Old approach - monolithic trait
trait MovieRepository {
    fn get_all(...);
    fn get_by_id(...);
    fn get_by_ids(...);
    fn insert(...);
    // ... 10 more methods
}
```

The new approach uses fine-grained traits:
```rust
// ✅ New approach - single-purpose traits
trait GetAllMovies { fn get_all(...); }
trait GetMovieById { fn get_by_id(...); }
trait GetMoviesByIds { fn get_by_ids(...); }
trait InsertMovie { fn insert(...); }
// Each operation is its own trait!
```

**Why This Matters:**
- A document database might implement `GetMovieById` and `GetAllMovies` but not `GetMoviesByIds`
- A caching layer might only implement `GetMovieById`
- Your test mocks only need to implement the exact traits your code uses
- Functions can require only the specific capabilities they need

## Testing Best Practices

1. **Use mocks for unit tests** - No database or AI service required
2. **Use real implementations for integration tests** - Test against actual PostgreSQL
3. **Inject dependencies** - Pass repositories as trait objects or generic parameters
4. **Keep tests isolated** - Each test gets its own mock repository instance

## Example Test Suite

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use database::{MockMovieRepository, MovieRepository};

    #[tokio::test]
    async fn test_get_all_movies() {
        let mut repo = MockMovieRepository::new();
        let movies = repo.get_all().await.unwrap();
        assert_eq!(movies.len(), 0);
    }

    #[tokio::test]
    async fn test_insert_movie() {
        let mut repo = MockMovieRepository::new();
        let movie = FullMovie {
            id: 0,
            name: "Test Movie".to_string(),
            // ... other fields
        };
        
        let inserted = repo.insert(movie).await.unwrap();
        assert_eq!(inserted.id, 1);
        assert_eq!(inserted.name, "Test Movie");
    }

    #[tokio::test]
    async fn test_structured_search() {
        let test_movies = vec![
            // ... create test data
        ];
        let mut repo = MockMovieRepository::with_movies(test_movies);
        
        let query = StructuredQuery {
            actors: vec!["Tom Hanks".to_string()],
            // ... other criteria
        };
        
        let results = repo.search_structured(&query).await.unwrap();
        assert!(!results.is_empty());
    }
}
```

## Benefits

1. **No Database Setup for Unit Tests** - Mock implementations don't need PostgreSQL
2. **No AI Service for Tests** - Mock embedding provider returns deterministic values
3. **Faster Test Execution** - No network calls or database queries
4. **Easier to Test Edge Cases** - Create specific scenarios with mock data
5. **Better Code Organization** - Clear separation between interface and implementation
6. **Future-Proof** - Easy to add new implementations (Redis cache, different databases, etc.)
