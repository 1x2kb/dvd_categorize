# Application Startup Flow

How the application boots up across Docker Compose services and initializes internal state.

## Docker Compose Service Startup

```mermaid
sequenceDiagram
    participant DC as Docker Compose
    participant PG as PostgreSQL
    participant Ollama as Ollama
    participant Redis as Redis
    participant Diesel as diesel_migrate
    participant API as dvd_catalog_api
    participant Web as dvd_categorizer_web

    DC->>PG: Start postgres container
    DC->>Ollama: Start ollama container
    DC->>Redis: Start redis container

    Note over PG: Waiting for healthcheck (pg_isready)

    PG-->>DC: Healthy

    DC->>Diesel: Run migrations
    Diesel->>PG: diesel migration run
    PG-->>Diesel: Migrations applied
    Diesel-->>DC: Exit 0

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

```mermaid
flowchart LR
    subgraph Stateful["Stateful Routes (CacheState)"]
        DVD_LIST["GET /dvd"]
        SEARCH["POST /ai/dvd-match"]
        CHAT["POST /ai/chat"]
        LOC["POST /movie/location"]
        CSV_PARSE["POST /csv/parse"]
        CSV_EXPORT["GET /csv/export"]
    end

    subgraph Stateless["Stateless Routes"]
        DVD_ID["GET /dvd/{id}"]
        PREVIEW["POST /csv/preview"]
        PULL["POST /ai/pull-model"]
        MODELS["GET /ai/models"]
        RECENT["GET /ai/recent"]
        HEALTH["GET /"]
    end
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
```

## Environment Variables

| Variable | Default | Used By |
|----------|---------|---------|
| `server_host` | `0.0.0.0` | API bind address |
| `server_port` | `3000` | API bind port |
| `DATABASE_URL` | — | Diesel database connection |
| `OLLAMA_HOST` | `ollama` | AI service hostname |
| `OLLAMA_PORT` | `11434` | AI service port |
| `RUST_LOG` | `info` | Log level filter |
