# Hybrid Search Refactor

## Background

The `Both` search mode was originally designed to combine text (ILIKE) and vector
(cosine similarity) search results using Reciprocal Rank Fusion (RRF). When an
in-memory movie cache existed, the text side used `keyword_search` against cached
data. The cache was removed (commit `9e01f0e`) but the text side was never converted
to use the database — it still calls `get_all()` and scores in-memory, making every
`Both` mode search load the entire catalog unnecessarily.

## Goals

1. Fix the broken `Both` mode by replacing `get_all` + in-memory `keyword_search`
   with `search_movies_by_text` (already exists, does DB-level ILIKE scoring).
2. Add a new `BothCombined` search mode that performs ILIKE filtering AND cosine
   ranking in a single SQL query, to compare results against the RRF approach.

## Plan

### Step 1 — Fix `Both` mode (RRF, two queries)

**`dvd_catalog_api/src/movie_search.rs`**
- Remove `get_all()` call from `get_matching_movies` handler
- Remove `movies: Arc<Vec<FullMovie>>` parameter from `hybrid_search` and
  `hybrid_both_search`
- Replace the `keyword_search` + `extract_entities` + `get_all` block in
  `hybrid_both_search` with a call to `repo.search_movies_by_text(query, limit * 2)`
- Feed `(id, score)` pairs from `search_movies_by_text` into the existing
  `reciprocal_rank_fusion` function alongside vector results
- Delete now-unused functions: `keyword_search`, `extract_entities`,
  `score_movie_by_keywords`, `SearchCriteria`, `extract_entities_from_movies`

**`dvd_catalog_api/src/lib.rs`**
- Remove `get_all()` call and `Arc<Vec<FullMovie>>` from `get_matching_movies`
- Update `combined_search` signature accordingly

### Step 2 — Add `BothCombined` mode (single query)

**`database/src/traits.rs`**
- Add `SearchMoviesByTextAndEmbedding` trait:
  ```rust
  async fn search_text_and_embedding(
      &self,
      query: &str,
      embedding: Vec<f32>,
      limit: i64,
  ) -> Result<Vec<FullMovie>, DatabaseError>;
  ```

**`database/src/postgres.rs`**
- Implement `search_text_and_embedding`:
  - ILIKE filter across `movie.name`, `actor.name`, `director.name`, `movie_genre.genre`
    (same conditions as `search_movies_by_text`)
  - `ORDER BY movie.embedding <=> $vector`
  - Returns `FullMovie` via bulk actor/genre load

**`models/src/lib.rs`**
- Add `BothCombined` variant to `SearchMode` enum

**`dvd_catalog_api/src/movie_search.rs`**
- Add `combined_text_vector_search` function that:
  1. Generates embedding for query
  2. Calls `repo.search_text_and_embedding(query, embedding, limit)`
  3. Returns `(id, score)` pairs using rank position as score
- Wire into `hybrid_search` match arm

**`dvd_catalog_web/src/components/search_mode_selector.rs`**
- Add "Both+" button for `SearchMode::BothCombined`

**`dvd_catalog_web/src/components/movie_grid.rs`**
- Add score label for `SearchMode::BothCombined` (e.g. "Combined Score: {:.4}")

### Step 3 — Compare

Run the same queries against `Both` (RRF) and `BothCombined` (single query) and
compare result ordering. Key test cases:
- Exact title match: "The Dark Knight" — RRF should rank it first due to title boost
- Semantic query: "medieval knight honor" — combined may rank thematically closer
  films higher since cosine ordering is applied within the text-filtered set
- Actor name: "Tom Hanks" — both should return same set, ordering may differ

## Notes

- `BothCombined` trades ranking flexibility for a single DB round trip
- RRF allows each dimension (text relevance vs semantic similarity) to be ranked
  on its own terms before fusion; the combined query loses text match quality as
  a ranking signal (only as a filter)
- For title/actor/genre lookups, RRF is likely better; for "vibe" queries,
  combined may be better
- The `extract_entities` machinery (actors/genres from in-memory movies) can be
  fully deleted — it was only needed for the broken in-memory keyword search path
