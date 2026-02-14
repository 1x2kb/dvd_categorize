# AI Chat Flow

How conversational AI chat requests are processed. The chat endpoint is a pure general-purpose assistant that can discuss any topic. No DVD collection context is provided.

## Chat Sequence

```mermaid
sequenceDiagram
    participant User as Web UI (Chat Page)
    participant API as dvd_catalog_api
    participant Ollama as Ollama (phi3.5)

    User->>API: POST /ai/chat {messages, model}

    Note over API: Build simple system prompt (general-purpose assistant)

    Note over API: Convert chat history to Ollama format (User/Assistant messages)

    API->>Ollama: send_chat_messages [system_prompt, ...history]
    Ollama-->>API: Generated response

    API-->>User: {message: "response text"}
```

## System Prompt

The API uses a simple general-purpose system prompt:

```
"You are a friendly and knowledgeable assistant. 
You can help with any topic the user asks about. 
Be conversational, helpful, and concise."
```

This is prepended to the message history before sending to Ollama.

## Message Flow

```mermaid
flowchart LR
    subgraph "Message Array Sent to Ollama"
        S["System<br/>(general assistant)"]
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
| No special conditions | Chat works independently of the movie cache |
| Ollama unreachable | `500 Internal Server Error` — Error message from Ollama |
| Model not installed | `500 Internal Server Error` — Ollama model error |

## Notes

- Chat history is maintained **client-side** in the Dioxus frontend (via `use_signal`)
- The full message history is sent with each request (no server-side session)
- The system prompt is a simple general-purpose assistant directive with no collection context
- The AI can discuss any topic without restrictions
- Default model: `phi3.5` (configurable per request)
