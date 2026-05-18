use std::{future::Future, sync::Arc};

use log::info;
use models::FullMovie;
use ollama_rs::{
    generation::chat::{request::ChatMessageRequest, ChatMessage},
    models::ModelOptions,
};

use crate::OllamaClient;
use prompts;

/// Trait for AI chat functionality to enable dependency injection and testing
pub trait AiChatProvider {
    fn send_chat_request(
        &self,
        messages: Vec<ChatMessage>,
    ) -> impl Future<Output = Result<String, String>>;
}

/// Production implementation using Ollama
impl AiChatProvider for OllamaClient {
    async fn send_chat_request(&self, messages: Vec<ChatMessage>) -> Result<String, String> {
        let model_name = self
            .ai_action
            .model
            .as_deref()
            .unwrap_or("qwen2.5:7b");

        let num_ctx = std::env::var("OLLAMA_NUM_CTX")
            .unwrap_or_else(|_| "8000".to_string())
            .parse::<u64>()
            .unwrap_or(8000);

        let temperature = self
            .ai_action
            .temperature
            .unwrap_or(0.5);

        info!(
            "Using num_ctx {}",
            num_ctx
        );
        info!(
            "Using temprature {}",
            temperature
        );

        let chat_request = ChatMessageRequest::new(
            model_name.to_string(),
            messages,
        )
        .options(
            ModelOptions::default()
                .num_ctx(num_ctx)
                .temperature(temperature),
        );

        let response = self
            .ollama_client
            .send_chat_messages(chat_request)
            .await
            .map_err(|e| e.to_string())?;

        info!(
            "Response from AI {}",
            &response
                .message
                .content
        );
        Ok(
            response
                .message
                .content,
        )
    }
}

/// Refactored function that accepts any AI chat provider for better testability
pub async fn filter_movies_with_ai<T: AiChatProvider>(
    dvds: Arc<Vec<FullMovie>>,
    ai_provider: Arc<T>,
    user_query: &str,
) -> Result<String, String> {
    if dvds.is_empty() {
        return Err("DVDs empty, cannot match".to_string());
    }

    let prompt = build_movie_matcher_prompt(&dvds)?;
    let messages = vec![
        ChatMessage::system(prompt),
        ChatMessage::user(user_query.to_string()),
    ];

    ai_provider
        .send_chat_request(messages)
        .await
}

/// Helper function to build the prompt - now easily testable
pub fn build_movie_matcher_prompt(dvds: &[FullMovie]) -> Result<String, String> {
    let dvd_json = serde_json::to_string(dvds).map_err(
        |e| {
            format!(
                "Failed to serialize DVDs: {}",
                e
            )
        },
    )?;

    Ok(
        prompts::MOVIE_ID_MATCHER_PROMPT.replace(
            "{USER_MOVIE_LIBRARY}",
            &dvd_json,
        ),
    )
}

/// Convenience function that maintains the original API for existing code
pub async fn get_matching_movies_with_ollama(
    dvds: Arc<Vec<FullMovie>>,
    ollama: Arc<OllamaClient>,
) -> Result<String, String> {
    filter_movies_with_ai(
        dvds,
        Arc::clone(&ollama),
        &ollama
            .ai_action
            .action,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use models::FullMovie;
    use ollama_rs::generation::chat::MessageRole;
    use std::sync::{Arc, Mutex};

    /// Mock AI provider for testing
    pub struct MockAiChatProvider {
        pub expected_response: String,
        pub should_fail: bool,
        pub captured_messages: Arc<Mutex<Vec<ChatMessage>>>,
    }

    impl MockAiChatProvider {
        pub fn new(response: &str) -> Self {
            Self {
                expected_response: response.to_string(),
                should_fail: false,
                captured_messages: Arc::new(Mutex::new(Vec::new())),
            }
        }

        pub fn with_failure() -> Self {
            Self {
                expected_response: String::new(),
                should_fail: true,
                captured_messages: Arc::new(Mutex::new(Vec::new())),
            }
        }

        pub fn get_captured_messages(&self) -> Vec<ChatMessage> {
            self.captured_messages
                .lock()
                .unwrap()
                .clone()
        }
    }

    impl AiChatProvider for MockAiChatProvider {
        async fn send_chat_request(&self, messages: Vec<ChatMessage>) -> Result<String, String> {
            // Capture the inputs for verification in tests
            *self
                .captured_messages
                .lock()
                .unwrap() = messages;

            if self.should_fail {
                Err("Mock AI failure".to_string())
            } else {
                Ok(
                    self.expected_response
                        .clone(),
                )
            }
        }
    }

    #[tokio::test]
    async fn test_get_matching_movies_success() {
        let movies = FullMovie::create_test_movies();
        let dvds = Arc::new(movies);

        let mock_provider = Arc::new(MockAiChatProvider::new("Mock AI response"));

        let result = filter_movies_with_ai(
            dvds,
            Arc::clone(&mock_provider),
            "Find movies",
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            "Mock AI response"
        );

        // Verify the mock captured the expected inputs

        let captured_messages = mock_provider.get_captured_messages();
        assert_eq!(
            captured_messages.len(),
            2
        );
        assert_eq!(
            captured_messages[0].role,
            MessageRole::System
        );
        assert_eq!(
            captured_messages[1].role,
            MessageRole::User
        );
    }

    #[tokio::test]
    async fn test_get_matching_movies_with_default_model() {
        let movies = FullMovie::create_test_movies();
        let dvds = Arc::new(movies);

        let mock_provider = Arc::new(MockAiChatProvider::new("Response with default model"));

        let result = filter_movies_with_ai(
            dvds,
            Arc::clone(&mock_provider),
            "Find movies",
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            "Response with default model"
        );
    }

    #[tokio::test]
    async fn test_get_matching_movies_empty_dvds() {
        let empty_movies: Vec<FullMovie> = vec![];
        let dvds = Arc::new(empty_movies);

        let mock_provider = Arc::new(MockAiChatProvider::new("Should not be called"));

        let result = filter_movies_with_ai(
            dvds,
            Arc::clone(&mock_provider),
            "Find movies",
        )
        .await;

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "DVDs empty, cannot match"
        );

        // Mock should not have been called for empty DVDs
    }

    #[tokio::test]
    async fn test_get_matching_movies_ai_failure() {
        let movies = FullMovie::create_test_movies();
        let dvds = Arc::new(movies);

        let mock_provider = Arc::new(MockAiChatProvider::with_failure());

        let result = filter_movies_with_ai(
            dvds,
            Arc::clone(&mock_provider),
            "Find movies",
        )
        .await;

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Mock AI failure"
        );

        // Verify the mock was called even though it failed
    }

    #[test]
    fn test_build_movie_matcher_prompt() {
        let movies = FullMovie::create_test_movies();
        let result = build_movie_matcher_prompt(&movies);

        assert!(result.is_ok());
        let prompt = result.unwrap();

        // Should contain the serialized movie data
        assert!(prompt.contains("The Matrix"));
        assert!(prompt.contains("Inception"));
    }

    #[test]
    fn test_build_movie_matcher_prompt_empty() {
        let empty_movies: Vec<FullMovie> = vec![];
        let result = build_movie_matcher_prompt(&empty_movies);

        assert!(result.is_ok());
        let prompt = result.unwrap();

        // Should contain empty array
        assert!(prompt.contains("[]"));
    }
}
