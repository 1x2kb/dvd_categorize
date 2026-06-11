# Postgres Feature Implementation Plan

## Overview
This document outlines the systematic approach to feature-locking the Postgres database implementation to prepare for adding MongoDB as an alternative database.

## Current State
- Basic postgres feature flag added to Cargo.toml files
- Partial feature gating applied across multiple files
- Cascading compilation errors due to over-gating of shared types

## Implementation Strategy

### Phase 1: Consolidate Postgres Code
1. **Move all postgres/diesel dependent code into postgres.rs**
   - Move functions from actors.rs, directors.rs, genres.rs, movies.rs, full_movies.rs, structured_search.rs
   - Create internal modules within postgres.rs for organization
   - Feature gate the entire postgres.rs module at once

### Phase 2: Update Database Crate Structure
1. **Reorganize lib.rs**
   - Keep shared types always available (FullMovie, DatabaseError, traits)
   - Only gate postgres-specific implementations
   - Remove individual feature gates from scattered files

### Phase 3: Update API Crate
1. **Feature gate API endpoints that depend on postgres**
   - Create conditional compilation blocks for entire endpoint groups
   - Provide fallback implementations for non-postgres builds
   - Update DbState to be conditional but always available

### Phase 4: Update Docker Compose
1. **Add postgres feature flag to API service**
   - Update docker-compose.yml to run API with --features postgres
   - Ensure proper feature flag propagation

## Detailed Steps

### Step 1: Consolidate Postgres Code
- [ ] Move all postgres functions into postgres.rs as internal modules
- [ ] Remove individual feature gates from other database files
- [ ] Test that postgres.rs compiles correctly with feature gate

### Step 2: Clean Database Crate
- [ ] Update lib.rs to only gate postgres module
- [ ] Keep shared types (FullMovie, DatabaseError, traits) always available
- [ ] Ensure database crate compiles with and without postgres feature

### Step 3: Update API Crate
- [ ] Feature gate entire API endpoint groups
- [ ] Update DbState to be conditional struct
- [ ] Add fallback implementations for non-postgres builds
- [ ] Fix imports to use correct crate sources

### Step 4: Update Docker Configuration
- [ ] Update docker-compose.yml to include postgres feature
- [ ] Test that API starts correctly with postgres feature

### Step 5: Final Testing
- [ ] Test compilation with --features postgres
- [ ] Test compilation without postgres feature
- [ ] Verify API functionality in both configurations

## Key Principles
1. **Only gate implementations, not interfaces**
2. **Keep shared types always available**
3. **Provide meaningful fallbacks for non-postgres builds**
4. **Feature gate at module level, not individual function level**
5. **Test both configurations thoroughly**

## Files to Modify
- `database/src/lib.rs` - Remove scattered feature gates
- `database/src/postgres.rs` - Consolidate all postgres code
- `database/src/actors.rs` - Move postgres functions, keep shared
- `database/src/directors.rs` - Move postgres functions, keep shared
- `database/src/genres.rs` - Move postgres functions, keep shared
- `database/src/movies.rs` - Move postgres functions, keep shared
- `database/src/full_movies.rs` - Move postgres functions, keep shared
- `database/src/structured_search.rs` - Move postgres functions, keep shared
- `dvd_catalog_api/src/lib.rs` - Feature gate endpoint groups
- `dvd_catalog_api/src/main.rs` - Update for conditional DbState
- `dvd_catalog_api/src/movie_search.rs` - Feature gate postgres-dependent endpoints
- `docker-compose.yml` - Add postgres feature flag

## Success Criteria
- Project compiles with `--features postgres`
- Project compiles without postgres feature
- API runs in both configurations
- Postgres-specific code is properly isolated
- Shared types remain available in all configurations

## Detailed Implementation Steps

### Phase 1: Consolidate Postgres Code (Step-by-Step)

#### 1.1 Update database/Cargo.toml
```toml
[features]
testing = ["rand"]
postgres = []  # Remove this line - make postgres always available
```

#### 1.2 Consolidate postgres functions into postgres.rs
Create internal modules within `database/src/postgres.rs`:

```rust
// Add at the end of postgres.rs
#[cfg(feature = "postgres")]
pub mod actors {
    use super::*;
    // Move insert_actors, insert_movie_actors, actors_for_movie here
}

#[cfg(feature = "postgres")]
pub mod directors {
    use super::*;
    // Move insert_directors here
}

#[cfg(feature = "postgres")]
pub mod genres {
    use super::*;
    // Move get_all_genres, insert_movie_genres, genres_for_movie here
}

#[cfg(feature = "postgres")]
pub mod movies {
    use super::*;
    // Move insert_movies, update_movie_location here
}

#[cfg(feature = "postgres")]
pub mod full_movies {
    use super::*;
    // Move insert_full_movies and helper functions here
}

#[cfg(feature = "postgres")]
pub mod structured_search {
    use super::*;
    // Move search_movies_structured and helper functions here
}

// Re-export for backward compatibility
#[cfg(feature = "postgres")]
pub use actors::*;
#[cfg(feature = "postgres")]
pub use directors::*;
#[cfg(feature = "postgres")]
pub use genres::*;
#[cfg(feature = "postgres")]
pub use movies::*;
#[cfg(feature = "postgres")]
pub use full_movies::*;
#[cfg(feature = "postgres")]
pub use structured_search::*;
```

#### 1.3 Remove individual feature gates from scattered files
Remove `#[cfg(feature = "postgres")]` from:
- `database/src/actors.rs` - Remove all feature gates
- `database/src/directors.rs` - Remove all feature gates  
- `database/src/genres.rs` - Remove all feature gates
- `database/src/movies.rs` - Remove all feature gates
- `database/src/full_movies.rs` - Remove all feature gates
- `database/src/structured_search.rs` - Remove all feature gates

### Phase 2: Update Database Crate Structure

#### 2.1 Update database/src/lib.rs
```rust
// Keep shared types always available
pub use actors::*;
pub use directors::*;
pub use embedding::*;
pub use full_movies::*;
pub use genres::*;
pub use movies::*;
pub use postgres::*;  // This will be empty without postgres feature
pub use structured_search::*;
pub use traits::*;
pub use models::{schema::*, *}; // FullMovie, DatabaseError, etc.

// Remove individual feature gates from DatabaseError enum
pub enum DatabaseError {
    ConnectionError(ConnectionError),  // Remove feature gate
    DieselError(diesel::result::Error),
    QueryError(String),
}

// Remove feature gates from helper functions
pub(crate) async fn load_actors_for_movies(...) { ... }
pub(crate) async fn load_genres_for_movies(...) { ... }
pub(crate) async fn get_connection_pool() -> Result<Pool<AsyncPgConnection>, DatabaseError> { ... }
```

### Phase 3: Update API Crate

#### 3.1 Update dvd_catalog_api/Cargo.toml
```toml
[dependencies]
database = { path = "../database" }  # Remove optional = true

[features]
internet = ["ai_chat/internet"]
# Remove postgres feature - make database always available
```

#### 3.2 Update dvd_catalog_api/src/lib.rs
```rust
// Remove feature gates from imports
use database::{
    traits::{
        GetAllMovies, GetMovieById, GetMoviesByReleaseYear, GetRecentMovies, GetUniqueLocations,
        GetUnknownLocationMovies, InsertMovie, MoviesByLocation, RandomMovies,
    },
    FullMovie, PostgresMovieRepository, SearchRequest,
};

// Make DbState always available but with conditional pool field
#[derive(Clone)]
pub struct DbState {
    pub pool: PostgresMovieRepository,
}

// Remove conditional DbState implementation
impl DbState {
    pub fn new(pool: PostgresMovieRepository) -> Self {
        Self { pool }
    }
}

// Remove feature gates from database calls
async fn insert_movies_batch(...) {
    // Remove #[cfg(feature = "postgres")]
    database::insert_full_movies(movies, &state.pool).await
}

async fn combined_search(...) {
    // Remove #[cfg(feature = "postgres")] from repo parameter
    // Remove conditional compilation blocks
}
```

#### 3.3 Update dvd_catalog_api/src/main.rs
```rust
// Remove feature gates from imports
use database::PostgresMovieRepository;

// Remove conditional compilation
async fn main() {
    let db_pool = PostgresMovieRepository::from_env().await.unwrap();
    let app = init_router(db_pool);  // Only one init_router function
}

// Remove init_router_no_db function
fn init_router(db_pool: PostgresMovieRepository) -> Router {
    let state = DbState { pool: db_pool };
    // ... rest of router setup
}
```

#### 3.4 Update dvd_catalog_api/src/movie_search.rs
```rust
// Remove feature gates from imports
use database::{
    question::AiAction,
    traits::{GetAllMovies, SearchMoviesByEmbedding},
    FullMovie, PostgresMovieRepository,
};

// Remove feature gates from function calls
async fn structured_search_handler(...) {
    let pool = database::get_connection_pool().await.unwrap();
    // Remove #[cfg(feature = "postgres")]
    match database::structured_search::search_movies_structured(...) { ... }
}
```

### Phase 4: Update Docker Configuration

#### 4.1 Update docker-compose.yml
```yaml
services:
  api:
    build:
      context: ./dvd_catalog_api
      dockerfile: Dockerfile
    # Add postgres feature flag
    command: ["--features", "postgres"]
    environment:
      - DATABASE_URL=postgres://postgres:password@postgres:5432/dvd_catalog
    depends_on:
      - postgres
```

## Troubleshooting Guide

### Common Compilation Issues

#### Issue 1: "cannot find type `FullMovie` in this scope"
**Cause**: Shared types are being feature-gated
**Solution**: Ensure `FullMovie` is always exported from database/src/lib.rs:
```rust
pub use models::{schema::*, *};
```

#### Issue 2: "no field `pool` on type `DbState`"
**Cause**: Conditional struct fields
**Solution**: Make DbState struct unconditional:
```rust
#[derive(Clone)]
pub struct DbState {
    pub pool: database::PostgresMovieRepository,
}
```

#### Issue 3: "unresolved import `database::PostgresMovieRepository`"
**Cause**: PostgresMovieRepository is feature-gated
**Solution**: Ensure postgres.rs always exports PostgresMovieRepository when available

#### Issue 4: Async/sync diesel mismatches
**Cause**: Awaiting synchronous diesel results
**Solution**: Check diesel async usage in postgres.rs functions

### Verification Steps

#### Step 1: Test Compilation
```bash
# Test with postgres feature
cargo check --features postgres

# Test without postgres feature  
cargo check

# Test full build
cargo build --features postgres
cargo build
```

#### Step 2: Test API Functionality
```bash
# Start with postgres
docker-compose up --build

# Test endpoints
curl http://localhost:3000/dvd
curl -X POST http://localhost:3000/search -d '{"query":"test"}'
```

#### Step 3: Verify Feature Isolation
```bash
# Check that postgres-specific code is only included when feature is enabled
cargo expand --features postgres | grep -i "postgres\|diesel"
```

## Implementation Checklist

### Database Crate Changes
- [ ] Remove postgres feature from database/Cargo.toml
- [ ] Consolidate postgres functions into postgres.rs modules
- [ ] Remove feature gates from scattered database files
- [ ] Update lib.rs to export shared types unconditionally
- [ ] Test database crate compilation with/without postgres feature

### API Crate Changes  
- [ ] Remove postgres feature from dvd_catalog_api/Cargo.toml
- [ ] Update DbState to be unconditional
- [ ] Remove feature gates from database imports and calls
- [ ] Update main.rs to remove conditional compilation
- [ ] Update movie_search.rs to remove feature gates
- [ ] Test API crate compilation

### Docker Changes
- [ ] Update docker-compose.yml with postgres feature flag
- [ ] Test API startup with docker-compose
- [ ] Verify database connectivity

### Final Verification
- [ ] Full workspace compiles with --features postgres
- [ ] Full workspace compiles without postgres feature
- [ ] API runs successfully in Docker with postgres
- [ ] All endpoints work correctly
- [ ] No lint errors or warnings

## Notes for Future MongoDB Integration

When adding MongoDB as an alternative database:
1. Create `mongodb.rs` file similar to `postgres.rs`
2. Add `mongodb` feature flag
3. Create conditional compilation blocks for mongodb vs postgres
4. Keep the same shared interface (traits, FullMovie, etc.)
5. Update DbState to support both database types

The current implementation provides a clean foundation for adding multiple database backends while maintaining API compatibility.
