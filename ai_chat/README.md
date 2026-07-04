# AI Chat Module

Ollama integration for AI-powered features in the DVD Categorizer application.

## Overview

This crate provides the interface between the DVD Categorizer and Ollama LLM services, enabling:

- **Vector Embeddings**: Generate semantic embeddings for movies and search queries
- **Query Enhancement**: Expand short queries for better semantic search results
- **Structured Query Parsing**: Convert natural language to structured search criteria
- **Chat Conversations**: Answer questions about the movie collection

## Features

### Embeddings (`embedding.rs`)

Generates vector embeddings using the `nomic-embed-text` model for semantic similarity search.

```rust
use ai_chat::get_embedding;

let embedding = get_embedding("science fiction movies").await?;
```

### Query Enhancement (`query_enhancement.rs`)

Automatically expands short queries (< 3 words) into more descriptive phrases for better vector search results.

```rust
use ai_chat::enhance_query_for_embedding;

let enhanced = enhance_query_for_embedding("robots", None).await;
// Returns: "movies about robots and robotic characters"
```

### Structured Query Parser (`structured_query_parser.rs`)

Parses natural language queries into structured criteria for type-safe database queries.

```rust
use ai_chat::parse_query_to_structured;

let structured = parse_query_to_structured("brad pitt action movies", None).await?;
// Returns: StructuredQuery { actors: ["brad pitt"], genres: ["action"], ... }
```

### Live UI (`live_ui.rs`)

Provides the `AiChatProvider` trait for dependency injection and testability, with production implementation using Ollama.

## Configuration

Environment variables:

- `OLLAMA_HOST`: Ollama service hostname (default: `ollama`)
- `OLLAMA_PORT`: Ollama service port (default: `11434`)
- `OLLAMA_NUM_CTX`: Context window size (default: `8000`)

## Models Used

- **qwen2.5:7b**: Default model for chat, query enhancement, and structured parsing
- **nomic-embed-text**: Embedding generation (768-dimensional vectors)

## Testing

The crate includes comprehensive unit tests with mock implementations:

```bash
cargo test -p ai_chat
```

Mock providers are available for testing without a live Ollama instance.

## Dependencies

- `ollama-rs`: Ollama client library
- `models`: Shared domain types
- `serde`/`serde_json`: Serialization
- `tokio`: Async runtime
