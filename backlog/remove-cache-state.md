# Remove CacheState

Replace in-memory movie cache with direct DB reads via `DbState`/`PostgresMovieRepository`.

## Handlers using CacheState

- `get_dvds` — reads `state.movies` cache → replace with `repo.get_all()`
- `export_csv` — reads `cache_state.movies` → replace with `repo.get_all()`
- `parse_csv` — uses `cache_state.repo.pool()` for insert → move to `DbState`
- `validate_titles` — reads `cache.movies` → replace with `repo.get_all()`
- `generate_movies_stream` — reads `cache.movies` → replace with `repo.get_all()`
- `get_matching_movies` — reads `state.movies` → replace with `repo.get_all()`
- `update_movie_location` — reads/writes `state.movies` cache → replace with DB read after update

## Steps

1. Move all `CacheState` routes in `main.rs` to `chat_stream_router` (already uses `DbState`)
   or give `stateless_router` a `DbState` too and merge
2. Update each handler signature: `State(state): State<CacheState>` → `State(state): State<DbState>`
3. Replace `state.movies.read().await` with `state.pool.get_all().await`
4. Delete `CacheState` struct and the `movies: Arc<RwLock<...>>` startup load in `main.rs`
5. Remove `repo` field added to `CacheState` (no longer needed)
6. Simplify `init_router` — single router with `DbState`
