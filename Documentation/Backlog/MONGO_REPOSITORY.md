# MongoDB Repository Plan

## Goal

Feature-gate the database crate so `postgres` and `mongo` are mutually exclusive backends at compile time, with `MongoMovieRepository` at full functional parity with `PostgresMovieRepository`.

All traits in `database/src/traits.rs` must be implemented. Any concrete methods on `PostgresMovieRepository` that are not yet behind a trait must be promoted to traits first so Mongo has a contract to satisfy.

---

## Traits to Implement on `MongoMovieRepository`

### Movie
- `GetMovieById`
- `GetMoviesByIds`
- `GetAllMovies`
- `InsertMovie`
- `InsertMovies`
- `UpdateMovieLocation`
- `SearchMoviesByEmbedding`
- `GetRecentMovies`
- `RandomMovies`
- `GetMoviesByReleaseYear`
- `GetUnknownLocationMovies`
- `GetUniqueLocations`
- `MoviesByLocation`
- `SearchMoviesStructured`

### Actor
- `InsertActors`
- `GetActorsForMovie`
- `InsertMovieActorAssociations`

### Director
- `InsertDirectors`

### Genre
- `GetAllGenres`
- `GetGenresForMovie`
- `InsertMovieGenreAssociations`

---

## Cargo Feature Gate

`database/Cargo.toml` needs:
- `postgres` feature enabling Diesel, diesel-async, deadpool, pgvector
- `mongo` feature enabling mongodb and Qdrant as the vector store
- Both features mutually exclusive — enforced by convention or a compile-time cfg check

`database/src/lib.rs` needs:
- `postgres` module gated behind `#[cfg(feature = "postgres")]`
- `mongo` module gated behind `#[cfg(feature = "mongo")]`

---

## Axum DbState Design

`DbState` holds any type that satisfies the full repository trait surface. A `MovieRepository` supertrait is defined in `database` to collect all required traits, avoiding repeated bounds at every call site.

```rust
pub trait MovieRepository:
    GetMovieById + GetMoviesByIds + GetAllMovies +
    InsertMovie + InsertMovies + UpdateMovieLocation +
    SearchMoviesByEmbedding + SearchMoviesStructured +
    GetRecentMovies + RandomMovies + GetMoviesByReleaseYear +
    GetUnknownLocationMovies + GetUniqueLocations + MoviesByLocation +
    Send + Sync {}
```

Blanket impl so any type satisfying all bounds automatically qualifies:
```rust
impl<T> MovieRepository for T where T: ... {}
```

`DbState` is then generic over the repository:
```rust
pub struct DbState<R: MovieRepository> {
    pub pool: R,
}
```

`DbState` never changes when switching backends. Axum handlers are generic over `R: MovieRepository` or use the concrete type at the app entry point. The active backend is resolved at compile time with no dynamic dispatch.

---

## Structured Search

`database/src/structured_search.rs` is Diesel-specific and has no Mongo equivalent. A new `database/src/mongo_search.rs` (or similar) must implement the same logic as a MongoDB aggregation pipeline.

---

## Vector Store

All vector search must use self-hosted, free options only. Atlas is excluded — it is not self-hostable and has associated costs.

**Chosen: Qdrant** — runs as a Docker container, free and open source, has a first-class Rust client (`qdrant-client`). `SearchMoviesByEmbedding` will delegate to Qdrant instead of pgvector.

Remove Redis from `docker-compose.yml` (and production equivalent) entirely. It was used for vector search on a previous branch and has no remaining role in this architecture.

---

## Notes

- `search_movies_by_text` has an N+1 query problem in Postgres — the Mongo equivalent resolves this natively via embedded documents
- Embedding generation is the caller's responsibility (e.g. `dvd_catalog_api`, standalone tools); `database` only stores and queries embeddings
- Actor/director/genre normalization goes away in Mongo — all stored as embedded arrays on the movie document; the actor/director/genre trait impls may become no-ops or be removed under the `mongo` feature
