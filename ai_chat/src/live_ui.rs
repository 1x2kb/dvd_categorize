use std::sync::Arc;

use log::info;
use models::FullMovie;
use ollama_rs::{
    generation::{
        chat::{request::ChatMessageRequest, ChatMessage},
        options::GenerationOptions,
    },
    Ollama,
};

use crate::{prompts, OllamaClient};

pub async fn get_matching_movies(
    dvds: Arc<&[FullMovie]>,
    ollama: Arc<OllamaClient>,
) -> Result<String, String> {
    if dvds.is_empty() {
        return Err("DVDs empty, cannot match".to_string());
    }

    let prompt = format!(
        "{}",
        prompts::MOVIE_ID_MATCHER_PROMPT
    )
    .replace(
        "{USER_MOVIE_LIBRARY}",
        &serde_json::to_string(&**dvds).unwrap_or_else(|_| "".to_string()),
    );

    let messages = vec![
        ChatMessage::system(prompt),
        ChatMessage::user(
            ollama
                .ai_action
                .action
                .to_string(),
        ),
    ];

    // Create the request object, controls the AIs options and passes the message.
    let chat_request = ChatMessageRequest::new(
        ollama
            .ai_action
            .model
            .as_ref()
            .map(|model| model.to_string())
            .unwrap_or_else(|| "mistral".to_string()),
        messages,
    )
    .options(GenerationOptions::default().num_ctx(64000));

    let response = ollama
        .ollama_client
        .send_chat_messages(chat_request)
        .await
        .map_err(|e| e.to_string())?;

    info!(
        "Response from AI {}",
        response
            .message
            .content
    );

    Ok(
        response
            .message
            .content,
    )
}
