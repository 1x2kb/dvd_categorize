# Application Startup Flow

How the application boots up across Docker Compose services and initializes internal state.

## Docker Compose Service Startup

The application uses Docker Compose profiles to select a backend. You must pass either `--profile postgres` or `--profile mongodb` when starting services. Shared services (`ollama`, `qdrant`, `grafana`, `prometheus`, etc.) have no profile and start with either selection.

### PostgreSQL Profile

```mermaid
sequenceDiagram
    participant DC as Docker Compose
    participant PG as PostgreSQL
    participant Ollama as Ollama
    participant Diesel as diesel_migrate
    participant API as dvd_catalog_api (postgres)
    participant Web as dvd_categorizer_web

    DC->>PG: Start postgres container
    DC->>Ollama: Start ollama container

    Note over PG: Waiting for healthcheck (pg_isready)

    PG-->>DC: Healthy

    DC->>Diesel: Run migrations
    Diesel->>PG: diesel migration run
    PG-->>Diesel: Migrations applied
    Diesel-->>DC: Exit 0

    DC->>API: Start API server
    DC->>Web: Start web server
```

### MongoDB Profile

```mermaid
sequenceDiagram
    participant DC as Docker Compose
    participant Mongo as MongoDB
    participant Qdrant as Qdrant
    participant Ollama as Ollama
    participant API as dvd_catalog_api (mongodb)
    participant Web as dvd_categorizer_web

    DC->>Mongo: Start mongodb container
    DC->>Qdrant: Start qdrant container
    DC->>Ollama: Start ollama container

    Note over Mongo: Waiting for healthcheck (mongosh ping)

    Mongo-->>DC: Healthy

    DC->>API: Start API server
    DC->>Web: Start web server
```

## API Server Initialization

```mermaid
flowchart TD
    Start["main()"] --> Logger["Initialize env_logger<br/>(timestamp + module info)"]
    Logger --> Env{"Debug build?"}
    Env -->|Yes| DotEnv["Load .env file"]
    Env -->|No| Skip["Use container env vars"]
    DotEnv --> LoadMovies
    Skip --> LoadMovies

    LoadMovies["database::get_movies()"] --> LoadResult{"Success?"}
    LoadResult -->|Yes| Cache["Wrap in Arc&lt;RwLock&lt;Vec&lt;FullMovie&gt;&gt;&gt;"]
    LoadResult -->|No| Empty["Log error, use empty Vec"]
    Empty --> Cache

    Cache --> Router["Build Axum Router"]
    Router --> Stateful["Stateful routes<br/>(need CacheState)"]
    Router --> Stateless["Stateless routes<br/>(no shared state)"]

    Stateful --> Merge["Merge routers"]
    Stateless --> Merge
    Merge --> CORS["Add CORS layer<br/>(Allow all origins)"]
    CORS --> Bind["Bind to SERVER_HOST:SERVER_PORT"]
    Bind --> Serve["axum::serve()"]
```

## Router Structure

All routes receive the database state (`DbState`) and are grouped by feature below.

```mermaid
flowchart LR
    subgraph Movies["Movies"]
        DVD_LIST["GET /dvd"]
        DVD_ID["GET /dvd/{id}"]
        DVD_RECENT["GET /dvd/recent-releases"]
        DVD_RANDOM["GET /dvd/random"]
        DVD_UNKNOWN["GET /dvd/unknown-location"]
        RECENT["GET /ai/recent"]
        SAVE["POST /saveMovie"]
    end

    subgraph Search["Search"]
        SEARCH["POST /ai/dvd-match"]
        LOCATION["GET /location/{location}"]
        LOCATIONS["GET /uniqueLocations()"]
    end

    subgraph Chat["Chat"]
        CHAT["POST /ai/chat"]
        CHAT_STREAM["POST /ai/chat/stream"]
        SESSIONS["GET /ai/chat/sessions"]
        SESSION["GET /ai/chat/sessions/{id}"]
    end

    subgraph CSV["CSV"]
        CSV_PREVIEW["POST /csv/preview"]
        CSV_PARSE["POST /csv/parse"]
        CSV_EXPORT["GET /csv/export"]
    end

    subgraph AI["AI / Models"]
        GENERATE["POST /ai/generate-movies-stream"]
        PULL["POST /ai/pull-model"]
        MODELS["GET /ai/models"]
    end

    subgraph Stats["Stats"]
        STATS_OVERVIEW["GET /stats/overview"]
        STATS_YEAR["GET /stats/movies-by-year"]
        STATS_GENRE["GET /stats/genres"]
        STATS_ACTORS["GET /stats/top-actors"]
    end

    subgraph Location["Location"]
        LOC_UPDATE["POST /movie/location"]
    end

    HEALTH["GET /"] --> Health["Health Check"]
```

## Cache Lifecycle

```mermaid
flowchart TD
    Startup["Server Start"] -->|"get_movies()"| Init["Cache initialized<br/>Arc&lt;RwLock&lt;Vec&lt;FullMovie&gt;&gt;&gt;"]

    Init --> Read["Read Lock<br/>(concurrent reads)"]
    Init --> Write["Write Lock<br/>(exclusive mutation)"]

    Read --> R1["GET /dvd — return all movies"]
    Read --> R2["POST /ai/dvd-match — search against cache"]
    Read --> R3["POST /ai/chat — build context from cache"]
    Read --> R4["GET /csv/export — serialize cache to CSV"]

    Write --> W1["POST /csv/parse — refresh after import"]
    Write --> W2["POST /movie/location — update after location change"]
    Write --> W3["POST /saveMovie — add generated movie"]
```

## Environment Variables

| Variable | Default | Used By | Backend |
|----------|---------|---------|---------|
| `server_host` | `0.0.0.0` | API bind address | Both |
| `server_port` | `3000` | API bind port | Both |
| `DATABASE_URL` | — | PostgreSQL connection | PostgreSQL |
| `MONGODB_URI` | — | MongoDB connection | MongoDB |
| `QDRANT_URL` | — | Qdrant gRPC URL | MongoDB |
| `QDRANT_COLLECTION` | `movies` | Qdrant collection name | MongoDB |
| `OLLAMA_HOST` | `ollama` | AI service hostname | Both |
| `OLLAMA_PORT` | `11434` | AI service port | Both |
| `RUST_LOG` | `info` | Log level filter | Both |
