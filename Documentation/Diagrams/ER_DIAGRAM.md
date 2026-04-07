# Entity Relationship Diagram

Database schema for the DVD Categorize application. Generated from `models/src/schema.rs`.

```mermaid
erDiagram
    movie {
        int id PK
        varchar(100) name
        int director_id FK
        text description
        vector embedding
        timestamp added_on
        varchar(100) location
    }

    director {
        int id PK
        varchar(100) name
    }

    actor {
        int id PK
        varchar(255) name
    }

    movie_actor {
        int id PK
        int movie_id FK
        int actor_id FK
    }

    movie_genre {
        int id PK
        int movie_id FK
        varchar(30) genre
    }

    director ||--o{ movie : "directs"
    movie ||--o{ movie_actor : "has"
    actor ||--o{ movie_actor : "appears in"
    movie ||--o{ movie_genre : "categorized as"
```

## Notes

- **movie.embedding** — pgvector `Vector` type storing AI-generated embeddings from `nomic-embed-text` for semantic similarity search
- **movie.director_id** — Nullable; not all movies have a director assigned
- **movie.description** — Nullable; used as the source text for embedding generation
- **movie_actor** — Many-to-many join table between movies and actors
- **movie_genre** — One-to-many; genres are stored as strings per movie (not a separate genre table)
- All joins defined via Diesel's `joinable!` macro in `schema.rs`
