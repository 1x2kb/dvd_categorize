//! Message conversion utilities for Ollama chat integration.
//!
//! Provides helper functions to convert between application-specific message
//! formats and Ollama's chat message format.

use models::{ChatMessage, RoledMessage, Role};

/// Convert a database ChatMessage to Ollama's ChatMessage format.
///
/// Maps role strings to appropriate Ollama message types:
/// - "user" -> user message
/// - anything else -> assistant message
pub fn db_msg_to_ollama(msg: &ChatMessage) -> ollama_rs::generation::chat::ChatMessage {
    match msg.role.as_str() {
        "user" => ollama_rs::generation::chat::ChatMessage::user(msg.content.clone()),
        _ => ollama_rs::generation::chat::ChatMessage::assistant(msg.content.clone()),
    }
}

/// Convert a RoledMessage to Ollama's ChatMessage format.
///
/// Maps the Role enum to appropriate Ollama message types:
/// - Role::User -> user message
/// - Role::Ai -> assistant message
pub fn model_msg_to_ollama(msg: &RoledMessage) -> ollama_rs::generation::chat::ChatMessage {
    match msg.role {
        Role::User => ollama_rs::generation::chat::ChatMessage::user(msg.message.clone()),
        Role::Ai => ollama_rs::generation::chat::ChatMessage::assistant(msg.message.clone()),
    }
}
