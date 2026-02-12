pub mod embedding;
pub mod live_ui;
mod prompts;
pub mod query_enhancement;
pub mod structured_query_parser;

use std::sync::Arc;

use log::{debug, info};
use models::{dvd_filters::DvdFilters, question::AiAction, FullMovie};
use ollama_rs::{
    generation::{
        chat::{request::ChatMessageRequest, ChatMessage},
        embeddings::request::GenerateEmbeddingsRequest,
        options::GenerationOptions,
    },
    Ollama,
};
use prompts::USER_LIBRARY_PROMPT;
use tracing::{instrument, Level};

pub use embedding::*;
pub use query_enhancement::*;

// Re-export the embedding model constant for easy access
pub use embedding::EMBEDDING_MODEL;

// AI Model constants
const DEFAULT_CHAT_MODEL: &str = "phi3.5";
const DEFAULT_SMALL_MODEL: &str = "phi3.5";
const DEFAULT_OLLAMA_HOST: &str = "ollama";
const DEFAULT_OLLAMA_PORT: &str = "11434";
const DEFAULT_CONTEXT_WINDOW: u64 = 64000;

#[derive(Debug)]
pub struct OllamaClient {
    pub ollama_client: Ollama,
    pub ai_action: AiAction,
}

#[instrument(level = Level::DEBUG)]
pub async fn ai_message(
    dvds: Arc<Vec<FullMovie>>,
    ollama: Arc<OllamaClient>,
) -> Result<String, String> {
    bot_message(
        &dvds,
        Arc::clone(&ollama),
    )
    .await
    .or_else(|_| Ok(String::from("There was a problem creating an AI response to the question")))
}

#[instrument(level = Level::DEBUG)]
pub async fn find_related_keys(
    question: impl AsRef<str> + std::fmt::Debug,
) -> Result<DvdFilters, String> {
    let ollama = Ollama::default();
    let question_message = format!(
        "User question: {}",
        question.as_ref()
    );

    let messages = vec![
        ChatMessage::system(prompts::DATA_POINTS_PROMPT.to_string()),
        ChatMessage::user(question_message),
    ];

    let chat_request = ChatMessageRequest::new(
        DEFAULT_SMALL_MODEL.to_string(),
        messages,
    );

    let response = ollama
        .send_chat_messages(chat_request)
        .await
        .map_err(|e| e.to_string())?;

    info!(
        "Response from datapoints AI: {}",
        response
            .message
            .content
    );

    serde_json::from_str(
        extract_enclosed_content(
            &response
                .message
                .content,
        ),
    )
    .map_err(|e| e.to_string())
}

async fn bot_message(dvds: &[FullMovie], ollama: Arc<OllamaClient>) -> Result<String, String> {
    let prompt = USER_LIBRARY_PROMPT.replace(
        "{USER_MOVIE_LIBRARY}",
        &serde_json::to_string(&dvds).unwrap_or_default(),
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

    let model = ollama
        .ai_action
        .model
        .as_ref()
        .map(|m| m.to_string())
        .unwrap_or_else(|| DEFAULT_CHAT_MODEL.to_string());

    let chat_request = ChatMessageRequest::new(
        model, messages,
    )
    .options(GenerationOptions::default().num_ctx(DEFAULT_CONTEXT_WINDOW));

    let response = ollama
        .ollama_client
        .send_chat_messages(chat_request)
        .await
        .map_err(|e| e.to_string())?;

    info!(
        "Response from AI: {}",
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

#[instrument(level = Level::INFO)]
pub async fn get_embedding(text: &str) -> Result<Vec<f32>, ollama_rs::error::OllamaError> {
    debug!(
        "Generating embedding for text: {}",
        text
    );

    // Use ollama service name for Docker container communication
    let ollama_host =
        std::env::var("OLLAMA_HOST").unwrap_or_else(|_| DEFAULT_OLLAMA_HOST.to_string());
    let ollama_port =
        std::env::var("OLLAMA_PORT").unwrap_or_else(|_| DEFAULT_OLLAMA_PORT.to_string());

    let ollama_url = format!(
        "http://{}:{}",
        &ollama_host, &ollama_port
    );
    debug!(
        "Connecting to ollama @ {}",
        &ollama_url
    );

    let ollama = Ollama::from_url(
        ollama_url
            .parse()
            .unwrap(),
    );

    let request = GenerateEmbeddingsRequest::new(
        EMBEDDING_MODEL.to_string(),
        text.into(),
    )
    .options(GenerationOptions::default().num_ctx(8192));

    let response = ollama
        .generate_embeddings(request)
        .await?
        .embeddings
        .into_iter()
        .flatten()
        .collect();

    Ok(response)
}

fn extract_enclosed_content(s: &str) -> &str {
    if let Some(start) = s.find('{') {
        if let Some(end) = s.rfind('}') {
            if end >= start {
                return &s[start..=end];
            }
        }
    }
    s
}

/// Lists all available Ollama models
///
/// # Returns
///
/// * `Ok(Vec<models::AvailableModel>)` - List of available models with their names and sizes
/// * `Err(String)` - Error message if listing fails
#[instrument(level = Level::INFO)]
pub async fn list_models() -> Result<Vec<models::AvailableModel>, String> {
    info!("Fetching available Ollama models");

    let ollama_host =
        std::env::var("OLLAMA_HOST").unwrap_or_else(|_| DEFAULT_OLLAMA_HOST.to_string());
    let ollama_port =
        std::env::var("OLLAMA_PORT").unwrap_or_else(|_| DEFAULT_OLLAMA_PORT.to_string());

    let ollama_url = format!(
        "http://{}:{}",
        &ollama_host, &ollama_port
    );
    debug!(
        "Connecting to ollama @ {}",
        &ollama_url
    );

    let ollama = Ollama::from_url(
        ollama_url
            .parse()
            .unwrap(),
    );

    match ollama
        .list_local_models()
        .await
    {
        Ok(models_list) => {
            let available_models: Vec<models::AvailableModel> = models_list
                .into_iter()
                .map(
                    |model| models::AvailableModel {
                        name: model.name,
                        size: model.size,
                    },
                )
                .collect();

            info!(
                "Found {} available models",
                available_models.len()
            );
            Ok(available_models)
        }
        Err(e) => {
            let error_msg = format!(
                "Failed to list models: {:?}",
                e
            );
            log::error!("{}", error_msg);
            Err(error_msg)
        }
    }
}

/// Pulls an Ollama model to make it available for use
///
/// # Arguments
///
/// * `model_name` - The name of the model to pull (e.g., "phi3.5", "nomic-embed-text")
///
/// # Returns
///
/// * `Ok(String)` - Success message with model name
/// * `Err(String)` - Error message if pull fails
#[instrument(level = Level::INFO)]
pub async fn pull_model(model_name: &str) -> Result<String, String> {
    info!(
        "Attempting to pull Ollama model: {}",
        model_name
    );

    let ollama_host =
        std::env::var("OLLAMA_HOST").unwrap_or_else(|_| DEFAULT_OLLAMA_HOST.to_string());
    let ollama_port =
        std::env::var("OLLAMA_PORT").unwrap_or_else(|_| DEFAULT_OLLAMA_PORT.to_string());

    let ollama_url = format!(
        "http://{}:{}",
        &ollama_host, &ollama_port
    );
    debug!(
        "Connecting to ollama @ {}",
        &ollama_url
    );

    let ollama = Ollama::from_url(
        ollama_url
            .parse()
            .unwrap(),
    );

    match ollama
        .pull_model(
            model_name.to_string(),
            false,
        )
        .await
    {
        Ok(_) => {
            info!(
                "Successfully pulled model: {}",
                model_name
            );
            Ok(
                format!(
                    "Successfully pulled model: {}",
                    model_name
                ),
            )
        }
        Err(e) => {
            let error_msg = format!(
                "Failed to pull model {}: {:?}",
                model_name, e
            );
            log::error!("{}", error_msg);
            Err(error_msg)
        }
    }
}

#[cfg(test)]
mod tests {}
