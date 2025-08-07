use std::{future::Future, sync::Arc};

use log::info;
use models::FullMovie;
use ollama_rs::generation::{
    chat::{request::ChatMessageRequest, ChatMessage},
    options::GenerationOptions,
};

use crate::{prompts, OllamaClient};

/// Trait for AI chat functionality to enable dependency injection and testing
pub trait AiChatProvider {
    fn send_chat_request(
        &self,
        model: &str,
        messages: Vec<ChatMessage>,
        user_action: &str,
    ) -> impl Future<Output = Result<String, String>>;
}

/// Production implementation using Ollama
impl AiChatProvider for OllamaClient {
    async fn send_chat_request(
        &self,
        model: &str,
        messages: Vec<ChatMessage>,
        _user_action: &str,
    ) -> Result<String, String> {
        let chat_request = ChatMessageRequest::new(
            model.to_string(),
            messages,
        )
        .options(GenerationOptions::default().num_ctx(64000));

        let response = self
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
}

/// Refactored function that accepts any AI chat provider for better testability
pub async fn get_matching_movies<T: AiChatProvider>(
    dvds: Arc<&[FullMovie]>,
    ai_provider: Arc<T>,
    user_action: &str,
    model: Option<&str>,
) -> Result<String, String> {
    if dvds.is_empty() {
        return Err("DVDs empty, cannot match".to_string());
    }

    let prompt = build_movie_matcher_prompt(&**dvds)?;
    let messages = vec![
        ChatMessage::system(prompt),
        ChatMessage::user(user_action.to_string()),
    ];

    let model_name = model.unwrap_or("mistral");

    ai_provider
        .send_chat_request(
            model_name,
            messages,
            user_action,
        )
        .await
}

/// Convenience function that maintains the original API for existing code
pub async fn get_matching_movies_with_ollama(
    dvds: Arc<&[FullMovie]>,
    ollama: Arc<OllamaClient>,
) -> Result<String, String> {
    let user_action = ollama
        .ai_action
        .action
        .clone();
    let model = ollama
        .ai_action
        .model
        .as_deref();

    get_matching_movies(
        dvds,
        Arc::clone(&ollama),
        &user_action,
        model,
    )
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
        pub captured_model: Mutex<Option<String>>,
        pub captured_messages: Mutex<Vec<ChatMessage>>,
        pub captured_user_action: Mutex<Option<String>>,
    }

    impl MockAiChatProvider {
        pub fn new(response: &str) -> Self {
            Self {
                expected_response: response.to_string(),
                should_fail: false,
                captured_model: Mutex::new(None),
                captured_messages: Mutex::new(Vec::new()),
                captured_user_action: Mutex::new(None),
            }
        }

        pub fn with_failure() -> Self {
            Self {
                expected_response: String::new(),
                should_fail: true,
                captured_model: Mutex::new(None),
                captured_messages: Mutex::new(Vec::new()),
                captured_user_action: Mutex::new(None),
            }
        }

        pub fn get_captured_model(&self) -> Option<String> {
            self.captured_model
                .lock()
                .unwrap()
                .clone()
        }

        pub fn get_captured_user_action(&self) -> Option<String> {
            self.captured_user_action
                .lock()
                .unwrap()
                .clone()
        }

        pub fn get_captured_messages(&self) -> Vec<ChatMessage> {
            self.captured_messages
                .lock()
                .unwrap()
                .clone()
        }
    }

    impl AiChatProvider for MockAiChatProvider {
        async fn send_chat_request(
            &self,
            model: &str,
            messages: Vec<ChatMessage>,
            user_action: &str,
        ) -> Result<String, String> {
            // Capture the inputs for verification in tests
            *self
                .captured_model
                .lock()
                .unwrap() = Some(model.to_string());
            *self
                .captured_messages
                .lock()
                .unwrap() = messages;
            *self
                .captured_user_action
                .lock()
                .unwrap() = Some(user_action.to_string());

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
        let movies_ref: &[FullMovie] = &movies;
        let dvds = Arc::new(movies_ref);

        let mock_provider = Arc::new(MockAiChatProvider::new("Mock AI response"));

        let result = get_matching_movies(
            dvds,
            mock_provider.clone(),
            "Find action movies",
            Some("test-model"),
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            "Mock AI response"
        );

        // Verify the mock captured the expected inputs
        assert_eq!(
            mock_provider.get_captured_model(),
            Some("test-model".to_string())
        );
        assert_eq!(
            mock_provider.get_captured_user_action(),
            Some("Find action movies".to_string())
        );

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
        let movies_ref: &[FullMovie] = &movies;
        let dvds = Arc::new(movies_ref);

        let mock_provider = Arc::new(MockAiChatProvider::new("Response with default model"));

        let result = get_matching_movies(
            dvds,
            mock_provider.clone(),
            "Find comedies",
            None, // No model specified, should use default
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(
            mock_provider.get_captured_model(),
            Some("mistral".to_string())
        );
    }

    #[tokio::test]
    async fn test_get_matching_movies_empty_dvds() {
        let empty_movies: &[FullMovie] = &[];
        let dvds = Arc::new(empty_movies);

        let mock_provider = Arc::new(MockAiChatProvider::new("Should not be called"));

        let result = get_matching_movies(
            dvds,
            mock_provider.clone(),
            "Find movies",
            Some("test-model"),
        )
        .await;

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "DVDs empty, cannot match"
        );

        // Mock should not have been called for empty DVDs
        assert_eq!(
            mock_provider.get_captured_model(),
            None
        );
    }

    #[tokio::test]
    async fn test_get_matching_movies_ai_failure() {
        let movies = FullMovie::create_test_movies();
        let movies_ref: &[FullMovie] = &movies;
        let dvds = Arc::new(movies_ref);

        let mock_provider = Arc::new(MockAiChatProvider::with_failure());

        let result = get_matching_movies(
            dvds,
            mock_provider.clone(),
            "Find movies",
            Some("test-model"),
        )
        .await;

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Mock AI failure"
        );

        // Verify the mock was called even though it failed
        assert_eq!(
            mock_provider.get_captured_model(),
            Some("test-model".to_string())
        );
        assert_eq!(
            mock_provider.get_captured_user_action(),
            Some("Find movies".to_string())
        );
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
