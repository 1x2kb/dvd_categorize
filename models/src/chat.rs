use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Response {
    pub model: String,
    pub created_at: String,
    pub message: Message,
    pub done: bool,
    pub done_reason: Option<String>,
    pub total_duration: Option<i64>,
    pub load_duration: Option<i64>,
    pub prompt_eval_count: Option<i64>,
    pub prompt_eval_duration: Option<i64>,
    pub eval_count: Option<i64>,
    pub eval_duration: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}
