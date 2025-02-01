use std::error::Error;

use uuid::Uuid;

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
    fn get_uuid() -> Uuid;
}

pub trait QuestionHistory {
    fn get_question_history(&self) -> &[String];
}

pub trait AnswerHistory {
    fn get_answer_history(&self) -> &[String];
}

pub trait GetLastQuestion {
    fn get_last_question<'a>(&'a self) -> Option<&'a str>;
}

pub trait GetLastAnswer {
    fn get_last_answer<'a>(&'a self) -> Option<&'a str>;
}

#[derive(Debug, Clone)]
pub struct AiState {
    chat_uuid: Uuid,
    question_history: Vec<String>,
    response_history: Vec<String>,
    latest_question: Option<String>,
    latest_answer: Option<String>,
}

impl AiState {
    pub fn new() -> Self {
        AiState {
            chat_uuid: Uuid::new_v4(),
            question_history: Vec::new(),
            response_history: Vec::new(),
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
        self.question_history
            .push(question);
    }
}

impl SaveAnswerHistory for AiState {
    fn save_history(&mut self, response: String) {
        self.response_history
            .push(response);
    }
}

impl QuestionHistory for AiState {
    fn get_question_history(&self) -> &[String] {
        &self.question_history
    }
}

impl AnswerHistory for AiState {
    fn get_answer_history(&self) -> &[String] {
        &self.response_history
    }
}

impl GetLastQuestion for AiState {
    fn get_last_question<'a>(&'a self) -> Option<&'a str> {
        self.latest_question
            .as_deref()
    }
}

impl GetLastAnswer for AiState {
    fn get_last_answer<'a>(&'a self) -> Option<&'a str> {
        self.latest_answer
            .as_deref()
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
            ai_state.get_question_history(),
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
            ai_state.get_answer_history(),
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
