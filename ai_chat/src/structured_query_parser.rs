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
- title_keywords: ONLY use for specific movie title references (e.g., "the matrix", "inception")
- description_keywords: Use for plot themes, subjects, or content (e.g., "nazi", "nazis", "space", "robot", "love story")

CRITICAL: title_keywords should ONLY contain words when the user is searching for a specific movie title. Words describing movie content, themes, or subjects go in description_keywords.

IMPORTANT: For description_keywords, include BOTH singular and plural forms (e.g., "nazi" AND "nazis", "robot" AND "robots"). This increases match likelihood.

IMPORTANT: Return ONLY a valid JSON object with these exact fields. Do not include any explanation, notes, or extra text.

Example 1:
User: "Show me brad pitt action adventure movies"
Response: {"actors":["brad pitt"],"directors":[],"genres":["action","adventure"],"title_keywords":[],"description_keywords":[]}

Example 2:
User: "brad pitt movies about nazis"
Response: {"actors":["brad pitt"],"directors":[],"genres":[],"title_keywords":[],"description_keywords":["nazi","nazis"]}

Example 3:
User: "christopher nolan space movies"
Response: {"actors":[],"directors":["christopher nolan"],"genres":[],"title_keywords":[],"description_keywords":["space"]}

Example 4:
User: "funny robot movies"
Response: {"actors":[],"directors":[],"genres":["comedy"],"title_keywords":[],"description_keywords":["robot","robots","funny"]}

Example 5:
User: "the matrix"
Response: {"actors":[],"directors":[],"genres":[],"title_keywords":["matrix"],"description_keywords":[]}

Example 6:
User: "movies about time travel"
Response: {"actors":[],"directors":[],"genres":[],"title_keywords":[],"description_keywords":["time travel"]}

Example 7:
User: "tom hanks war movies"
Response: {"actors":["tom hanks"],"directors":[],"genres":[],"title_keywords":[],"description_keywords":["war"]}

Example 8:
User: "inception"
Response: {"actors":[],"directors":[],"genres":[],"title_keywords":["inception"],"description_keywords":[]}

Rules:
- Always return valid JSON
- Use lowercase for all values
- Keep actor/director names as they appear in the query
- Map common genre synonyms: funny->comedy, scary->horror, sci-fi/science fiction->sci-fi
- title_keywords: ONLY for specific movie title searches (rare)
- description_keywords: For themes, subjects, plot elements, content descriptors (common)
- When in doubt, use description_keywords NOT title_keywords
- Phrases like "about X", "X movies", "movies with X" → X goes in description_keywords
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

/// Extracts JSON from AI response, handling cases where the AI adds extra text or markdown code blocks
fn extract_json_from_response(content: &str) -> String {
    let mut content = content.trim();

    // Remove markdown code blocks if present
    if content.starts_with("```json") {
        content = content.strip_prefix("```json").unwrap_or(content).trim();
    } else if content.starts_with("```") {
        content = content.strip_prefix("```").unwrap_or(content).trim();
    }

    if content.ends_with("```") {
        content = content.strip_suffix("```").unwrap_or(content).trim();
    }

    // Find the first complete JSON object by tracking brace depth
    if let Some(start) = content.find('{') {
        let mut depth = 0;
        let mut in_string = false;
        let mut escape_next = false;
        
        for (i, ch) in content[start..].char_indices() {
            if escape_next {
                escape_next = false;
                continue;
            }
            
            match ch {
                '\\' if in_string => escape_next = true,
                '"' => in_string = !in_string,
                '{' if !in_string => depth += 1,
                '}' if !in_string => {
                    depth -= 1;
                    if depth == 0 {
                        // Found the end of the first complete JSON object
                        return content[start..=start + i].to_string();
                    }
                }
                _ => {}
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
            // Plain JSON
            (
                r#"{"actors":["brad pitt"],"directors":[],"genres":["action"],"title_keywords":[],"description_keywords":[]}"#,
                r#"{"actors":["brad pitt"],"directors":[],"genres":["action"],"title_keywords":[],"description_keywords":[]}"#,
            ),
            // JSON with prefix text
            (
                r#"Here is the result: {"actors":[],"directors":[],"genres":[],"title_keywords":[],"description_keywords":[]}"#,
                r#"{"actors":[],"directors":[],"genres":[],"title_keywords":[],"description_keywords":[]}"#,
            ),
            // JSON with suffix text
            (
                r#"{"actors":["tom hanks"],"directors":[],"genres":["comedy"],"title_keywords":[],"description_keywords":[]} - This is a comedy movie"#,
                r#"{"actors":["tom hanks"],"directors":[],"genres":["comedy"],"title_keywords":[],"description_keywords":[]}"#,
            ),
            // Markdown code block with json tag
            (
                r#"```json
{"actors":["brad pitt"],"directors":[],"genres":[],"title_keywords":[],"description_keywords":["nazis"]}
```"#,
                r#"{"actors":["brad pitt"],"directors":[],"genres":[],"title_keywords":[],"description_keywords":["nazis"]}"#,
            ),
            // Markdown code block without json tag
            (
                r#"```
{"actors":["brad pitt"],"directors":[],"genres":[],"title_keywords":[],"description_keywords":["nazis"]}
```"#,
                r#"{"actors":["brad pitt"],"directors":[],"genres":[],"title_keywords":[],"description_keywords":["nazis"]}"#,
            ),
            // Markdown with extra text after closing backticks
            (
                r#"{"actors":["brad pitt"],"directors":[],"genres":[],"title_keywords":[],"description_keywords":["nazis"]}

    ```json
    {
      "actors": ["brad pitt"],
      "directors": [],
      "genres": [],
      "title_keywords": [],
      "description_keywords": ["nazis"]
    }"#,
                r#"{"actors":["brad pitt"],"directors":[],"genres":[],"title_keywords":[],"description_keywords":["nazis"]}"#,
            ),
        ];

        for (input, expected) in cases {
            let result = extract_json_from_response(input);
            assert_eq!(result, expected, "Failed for input: {}", input);
        }
    }
}
