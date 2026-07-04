# AI Chat Flow

How conversational AI chat requests are processed. The chat assistant is designed to answer questions about your movie collection. It supports two modes: **RAG** (streaming, retrieval-augmented generation) and **Tool** (non-streaming, tool-calling). Chat sessions are persisted in the database.

> Chat is still very experimental.

## Chat Modes

### RAG Mode — `/ai/chat/stream`

The API first finds relevant movies from the collection, then streams the LLM response that includes those movies as context.

```mermaid
sequenceDiagram
    participant User as Web UI (Chat Page)
    participant API as dvd_catalog_api
    participant Ollama as Ollama (qwen2.5:7b)
    participant DB as Database

    User->>API: POST /ai/chat/stream {session_id, messages, model}

    Note over API: Load or create chat session

    API->>Ollama: Find matching movies (hybrid search)
    Ollama-->>API: Relevant movie IDs

    API->>DB: Fetch movie details by ID
    DB-->>API: Movie records

    Note over API: Build RAG system prompt with movie context

    API->>Ollama: send_chat_messages [rag_prompt, history, context]
    Ollama-->>API: Stream response chunks

    API-->>User: SSE stream of response text
```

### Tool Mode — `/ai/chat`

The LLM is given a set of tools it can call to query the API (e.g., search by actor, genre, director). It returns a single non-streaming response.

```mermaid
sequenceDiagram
    participant User as Web UI (Chat Page)
    participant API as dvd_catalog_api
    participant Ollama as Ollama (qwen2.5:7b)

    User->>API: POST /ai/chat {session_id, messages, model}

    Note over API: Load or create chat session
    Note over API: Build tool-enabled system prompt

    API->>Ollama: send_chat_messages with tool definitions
    Ollama-->>API: Tool calls or final response

    alt Tool calls requested
        API->>API: Execute tool (search, filter, etc.)
        API->>Ollama: Send tool results back
        Ollama-->>API: Final response
    end

    API-->>User: {message: "response text"}
```

## Session Persistence

Chat sessions are stored in the database. Each session has a UUID, creation time, and last-updated time. Messages are saved as the conversation progresses.

```mermaid
sequenceDiagram
    participant User as Web UI
    participant API as dvd_catalog_api
    participant DB as Database

    User->>API: GET /ai/chat/sessions
    API->>DB: list_chat_sessions()
    DB-->>API: Sessions with previews
    API-->>User: Display session list

    User->>API: GET /ai/chat/sessions/{session_id}
    API->>DB: get_chat_history(session_id)
    DB-->>API: Messages
    API-->>User: Load full conversation
```

## System Prompts

The prompts are defined in the `prompts` crate. Users can edit them in the web UI and reset to defaults.

- **RAG prompt** — instructs the assistant to answer based on the retrieved movie context.
- **Tool prompt** — instructs the assistant to use the available tools to query the collection.

## Message Flow

```mermaid
flowchart LR
    subgraph "Message Array Sent to Ollama"
        S["System<br/>(RAG or Tool prompt)"]
        U1["User<br/>(message 1)"]
        A1["Assistant<br/>(response 1)"]
        U2["User<br/>(message 2)"]
        A2["Assistant<br/>(response 2)"]
        UN["User<br/>(latest message)"]
    end

    S --> U1 --> A1 --> U2 --> A2 --> UN
```

## Error Handling

| Condition | Response |
|-----------|----------|
| Ollama unreachable | `500 Internal Server Error` — Error message from Ollama |
| Model not installed | `500 Internal Server Error` — Ollama model error |
| Invalid session ID | `400 Bad Request` |

## Notes

- Two modes are available: **RAG** (streaming with retrieval) and **Tool** (non-streaming with tool-calling).
- Chat sessions are persisted server-side; users can resume past conversations from the history panel.
- The assistant answers questions about the movie collection, not general topics.
- Default chat model: `qwen2.5:7b` (configurable per request).
