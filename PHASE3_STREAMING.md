# Phase 3: SSE Streaming for Real-time AI Responses

## Status: Ready after Phase 2 complete

## Goal
Stream AI responses word-by-word to frontend using Server-Sent Events (SSE).

## Prerequisites
- Phase 2 complete (connection pooling + tool calling working)
- `stream` feature enabled in ollama-rs
- `sse` feature enabled in axum

## Dependencies to Add

### Workspace Cargo.toml
```toml
[workspace.dependencies]
ollama-rs = { version = "0.3.4", features = ["tokio", "macros", "stream"] }
async-stream = "0.3"
```

### dvd_catalog_api/Cargo.toml
```toml
[dependencies]
axum = { version = "0.7", features = ["sse"] }
async-stream.workspace = true
futures-util = "0.3"
tokio-stream = "0.1"
```

## Backend Changes

### 1. Create Streaming Endpoint
File: `dvd_catalog_api/src/lib.rs`

```rust
use axum::response::sse::{Event, Sse};
use futures_util::stream::Stream;
use std::convert::Infallible;
use tokio_stream::StreamExt;

pub async fn chat_stream(
    State(state): State<AppState>,
    Json(request): Json<ChatRequest>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let ollama = Ollama::default();
    
    // Build chat request
    let messages = request.messages.iter()
        .map(|m| match m.role.as_str() {
            "user" => ChatMessage::user(m.content.clone()),
            "assistant" => ChatMessage::assistant(m.content.clone()),
            _ => ChatMessage::system(m.content.clone()),
        })
        .collect();
    
    let chat_request = ChatMessageRequest::new(
        request.model.unwrap_or("llama3.2:latest".to_string()),
        messages,
    );
    
    // Get Ollama stream
    let ollama_stream = match ollama.send_chat_messages_stream(chat_request).await {
        Ok(stream) => stream,
        Err(e) => {
            // Return error as single SSE event
            let error_stream = async_stream::stream! {
                yield Ok(Event::default()
                    .event("error")
                    .data(format!("Failed to start stream: {}", e)));
            };
            return Sse::new(error_stream);
        }
    };
    
    // Convert to SSE stream
    let sse_stream = async_stream::stream! {
        tokio::pin!(ollama_stream);
        
        while let Some(chunk) = ollama_stream.next().await {
            match chunk {
                Ok(response) => {
                    // Send content chunk
                    yield Ok(Event::default()
                        .event("message")
                        .data(response.message.content));
                    
                    // Send done event on final chunk
                    if response.done {
                        yield Ok(Event::default()
                            .event("done")
                            .data(""));
                    }
                }
                Err(e) => {
                    yield Ok(Event::default()
                        .event("error")
                        .data(format!("Stream error: {}", e)));
                    break;
                }
            }
        }
    };
    
    Sse::new(sse_stream)
        .keep_alive(axum::response::sse::KeepAlive::default())
}
```

### 2. Add Route
```rust
pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/api/chat", post(bot_message))
        .route("/api/chat/stream", post(chat_stream))  // New streaming endpoint
        // ... other routes
        .with_state(state)
}
```

## Frontend Changes (Dioxus)

### 1. Create SSE Hook
File: `dvd_categorizer_web/src/hooks/use_sse.rs`

```rust
use dioxus::prelude::*;
use web_sys::{EventSource, MessageEvent};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

pub fn use_sse_chat(
    url: String,
    on_chunk: impl Fn(String) + 'static,
    on_done: impl Fn() + 'static,
    on_error: impl Fn(String) + 'static,
) -> Signal<Option<EventSource>> {
    let mut event_source = use_signal(|| None::<EventSource>);
    
    use_effect(move || {
        let es = EventSource::new(&url).ok();
        
        if let Some(ref source) = es {
            // Handle message chunks
            let on_chunk = Closure::wrap(Box::new(move |e: MessageEvent| {
                if let Some(data) = e.data().as_string() {
                    on_chunk(data);
                }
            }) as Box<dyn FnMut(_)>);
            
            source.add_event_listener_with_callback(
                "message",
                on_chunk.as_ref().unchecked_ref()
            ).ok();
            on_chunk.forget();
            
            // Handle done event
            let on_done = Closure::wrap(Box::new(move |_: MessageEvent| {
                on_done();
            }) as Box<dyn FnMut(_)>);
            
            source.add_event_listener_with_callback(
                "done",
                on_done.as_ref().unchecked_ref()
            ).ok();
            on_done.forget();
            
            // Handle errors
            let on_error = Closure::wrap(Box::new(move |e: MessageEvent| {
                if let Some(data) = e.data().as_string() {
                    on_error(data);
                }
            }) as Box<dyn FnMut(_)>);
            
            source.add_event_listener_with_callback(
                "error",
                on_error.as_ref().unchecked_ref()
            ).ok();
            on_error.forget();
        }
        
        event_source.set(es);
    });
    
    event_source
}
```

### 2. Update Chat Component
File: `dvd_categorizer_web/src/components/chat.rs`

```rust
#[component]
pub fn ChatInterface() -> Element {
    let mut messages = use_signal(Vec::<ChatMessage>::new);
    let mut current_response = use_signal(String::new);
    let mut is_streaming = use_signal(|| false);
    
    let send_message = move |user_message: String| {
        // Add user message
        messages.write().push(ChatMessage {
            role: "user".to_string(),
            content: user_message.clone(),
        });
        
        // Clear current response
        current_response.set(String::new());
        is_streaming.set(true);
        
        // Start SSE stream
        spawn(async move {
            let request = ChatRequest {
                messages: messages.read().clone(),
                model: Some("llama3.2:latest".to_string()),
            };
            
            let body = serde_json::to_string(&request).unwrap();
            
            // POST to start stream
            let response = gloo_net::http::Request::post("/api/chat/stream")
                .header("Content-Type", "application/json")
                .body(body)
                .send()
                .await
                .unwrap();
            
            // Read SSE stream
            let reader = response.body().unwrap().get_reader();
            // ... handle stream reading ...
        });
    };
    
    rsx! {
        div { class: "chat-container",
            // Message history
            for msg in messages.read().iter() {
                div { class: "message {msg.role}",
                    "{msg.content}"
                }
            }
            
            // Streaming response
            if is_streaming() {
                div { class: "message assistant streaming",
                    "{current_response}"
                    span { class: "cursor", "▋" }
                }
            }
            
            // Input
            ChatInput { on_send: send_message }
        }
    }
}
```

## Testing Plan

1. **Basic streaming:** Send message, verify chunks arrive in order
2. **Multiple concurrent streams:** Two users chat simultaneously
3. **Error handling:** Kill Ollama mid-stream, verify error event
4. **Reconnection:** Network drop, verify auto-reconnect
5. **Long responses:** Multi-paragraph response streams correctly

## Benefits

- ✓ Real-time typing effect
- ✓ User sees progress immediately
- ✓ Better perceived performance
- ✓ No blocking - handle multiple chats concurrent
- ✓ Auto-reconnect on network issues
- ✓ Works with existing REST API

## Estimated Time
2-3 hours (backend + frontend + testing)

## Notes

- SSE one-way only (server → client) - perfect for AI responses
- Browser limit: 6 concurrent SSE connections per domain
- Keep-alive prevents connection timeout
- EventSource API built into browsers (no library needed)
- Falls back gracefully if SSE not supported

## After Phase 3

Consider:
- Tool calling with streaming (show tool execution progress)
- Streaming embeddings for bulk operations
- Progress bars for long operations
