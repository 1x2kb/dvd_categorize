# Search Flow

How movie search requests are processed through the application. The API dispatches to one of four search strategies based on the `search_mode` field in the request.

## Dispatch Flow

```mermaid
flowchart TD
    Client["Web UI / API Client"] -->|POST /ai/dvd-match| API["dvd_catalog_api"]
    API --> Dispatch{search_mode?}

    Dispatch -->|Text| Text["Text Search"] --> Results["Build ScoredMovie response"]
    Dispatch -->|Vector| Vector["Vector Search"] --> Results
    Dispatch -->|Structured| Structured["Structured Search"] --> Results

    Dispatch -->|Both| Both["Hybrid Search"]
    Both --> HText["Text Search"]
    Both --> HVector["Vector Search"]
    HText --> RRF["RRF Fusion"]
    HVector --> RRF
    RRF --> Results

    Results --> Client
```

## Text Search

Pure keyword matching against all movies in the database. No AI involvement.

```mermaid
flowchart TD
    Q["User Query"] --> EE["Extract Entities"]
    EE --> |"Scan all movies from DB"| Entities["Titles, Actors, Directors, Genres"]
    Entities --> KW["Keyword Scoring"]
    KW --> |"Title: 100pts exact, 50pts partial<br/>Actor: 20pts<br/>Director: 25pts<br/>Genre: 15pts"| Ranked["Ranked Results"]
```

## Vector Search

Semantic search using AI embeddings. Short queries are enhanced first. The actual vector store depends on the active backend:

- **PostgreSQL** — `pgvector` column with cosine distance.
- **MongoDB** — Qdrant vector database with cosine similarity.

```mermaid
sequenceDiagram
    participant Client
    participant API as dvd_catalog_api
    participant Ollama
    participant VectorStore as Vector Store (pgvector or Qdrant)

    Client->>API: Search query
    
    alt Enhancement enabled & query is short
        API->>Ollama: Enhance query (qwen2.5:7b)
        Ollama-->>API: Expanded descriptive query
    end

    API->>Ollama: Generate embedding (nomic-embed-text)
    Ollama-->>API: Vector embedding

    API->>VectorStore: Cosine similarity search
    VectorStore-->>API: Nearest movies by embedding distance

    API-->>Client: Scored results
```

## Hybrid Search (Both)

Runs Text and Vector in parallel, then fuses with Reciprocal Rank Fusion.

```mermaid
flowchart TD
    Q["User Query"] --> Fork["Parallel Execution"]

    Fork --> Branch1["Entity Extraction"]
    Fork --> Branch2["Query Enhancement"]

    Branch1 --> KW["Keyword Search<br/>(DB via get_all)"]
    Branch2 --> Embed["Generate Embedding<br/>(Ollama)"]
    Embed --> VS["Vector Search<br/>(pgvector or Qdrant)"]

    KW --> RRF["Reciprocal Rank Fusion"]
    VS --> RRF

    RRF --> |"score = 1/(k + rank)"| Boost{"Exact title match?"}
    Boost -->|Yes| BoostScore["score *= 10x boost"]
    Boost -->|No| Final["Final Ranked Results"]
    BoostScore --> Final
```

### RRF Formula

```
RRF_score(movie) = Σ 1 / (k + rank_i)
```

- **k = 60** (standard constant)
- Exact title matches (keyword score >= 100) receive a **10x boost multiplier**
- Results from both lists are merged by movie ID, scores summed

## Structured Search

AI parses natural language into structured JSON criteria, then Diesel builds type-safe queries.

```mermaid
sequenceDiagram
    participant Client
    participant API as dvd_catalog_api
    participant Ollama
    participant PG as PostgreSQL

    Client->>API: "brad pitt action adventure movies"

    API->>Ollama: Parse to structured JSON (qwen2.5:7b)
    Ollama-->>API: {"actors": ["brad pitt"], "genres": ["action", "adventure"]}

    API->>PG: Dynamic Diesel query with JOINs
    Note right of PG: movie JOIN movie_actor JOIN actor WHERE actor.name ILIKE brad pitt AND movie_genre.genre ILIKE action, adventure
    PG-->>API: Matching movies

    API-->>Client: Scored results
```

### Structured Query Fields

| Field | Maps To | Join Required |
|-------|---------|---------------|
| `actors` | `actor.name` | `movie_actor` + `actor` |
| `directors` | `director.name` | `director` |
| `genres` | `movie_genre.genre` | `movie_genre` |
| `title_keywords` | `movie.name` | None |
| `description_keywords` | `movie.description` | None |

All filters use **ILIKE** (case-insensitive) and combine with **AND** logic.

See [STRUCTURED_SEARCH.md](../Explantion/STRUCTURED_SEARCH.md) for additional details.
