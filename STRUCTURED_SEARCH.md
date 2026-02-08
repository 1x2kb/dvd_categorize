# Structured Query Search Mode

## Overview

The Structured Query Search mode uses AI to parse natural language queries into structured criteria, then dynamically builds Diesel queries to search the database. Unlike other search modes, it does **NOT generate SQL** but instead extracts structured data that Diesel uses to build type-safe queries.

## How It Works

### 1. AI Query Parser
Location: `ai_chat/src/structured_query_parser.rs`

The AI takes a natural language query and extracts:
- **actors**: List of actor names mentioned
- **directors**: List of director names mentioned  
- **genres**: List of genres mentioned
- **title_keywords**: Keywords that might be part of a movie title
- **description_keywords**: Keywords describing the movie plot or theme

**Example:**
```
User query: "Show me brad pitt action adventure movies"

Parsed output:
{
  "actors": ["brad pitt"],
  "directors": [],
  "genres": ["action", "adventure"],
  "title_keywords": [],
  "description_keywords": []
}
```

### 2. Dynamic Diesel Query Builder
Location: `database/src/structured_search.rs`

Takes the structured query and dynamically builds Diesel queries:

- **Directors**: Filters by director name (case-insensitive ILIKE)
- **Title Keywords**: Filters movie titles (AND logic - all keywords must match)
- **Description Keywords**: Filters movie descriptions (AND logic)
- **Actors**: Joins through `movie_actor` table, filters by actor names
- **Genres**: Joins through `movie_genre` table, filters by genre names

The function `search_movies_structured()` returns fully hydrated `FullMovie` objects with all relationships loaded.

### 3. SearchMode Enum
Location: `models/src/lib.rs`

New variant added:
```rust
pub enum SearchMode {
    Text,      // Keyword-based search
    Vector,    // Semantic embedding search
    Both,      // Hybrid RRF fusion
    Structured, // AI-parsed + Diesel queries (NEW)
}
```

## API Usage

### Endpoint
```
POST /search
```

### Request Body
```json
{
  "query": "Show me brad pitt action adventure movies",
  "search_mode": "structured",
  "model": "phi3.5"  // Optional, defaults to phi3.5
}
```

### Response
```json
{
  "results": [
    {
      "movie": {
        "id": 123,
        "name": "Troy",
        "actors": [{"id": 1, "name": "Brad Pitt"}],
        "genres": ["Action", "Adventure"],
        ...
      },
      "vector_score": 1.0
    }
  ],
  "original_query": "Show me brad pitt action adventure movies",
  "enhanced_query": "StructuredQuery { actors: [\"brad pitt\"], directors: [], genres: [\"action\", \"adventure\"], ... }"
}
```

## Benefits

1. **Type-Safe**: Uses Diesel's query builder - no SQL injection risk
2. **Precise Filtering**: Directly queries database tables with proper joins
3. **Efficient**: Database does the filtering, not application code
4. **Transparent**: Structured output shows exactly what was parsed
5. **No SQL Generation**: AI never writes SQL, only extracts structured data

## Example Queries

| User Query | Parsed Structure |
|------------|------------------|
| "brad pitt action movies" | actors: ["brad pitt"], genres: ["action"] |
| "christopher nolan space films" | directors: ["christopher nolan"], description_keywords: ["space"] |
| "funny robot movies" | genres: ["comedy"], description_keywords: ["robot", "funny"] |
| "tom hanks comedy" | actors: ["tom hanks"], genres: ["comedy"] |
| "the matrix" | title_keywords: ["matrix"] |

## Implementation Files

- `models/src/lib.rs` - StructuredQuery type, SearchMode::Structured
- `ai_chat/src/structured_query_parser.rs` - AI query parser
- `database/src/structured_search.rs` - Diesel query builder
- `dvd_catalog_api/src/movie_search.rs` - API integration
- `dvd_categorizer_web/src/components/movie_grid.rs` - UI display

## Testing

The AI parser includes unit tests for JSON extraction. To test the full flow:

```bash
curl -X POST http://localhost:3000/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": "Show me brad pitt action movies",
    "search_mode": "structured"
  }'
```

## Comparison with Other Search Modes

| Mode | How It Works | Best For |
|------|-------------|----------|
| Text | Keyword matching + entity extraction | Exact title/actor matches |
| Vector | Semantic embeddings + cosine similarity | Conceptual/thematic searches |
| Both | RRF fusion of Text + Vector | General-purpose search |
| **Structured** | **AI parsing → Diesel queries** | **Specific criteria (actors, genres, directors)** |

## Notes

- The AI model defaults to `phi3.5` for parsing (fast and accurate)
- All database fields use case-insensitive ILIKE for flexible matching
- Multiple criteria use AND logic (all must match)
- Returns movies in relevance order based on database results
