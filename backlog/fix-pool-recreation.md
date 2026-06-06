# Fix Handlers That Recreate Pool Per Request

These handlers call `PostgresMovieRepository::from_env()` on every request,
creating a fresh pool each time instead of reusing the shared one.

## Offending handlers (all in `dvd_catalog_api/src/lib.rs`)

- `get_dvd` (line ~107)
- `insert_dvd` (line ~126)
- `update_movie_location` cache refresh (line ~1438)
- `get_recent_movies` (line ~1555)
- `get_recent_releases` (line ~1601)
- `get_random_movies` (line ~1651)
- `get_unknown_location_movies` (line ~1692)
- `unique_locations` (line ~1731)
- `get_movies_by_location` (line ~1759)
- `stats_overview` (line ~1791)
- `stats_movies_by_year` (line ~1873)
- `stats_genres` (line ~1948)
- `stats_top_actors` (line ~2235)

## Fix

All these are on `stateless_router` which currently has no state.
Give `stateless_router` a `DbState` and update each handler to extract
`State(state): State<DbState>` and use `state.pool` directly.

This is a prerequisite / natural companion to the `remove-cache-state` task.
