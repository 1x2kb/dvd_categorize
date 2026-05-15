# Phase 4: Conversation History Management

## Status: Ready after Phase 3 complete

## Goal
Persistent chat history with built-in Ollama history tracking.

## Current State
Chat history lost on page refresh. No conversation context between sessions.

## Solution
Use Ollama's built-in history management + database persistence.

## Dependencies
Already have everything needed. Just use existing features.

## Backend Changes

### 1. Add History Table
File: `migrations/XXXX_create_chat_history.sql`

```sql
CREATE TABLE chat_sessions (
    id SERIAL PRIMARY KEY,
    session_id UUID NOT NULL UNIQUE,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE TABLE chat_messages (
    id SERIAL PRIMARY KEY,
    session_id UUID NOT NULL REFERENCES chat_sessions(session_id) ON DELETE CASCADE,
    role VARCHAR(20) NOT NULL,
    content TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_chat_messages_session ON chat_messages(session_id, created_at);
```

### 2. Add Models
File: `models/src/chat_history.rs`

```rust
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg_attr(feature="postgres", derive(Queryable, Selectable, Identifiable))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSession {
    pub id: i32,
    pub session_id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[cfg_attr(feature="postgres", derive(Insertable))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewChatSession {
    pub session_id: Uuid,
}

#[cfg_attr(feature="postgres", derive(Queryable, Selectable, Identifiable))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: i32,
    pub session_id: Uuid,
    pub role: String,
    pub content: String,
    pub created_at: NaiveDateTime,
}

#[cfg_attr(feature="postgres", derive(Insertable))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewChatMessage {
    pub session_id: Uuid,
    pub role: String,
    pub content: String,
}
```

### 3. Add Repository Methods
File: `database/src/chat_history.rs`

```rust
use models::{ChatMessage, ChatSession, NewChatMessage, NewChatSession};
use uuid::Uuid;

impl PostgresMovieRepository {
    pub async fn create_chat_session(&self) -> Result<Uuid, DatabaseError> {
        let session_id = Uuid::new_v4();
        let new_session = NewChatSession { session_id };
        
        let mut conn = self.pool.get().await?;
        diesel::insert_into(chat_sessions::table)
            .values(&new_session)
            .execute(&mut conn)
            .await?;
        
        Ok(session_id)
    }
    
    pub async fn get_chat_history(
        &self,
        session_id: Uuid,
    ) -> Result<Vec<ChatMessage>, DatabaseError> {
        let mut conn = self.pool.get().await?;
        
        chat_messages::table
            .filter(chat_messages::session_id.eq(session_id))
            .order(chat_messages::created_at.asc())
            .load(&mut conn)
            .await
            .map_err(Into::into)
    }
    
    pub async fn save_chat_message(
        &self,
        message: NewChatMessage,
    ) -> Result<(), DatabaseError> {
        let mut conn = self.pool.get().await?;
        
        diesel::insert_into(chat_messages::table)
            .values(&message)
            .execute(&mut conn)
            .await?;
        
        // Update session timestamp
        diesel::update(chat_sessions::table)
            .filter(chat_sessions::session_id.eq(message.session_id))
            .set(chat_sessions::updated_at.eq(diesel::dsl::now))
            .execute(&mut conn)
            .await?;
        
        Ok(())
    }
    
    pub async fn list_chat_sessions(
        &self,
        limit: i64,
    ) -> Result<Vec<ChatSession>, DatabaseError> {
        let mut conn = self.pool.get().await?;
        
        chat_sessions::table
            .order(chat_sessions::updated_at.desc())
            .limit(limit)
            .load(&mut conn)
            .await
            .map_err(Into::into)
    }
}
```

### 4. Update Chat Endpoint
File: `dvd_catalog_api/src/lib.rs`

```rust
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct ChatRequest {
    pub session_id: Option<Uuid>,
    pub messages: Vec<RoledMessage>,
    pub model: Option<String>,
}

pub async fn chat_stream(
    State(state): State<AppState>,
    Json(request): Json<ChatRequest>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let ollama = Ollama::default();
    let repo = state.db_pool.clone();
    
    // Get or create session
    let session_id = match request.session_id {
        Some(id) => id,
        None => repo.create_chat_session().await.unwrap(),
    };
    
    // Load history from DB
    let history_messages = repo.get_chat_history(session_id).await.unwrap_or_default();
    
    // Convert to Ollama format
    let mut messages: Vec<ChatMessage> = history_messages
        .iter()
        .map(|m| match m.role.as_str() {
            "user" => ChatMessage::user(m.content.clone()),
            "assistant" => ChatMessage::assistant(m.content.clone()),
            _ => ChatMessage::system(m.content.clone()),
        })
        .collect();
    
    // Add new user message
    messages.extend(request.messages.iter().map(|m| {
        ChatMessage::user(m.content.clone())
    }));
    
    // Save user message to DB
    for msg in &request.messages {
        repo.save_chat_message(NewChatMessage {
            session_id,
            role: "user".to_string(),
            content: msg.content.clone(),
        }).await.ok();
    }
    
    // Use Ollama history tracking
    let history = Arc::new(Mutex::new(messages));
    
    let chat_request = ChatMessageRequest::new(
        request.model.unwrap_or("llama3.2:latest".to_string()),
        vec![],  // Empty - history has everything
    );
    
    // Stream with history
    let ollama_stream = ollama
        .send_chat_messages_with_history_stream(history.clone(), chat_request)
        .await
        .unwrap();
    
    // Accumulate response for DB save
    let response_content = Arc::new(Mutex::new(String::new()));
    let response_clone = response_content.clone();
    let repo_clone = repo.clone();
    
    let sse_stream = async_stream::stream! {
        tokio::pin!(ollama_stream);
        
        while let Some(chunk) = ollama_stream.next().await {
            match chunk {
                Ok(response) => {
                    // Accumulate content
                    response_clone.lock().unwrap().push_str(&response.message.content);
                    
                    yield Ok(Event::default()
                        .event("message")
                        .data(response.message.content));
                    
                    if response.done {
                        // Save assistant response to DB
                        let content = response_clone.lock().unwrap().clone();
                        repo_clone.save_chat_message(NewChatMessage {
                            session_id,
                            role: "assistant".to_string(),
                            content,
                        }).await.ok();
                        
                        // Send session ID to frontend
                        yield Ok(Event::default()
                            .event("session")
                            .data(session_id.to_string()));
                        
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
    
    Sse::new(sse_stream).keep_alive(axum::response::sse::KeepAlive::default())
}

// List sessions endpoint
pub async fn list_sessions(
    State(state): State<AppState>,
) -> Json<Vec<ChatSession>> {
    let sessions = state.db_pool.list_chat_sessions(50).await.unwrap_or_default();
    Json(sessions)
}

// Get session history endpoint
pub async fn get_session(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
) -> Json<Vec<ChatMessage>> {
    let messages = state.db_pool.get_chat_history(session_id).await.unwrap_or_default();
    Json(messages)
}
```

### 5. Add Routes
```rust
pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/api/chat/stream", post(chat_stream))
        .route("/api/chat/sessions", get(list_sessions))
        .route("/api/chat/sessions/:id", get(get_session))
        // ... other routes
        .with_state(state)
}
```

## Frontend Changes

### 1. Session Management
File: `dvd_categorizer_web/src/state/chat.rs`

```rust
use uuid::Uuid;

#[derive(Clone, PartialEq)]
pub struct ChatState {
    pub current_session: Option<Uuid>,
    pub sessions: Vec<ChatSession>,
    pub messages: Vec<ChatMessage>,
}

pub fn use_chat_state() -> Signal<ChatState> {
    let mut state = use_signal(|| ChatState {
        current_session: None,
        sessions: vec![],
        messages: vec![],
    });
    
    // Load sessions on mount
    use_effect(move || {
        spawn(async move {
            let sessions = fetch_sessions().await.unwrap_or_default();
            state.write().sessions = sessions;
        });
    });
    
    state
}

async fn fetch_sessions() -> Result<Vec<ChatSession>, Error> {
    gloo_net::http::Request::get("/api/chat/sessions")
        .send()
        .await?
        .json()
        .await
}

async fn fetch_session_history(session_id: Uuid) -> Result<Vec<ChatMessage>, Error> {
    gloo_net::http::Request::get(&format!("/api/chat/sessions/{}", session_id))
        .send()
        .await?
        .json()
        .await
}
```

### 2. Session Sidebar
File: `dvd_categorizer_web/src/components/session_list.rs`

```rust
#[component]
pub fn SessionList(
    sessions: Signal<Vec<ChatSession>>,
    current_session: Signal<Option<Uuid>>,
    on_select: EventHandler<Uuid>,
) -> Element {
    rsx! {
        div { class: "session-list",
            button {
                class: "new-session-btn",
                onclick: move |_| on_select.call(Uuid::nil()),
                "+ New Chat"
            }
            
            for session in sessions.read().iter() {
                div {
                    class: if current_session() == Some(session.session_id) {
                        "session-item active"
                    } else {
                        "session-item"
                    },
                    onclick: move |_| on_select.call(session.session_id),
                    
                    div { class: "session-date",
                        "{format_date(session.updated_at)}"
                    }
                }
            }
        }
    }
}
```

## Benefits

- ✓ Persistent chat history across sessions
- ✓ Multiple conversation threads
- ✓ Resume conversations anytime
- ✓ Ollama handles context automatically
- ✓ DB backup of all conversations
- ✓ Session management UI

## Estimated Time
3-4 hours (migration + backend + frontend + testing)

## Testing

1. Create new session, verify UUID returned
2. Send messages, verify saved to DB
3. Refresh page, load session, verify history restored
4. Multiple sessions, verify isolation
5. Long conversation, verify context maintained

## Notes

- Ollama history = in-memory context for current conversation
- DB history = persistent storage across restarts
- Session ID in SSE event lets frontend track which session
- Consider adding session titles (first message preview)
- Consider session search/filter
- Consider export chat history feature
