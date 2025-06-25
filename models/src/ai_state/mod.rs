pub mod dvd_filters;

use std::error::Error;

#[cfg(feature = "ai")]
pub use ollama_rs::generation::chat::MessageRole;
use serde::{Deserialize, Serialize};

pub mod question;
pub use question::*;

pub trait ChatWithHistory {
    fn chat(&self, message: String) -> Result<String, Box<dyn Error>>;
}

pub trait SaveQuestion {
    fn save_question(&mut self, question: Option<String>);
}

pub trait SaveAnswer {
    fn save_answer(&mut self, response: Option<String>);
}

pub trait SaveQuestionHistory {
    fn save_history(&mut self, question: String);
}

pub trait SaveAnswerHistory {
    fn save_history(&mut self, response: String);
}

pub trait SendChat {
    fn send_chat(&self, question: String);
}

pub trait GetUuid {
    fn get_uuid(&self) -> &str;
}

pub trait QuestionHistory {
    fn get_question_history(&self) -> impl Iterator<Item = &str>;
}

pub trait AnswerHistory {
    fn get_answer_history(&self) -> impl Iterator<Item = &str>;
}

pub trait FullHistory {
    fn get_history(&self) -> impl Iterator<Item = &AiMessage>;
}

pub trait GetLastQuestion {
    fn get_last_question(&self) -> Option<&str>;
}

pub trait GetLastAnswer {
    fn get_last_answer(&self) -> Option<&str>;
}

pub trait SetUuid {
    fn set_uuid(&mut self, uuid: String);
}

pub trait GenerateUuid {
    fn generate_uuid(&mut self) -> &str;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiState {
    chat_uuid: String,
    chat_history: Vec<AiMessage>,
    latest_question: Option<String>,
    latest_answer: Option<String>,
}

impl Default for AiState {
    fn default() -> Self {
        Self::new()
    }
}

impl AiState {
    pub fn new() -> Self {
        AiState {
            chat_uuid: "7".to_string(),
            chat_history: Vec::new(),
            latest_question: None,
            latest_answer: None,
        }
    }
}

impl SaveQuestion for AiState {
    fn save_question(&mut self, question: Option<String>) {
        self.latest_question = question;
    }
}

impl SaveAnswer for AiState {
    fn save_answer(&mut self, response: Option<String>) {
        self.latest_answer = response;
    }
}

impl SaveQuestionHistory for AiState {
    fn save_history(&mut self, question: String) {
        self.chat_history
            .push(
                AiMessage {
                    role: MessageRole::User,
                    message: question,
                },
            );
    }
}

impl SaveAnswerHistory for AiState {
    fn save_history(&mut self, response: String) {
        self.chat_history
            .push(
                AiMessage {
                    role: MessageRole::Assistant,
                    message: response,
                },
            );
    }
}

impl QuestionHistory for AiState {
    fn get_question_history(&self) -> impl Iterator<Item = &str> {
        self.chat_history
            .iter()
            .filter_map(
                |ai_message| {
                    (ai_message.role == MessageRole::User).then_some(
                        ai_message
                            .message
                            .as_str(),
                    )
                },
            )
    }
}

impl AnswerHistory for AiState {
    fn get_answer_history(&self) -> impl Iterator<Item = &str> {
        self.chat_history
            .iter()
            .filter_map(
                |ai_message| {
                    (ai_message.role == MessageRole::Assistant).then_some(
                        ai_message
                            .message
                            .as_str(),
                    )
                },
            )
    }
}

impl FullHistory for AiState {
    fn get_history(&self) -> impl Iterator<Item = &AiMessage> {
        self.chat_history
            .iter()
    }
}

impl GetLastQuestion for AiState {
    fn get_last_question(&self) -> Option<&str> {
        self.latest_question
            .as_deref()
    }
}

impl GetLastAnswer for AiState {
    fn get_last_answer(&self) -> Option<&str> {
        self.latest_answer
            .as_deref()
    }
}

impl GetUuid for AiState {
    fn get_uuid(&self) -> &str {
        &self.chat_uuid
    }
}

impl SetUuid for AiState {
    fn set_uuid(&mut self, uuid: String) {
        self.chat_uuid = uuid;
    }
}

impl GenerateUuid for AiState {
    fn generate_uuid(&mut self) -> &str {
        self.chat_uuid = "7".to_string();
        &self.chat_uuid
    }
}

#[cfg(test)]
mod saves_state {
    use super::*;

    #[test]
    fn saves_question() {
        let mut ai_state = AiState::new();

        let question = "test string".to_string();

        ai_state.save_question(Some(question.to_string()));
        assert_eq!(
            ai_state.get_last_question(),
            Some(question.as_str())
        );

        SaveQuestionHistory::save_history(
            &mut ai_state,
            question.to_string(),
        );

        assert_eq!(
            ai_state
                .get_question_history()
                .collect::<Vec<&str>>(),
            vec![question]
        );
    }

    #[test]
    fn saves_answer() {
        let mut ai_state = AiState::new();

        let answer = "test answer".to_string();

        ai_state.save_answer(Some(answer.to_string()));

        assert_eq!(
            ai_state.get_last_answer(),
            Some(answer.as_str())
        );

        SaveAnswerHistory::save_history(
            &mut ai_state,
            answer.to_string(),
        );

        assert_eq!(
            ai_state
                .get_answer_history()
                .collect::<Vec<&str>>(),
            vec![answer.to_string()]
        );
    }

    #[test]
    fn returns_none() {
        let ai_state = AiState::new();

        assert_eq!(
            ai_state.get_last_question(),
            None
        );

        assert_eq!(
            ai_state.get_last_answer(),
            None
        );
    }
}
