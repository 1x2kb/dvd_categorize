use dioxus::signals::Signal;
use models::RoledMessage;
use serde::{Deserialize, Serialize};

// WASM-compatible ChatSession (no diesel/postgres deps)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSession {
    pub id: i32,
    pub session_id: String,
    pub created_at: String,
    pub updated_at: String,
    pub first_query: Option<String>,
}

// Global state structure
#[derive(Clone)]
pub struct AppData {
    pub ai_chat: Signal<Option<Vec<RoledMessage>>>,
    pub chat_session_id: Signal<Option<String>>,
    pub chat_sessions: Signal<Vec<ChatSession>>,
}
