use std::{future::Future, sync::Arc};

use log::{error, info};
use models::{question::AiAction, FullMovie};
use ollama_rs::{
    generation::chat::{request::ChatMessageRequest, ChatMessage},
    Ollama,
};

pub trait GenerateMessage {
    fn generate_message(prompt: String) -> impl Future<Output = String>;
}

pub trait Chat {
    fn chat(prompt: String) -> impl Future<Output = String>;
}

pub struct OllamaClient {
    pub host: String,
    pub port: String,
}

pub async fn ai_message(ai_action: AiAction, dvds: Arc<Vec<FullMovie>>) -> Result<String, String> {
    let mut count = 0u8;

    let ai_action = Arc::new(ai_action);
    let mut verification = false;
    let model = ai_action
        .model
        .as_ref()
        .map(|model| model.to_string())
        .unwrap_or_else(|| "mistral".to_string());

    let mut response = String::new();

    while !verification && count < 3 {
        let ai_response = bot_message(
            Arc::clone(&ai_action),
            &dvds,
        )
        .await;

        verification = match ai_response {
            Ok(answer) => {
                response = answer;
                verify_message(
                    &ai_action.action,
                    &response,
                    model.to_string(),
                )
                .await
            }
            Err(e) => {
                error!("{e}");
                count += 1;
                false
            }
        };
    }

    Ok(response)
}

pub async fn bot_message(ai_action: Arc<AiAction>, dvds: &[FullMovie]) -> Result<String, String> {
    // Initialize Ollama (default connects to localhost:11434)
    let ollama = Ollama::default();

    let prompt = format!(
        "You are a movie expert, you may answer questions only when using information from the users library. The following list of movies and shows represent the users personal library. Use this list to answer questions on recommendations or how many movies do I own. \n\n{}\n\n It is against the law to lie, and saying the user owns dvds that they do not is a lie. I do not want you to go to prison.", serde_json::to_string(&dvds).unwrap_or_else(|_| "".to_string())
    );

    let messages = vec![
        ChatMessage::system(prompt),
        ChatMessage::user(
            ai_action
                .action
                .to_string(),
        ),
    ];

    let chat_request = ChatMessageRequest::new(
        ai_action
            .model
            .as_ref()
            .map(|model| model.to_string())
            .unwrap_or_else(|| "mistral".to_string()),
        messages,
    );

    // Generate a response
    let response = ollama
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

async fn verify_message(question: &str, answer: &str, model: String) -> bool {
    let ollama = Ollama::default();

    let prompt =
        format!("You are an expert on communcatation and reasoning. Your job is to decide if the user's question was answered by the AI. Do not be overly literal, the answer given does not have to be perfect. Your job is to decide if it fits. When giving a response please respond with yes or no, and then why or why not.");

    let question_message = format!(
        "User question: {}\n\nAI Answer: {}",
        question, answer
    );

    let verification_response = ollama
        .send_chat_messages(
            ChatMessageRequest::new(
                model,
                vec![
                    ChatMessage::system(prompt),
                    ChatMessage::user(question_message),
                ],
            ),
        )
        .await;

    info!(
        "Response from verification: {}",
        verification_response
            .as_ref()
            .map(
                |value| value
                    .message
                    .content
                    .to_string()
            )
            .unwrap_or_else(|e| format!("Error occurred {e}"))
    );

    verification_response
        .as_ref()
        .map(
            |response| {
                response
                    .message
                    .content
                    .trim()
                    .to_lowercase()
                    .get(0..3)
                    .is_some_and(|response_str| response_str.contains("yes"))
            },
        )
        .unwrap_or(false)
}

pub async fn save_message() {}

#[cfg(test)]
mod tests {}
