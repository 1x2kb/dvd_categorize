# Phase 2: Connection Pooling + Tool Calling

## Status: Ready to implement (separate session)

## Problem
Current `PostgresMovieRepository` holds single `AsyncPgConnection`:
- ✗ Not `Send + Sync` - can't share across threads
- ✗ Can't run concurrent queries
- ✗ Blocks tool calling implementation

## Solution: Connection Pooling

### Changes Required

#### 1. Update PostgresMovieRepository
```rust
// Before
pub struct PostgresMovieRepository {
    connection: AsyncPgConnection,
}

// After
use diesel_async::pooled_connection::deadpool::Pool;

#[derive(Clone)]
pub struct PostgresMovieRepository {
    pool: Pool<AsyncPgConnection>,
}

impl PostgresMovieRepository {
    pub fn new(pool: Pool<AsyncPgConnection>) -> Self {
        Self { pool }
    }
    
    pub async fn from_env() -> Result<Self, DatabaseError> {
        let pool = crate::get_connection_pool().await?;
        Ok(Self::new(pool))
    }
}
```

#### 2. Update All Trait Implementations
Change `&mut self` to `&self`, borrow connection from pool:

```rust
#[async_trait]
impl GetAllMovies for PostgresMovieRepository {
    // Before: async fn get_all(&mut self) -> Result<Vec<FullMovie>, DatabaseError>
    async fn get_all(&self) -> Result<Vec<FullMovie>, DatabaseError> {
        let mut conn = self.pool.get().await?;
        
        let movies: Vec<Movie> = schema::movie::table
            .load(&mut conn)
            .await?;
        
        // ... rest of implementation
    }
}
```

#### 3. Update Trait Definitions
In `database/src/traits.rs`, change all `&mut self` to `&self`:

```rust
#[async_trait]
pub trait GetMovieById: Send + Sync {
    async fn get_by_id(&self, id: i32) -> Result<FullMovie, DatabaseError>;
    //                 ^^^^^ not &mut
}

#[async_trait]
pub trait GetMoviesByIds: Send + Sync {
    async fn get_by_ids(&self, ids: Vec<i32>) -> Result<Vec<FullMovie>, DatabaseError>;
}

// ... etc for all traits
```

#### 4. Create Pool Initialization
In `database/src/lib.rs`:

```rust
use diesel_async::pooled_connection::deadpool::{Pool, Manager};
use diesel_async::AsyncPgConnection;

pub async fn get_connection_pool() -> Result<Pool<AsyncPgConnection>, DatabaseError> {
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL environment variable must be set");
    
    let config = deadpool::managed::PoolConfig::default();
    let manager = Manager::new(database_url, deadpool::Runtime::Tokio1);
    let pool = Pool::builder(manager)
        .config(config)
        .build()
        .map_err(|e| DatabaseError::ConnectionError(
            diesel::ConnectionError::BadConnection(e.to_string())
        ))?;
    
    Ok(pool)
}

// Keep old function for backwards compat, but mark deprecated
#[deprecated(note = "Use get_connection_pool() instead")]
pub async fn get_database_connection() -> Result<AsyncPgConnection, DatabaseError> {
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL environment variable must be set");
    AsyncPgConnection::establish(&database_url).await
}
```

#### 5. Update All Call Sites
Find all places that call `get_database_connection()` and update:

```bash
# Find usage
grep -r "get_database_connection" --include="*.rs"

# Common patterns to update:
# Before:
let mut conn = database::get_database_connection().await?;
let mut repo = PostgresMovieRepository::new(conn);

# After:
let repo = database::PostgresMovieRepository::from_env().await?;
```

### Benefits After Refactor

1. **Concurrent Queries**
```rust
// Can now run in parallel!
let (movies, actors, genres) = tokio::join!(
    repo.get_all_movies(),
    repo.get_all_actors(),
    repo.get_all_genres(),
);
```

2. **Better get_full_movie**
```rust
async fn get_full_movie(&self, id: i32) -> Result<FullMovie, DatabaseError> {
    let mut conn = self.pool.get().await?;
    
    // Get movie
    let movie: Movie = schema::movie::table
        .find(id)
        .first(&mut conn)
        .await?;
    
    // Parallel fetch related data
    let (actors, genres, director) = tokio::join!(
        self.get_actors_for_movie(id),
        self.get_genres_for_movie(id),
        self.get_director_for_movie(movie.director_id),
    );
    
    // Assemble FullMovie
    Ok(FullMovie { /* ... */ })
}
```

3. **Tool Calling Works**
```rust
// ai_tools/src/lib.rs now compiles!
#[ollama_rs::function]
pub async fn search_by_text(
    search_text: String,
    limit: Option<usize>,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let repo = database::PostgresMovieRepository::from_env().await?;
    // repo is Clone + Send + Sync ✓
    
    let embedding = ai_chat::get_embedding(&search_text).await?;
    let movies = repo.search_by_embedding(embedding, limit as i64).await?;
    
    Ok(serde_json::to_string(&movies)?)
}
```

## Testing Plan

1. Run existing tests - should all pass
2. Test concurrent queries:
```rust
#[tokio::test]
async fn test_concurrent_queries() {
    let repo = PostgresMovieRepository::from_env().await.unwrap();
    
    let (r1, r2, r3) = tokio::join!(
        repo.get_all(),
        repo.get_by_id(1),
        repo.get_by_ids(vec![1, 2, 3]),
    );
    
    assert!(r1.is_ok());
    assert!(r2.is_ok());
    assert!(r3.is_ok());
}
```

3. Test tool calling with ai_tools crate
4. Load test - verify pool handles concurrent requests

## Estimated Time
2-3 hours for careful refactoring + testing

## Files to Update
- `database/src/lib.rs` - pool initialization
- `database/src/postgres.rs` - repository struct + all impls
- `database/src/traits.rs` - change &mut self to &self
- `dvd_catalog_api/src/*.rs` - update call sites
- `ai_tools/src/lib.rs` - tool implementations
- Any other crates using database

## Dependencies Added
- ✓ `diesel-async` with "deadpool" feature
- ✓ `deadpool` workspace dependency
