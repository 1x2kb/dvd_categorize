# CSV Import/Export Flow

How movie data is imported via the web UI and exported from the API.

## Import Flow (Web UI → API → Database)

```mermaid
sequenceDiagram
    participant User as Web UI (Insert Page)
    participant API as dvd_catalog_api
    participant CSV as csv_utils
    participant DB as PostgreSQL
    participant Cache as Movie Cache

    User->>API: POST /csv/preview
    API->>CSV: parse_csv(bytes)
    CSV-->>API: Vec FullMovie
    API-->>User: Preview parsed movies

    Note over User: User reviews movie cards and confirms import

    User->>API: POST /csv/parse
    API->>CSV: parse_csv(bytes)
    CSV-->>API: Vec FullMovie

    API->>DB: insert_full_movies(movies)
    DB-->>API: Success

    API->>DB: get_movies()
    DB-->>API: All movies (refreshed)
    API->>Cache: Write lock → replace cache
    Cache-->>API: Cache updated

    API-->>User: 200 OK
```

## Import Decision Flow

```mermaid
flowchart TD
    Paste["User pastes CSV text"] --> Preview{"Preview or Import?"}

    Preview -->|Preview| PreviewReq["POST /csv/preview"]
    PreviewReq --> Parse1["csv_utils::parse_csv"]
    Parse1 --> Grid["Display movie cards<br/>(no database write)"]

    Preview -->|Import| ImportReq["POST /csv/parse"]
    ImportReq --> Parse2["csv_utils::parse_csv"]
    Parse2 --> Valid{"Parse OK?"}
    Valid -->|No| Error["400 Bad Request<br/>Parse error message"]
    Valid -->|Yes| Insert["database::insert_full_movies"]
    Insert --> InsertOK{"Insert OK?"}
    InsertOK -->|No| DBError["500 Internal Server Error"]
    InsertOK -->|Yes| Refresh["Refresh movie cache"]
    Refresh --> Success["200 OK"]
```

## Export Flow

```mermaid
sequenceDiagram
    participant User as Web UI / Client
    participant API as dvd_catalog_api
    participant Cache as Movie Cache
    participant CSV as csv_utils

    User->>API: GET /csv/export
    API->>Cache: Read movies
    Cache-->>API: Vec FullMovie
    API->>CSV: movies_to_csv(movies)
    CSV-->>API: CSV string
    API-->>User: text/csv attachment (movies.csv)
```

## CSV Format

Header row is **required**. Column order does not matter.

```
Title,Year,Description,Actors,Genres,Director,AddedOn,Location
```

| Column | Required | Notes |
|--------|----------|-------|
| Title | Yes | Movie name (max 100 chars) |
| Year | Yes | Release year (integer, use 0 for unknown) |
| Description | No | Free text, used for embedding generation |
| Actors | No | Comma-separated within the field |
| Genres | No | Comma-separated within the field |
| Director | No | Single director name |
| AddedOn | No | Timestamp |
| Location | Yes | Physical location (max 100 chars) |
