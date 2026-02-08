use log::{debug, error, info};
use models::StructuredQuery;
use ollama_rs::{
    generation::chat::{request::ChatMessageRequest, ChatMessage},
    Ollama,
};

use crate::{DEFAULT_OLLAMA_HOST, DEFAULT_OLLAMA_PORT, DEFAULT_SMALL_MODEL};

/// Parses a natural language query into a structured format for Diesel query building
///
/// # Arguments
/// * `query` - The user's natural language query
/// * `model` - Optional AI model to use for parsing
///
/// # Returns
/// A StructuredQuery containing actors, directors, genres, and keywords extracted from the query
pub async fn parse_query_to_structured(
    query: &str,
    model: Option<&str>,
) -> Result<StructuredQuery, String> {
    info!("Parsing query to structured format: {}", query);

    let ollama_host =
        std::env::var("OLLAMA_HOST").unwrap_or_else(|_| DEFAULT_OLLAMA_HOST.to_string());
    let ollama_port =
        std::env::var("OLLAMA_PORT").unwrap_or_else(|_| DEFAULT_OLLAMA_PORT.to_string());
    let ollama_url = format!("http://{}:{}", ollama_host, ollama_port);

    let ollama = match ollama_url.parse() {
        Ok(url) => Ollama::from_url(url),
        Err(e) => {
            let error_msg = format!("Failed to parse Ollama URL: {}", e);
            error!("{}", error_msg);
            return Err(error_msg);
        }
    };

    let system_prompt = r#"You are a movie search query parser. Your job is to extract structured search criteria from natural language queries.

Parse the user's query and extract:
- actors: List of actor names mentioned
- directors: List of director names mentioned
- genres: List of genres mentioned (e.g., action, comedy, sci-fi, thriller, drama, horror, adventure)
- title_keywords: Keywords that might be part of a movie title
- description_keywords: Keywords that describe the movie plot or theme

IMPORTANT: Return ONLY a valid JSON object with these exact fields. Do not include any explanation, notes, or extra text.

Example 1:
User: "Show me brad pitt action adventure movies"
Response: {"actors":["brad pitt"],"directors":[],"genres":["action","adventure"],"title_keywords":[],"description_keywords":[]}

Example 2:
User: "christopher nolan space movies"
Response: {"actors":[],"directors":["christopher nolan"],"genres":[],"title_keywords":[],"description_keywords":["space"]}

Example 3:
User: "funny robot movies"
Response: {"actors":[],"directors":[],"genres":["comedy"],"title_keywords":[],"description_keywords":["robot","funny"]}

Example 4:
User: "the matrix"
Response: {"actors":[],"directors":[],"genres":[],"title_keywords":["matrix"],"description_keywords":[]}

Example 5:
User: "tom hanks comedy"
Response: {"actors":["tom hanks"],"directors":[],"genres":["comedy"],"title_keywords":[],"description_keywords":[]}

Rules:
- Always return valid JSON
- Use lowercase for all values
- Keep actor/director names as they appear in the query
- Map common genre synonyms: funny->comedy, scary->horror, sci-fi/science fiction->sci-fi
- If unclear whether something is a title keyword or description keyword, prefer description_keywords
- All arrays can be empty if nothing is found
"#;

    let user_message = format!("User: \"{}\"\nResponse:", query);

    let model_name = model.unwrap_or(DEFAULT_SMALL_MODEL);
    debug!("Using model for query parsing: {}", model_name);

    let request = ChatMessageRequest::new(
        model_name.to_string(),
        vec![
            ChatMessage::system(system_prompt.to_string()),
            ChatMessage::user(user_message),
        ],
    );

    match ollama.send_chat_messages(request).await {
        Ok(response) => {
            let content = response.message.content.trim();
            debug!("Raw AI response: {}", content);

            let json_content = extract_json_from_response(content);
            debug!("Extracted JSON: {}", json_content);

            match serde_json::from_str::<StructuredQuery>(&json_content) {
                Ok(structured) => {
                    info!("Successfully parsed query to: {:?}", structured);
                    Ok(structured)
                }
                Err(e) => {
                    let error_msg = format!("Failed to parse JSON response: {}. JSON content was: {}", e, json_content);
                    error!("{}", error_msg);
                    Err(error_msg)
                }
            }
        }
        Err(e) => {
            let error_msg = format!("Failed to get AI response: {}", e);
            error!("{}", error_msg);
            Err(error_msg)
        }
    }
}

/// Extracts JSON from AI response, handling cases where the AI adds extra text
fn extract_json_from_response(content: &str) -> String {
    let content = content.trim();

    if let Some(start) = content.find('{') {
        if let Some(end) = content.rfind('}') {
            if start <= end {
                return content[start..=end].to_string();
            }
        }
    }

    content.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_from_response() {
        let cases = vec![
            (
                r#"{"actors":["brad pitt"],"directors":[],"genres":["action"],"title_keywords":[],"description_keywords":[]}"#,
                r#"{"actors":["brad pitt"],"directors":[],"genres":["action"],"title_keywords":[],"description_keywords":[]}"#,
            ),
            (
                r#"Here is the result: {"actors":[],"directors":[],"genres":[],"title_keywords":[],"description_keywords":[]}"#,
                r#"{"actors":[],"directors":[],"genres":[],"title_keywords":[],"description_keywords":[]}"#,
            ),
            (
                r#"{"actors":["tom hanks"],"directors":[],"genres":["comedy"],"title_keywords":[],"description_keywords":[]} - This is a comedy movie"#,
                r#"{"actors":["tom hanks"],"directors":[],"genres":["comedy"],"title_keywords":[],"description_keywords":[]}"#,
            ),
        ];

        for (input, expected) in cases {
            let result = extract_json_from_response(input);
            assert_eq!(result, expected);
        }
    }
}
