# AI Chat Flow

How conversational AI chat requests are processed. The chat endpoint is a general-purpose assistant that also has access to the user's DVD collection as context. It builds movie context from the cache, constructs a message history with a system prompt, and sends the conversation through Ollama.

## Chat Sequence

```mermaid
sequenceDiagram
    participant User as Web UI (Chat Page)
    participant API as dvd_catalog_api
    participant Cache as Movie Cache
    participant Ollama as Ollama (phi3.5)

    User->>API: POST /ai/chat {messages, model}

    API->>Cache: Read movies
    Cache-->>API: Vec FullMovie (up to 100)

    Note over API: Build system prompt with movie collection context (names, genres, locations)

    Note over API: Convert chat history to Ollama format (User/Assistant messages)

    API->>Ollama: send_chat_messages [system_prompt, ...history]
    Ollama-->>API: Generated response

    API-->>User: {message: "response text"}
```

## System Prompt Construction

The API dynamically builds the system prompt by injecting up to 100 movies from the cache:

```mermaid
flowchart TD
    Cache["Movie Cache"] --> Take["Take first 100 movies"]
    Take --> Format["Format each movie:<br/>Name - Genres - Location"]
    Format --> Join["Join with newlines"]
    Join --> System["System Prompt:<br/>'You are a helpful assistant that<br/>answers questions about a user's<br/>DVD movie collection...'<br/>+ movie list"]
    System --> Messages["Prepend to message history"]
```

## Message Flow

```mermaid
flowchart LR
    subgraph "Message Array Sent to Ollama"
        S["System<br/>(collection context)"]
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
| Empty movie cache | Chat still works — system prompt notes the collection is empty |
| Ollama unreachable | `500 Internal Server Error` — Error message from Ollama |
| Model not installed | `500 Internal Server Error` — Ollama model error |

## Notes

- Chat history is maintained **client-side** in the Dioxus frontend (via `use_signal`)
- The full message history is sent with each request (no server-side session)
- The system prompt includes movie names, genres, and physical locations so the AI can recommend where to find a DVD
- Default model: `phi3.5` (configurable per request)
