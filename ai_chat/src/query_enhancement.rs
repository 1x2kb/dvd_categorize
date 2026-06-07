use log::{debug, error};
use ollama_rs::generation::chat::{request::ChatMessageRequest, ChatMessage};

use crate::{make_ollama_client, DEFAULT_SMALL_MODEL};

/// Enhances a user query for better semantic search by expanding it into a more descriptive phrase
///
/// # Arguments
/// * `query` - The original user query
/// * `model` - Optional AI model to use for enhancement. Defaults to DEFAULT_SMALL_MODEL if None
///
/// # Returns
/// An enhanced query string suitable for embedding, or the original query if enhancement fails
pub async fn enhance_query_for_embedding(query: &str, model: Option<&str>) -> String {
    debug!(
        "Enhancing query for embedding: {}",
        query
    );

    // Skip enhancement for already descriptive queries (3+ words)
    if query
        .split_whitespace()
        .count()
        >= 3
    {
        debug!("Query already descriptive, skipping enhancement");
        return query.to_string();
    }

    let ollama = match make_ollama_client() {
        Ok(o) => o,
        Err(e) => {
            error!("Failed to create Ollama client: {}", e);
            return query.to_string();
        }
    };

    let system_prompt = r#"You are a query enhancement assistant for movie search. 
Your job is to take a short user query and expand it into a descriptive phrase that will work better for semantic search.

Rules:
- Keep it concise (1-2 sentences max)
- Focus on the theme, genre, or content
- Use movie-related context
- Do NOT add movies titles, actor names, or specific examples. You MUST include titles or actors if the user typed them.
- IMPORTANT! Return ONLY the enhanced query, nothing else. Do not provide a reason why you are returning this query. Return ONLY the query. This is crucial!
- Do not include notes or explanations.

Examples:
User: "Robot"
You: movies about robots and robotic characters

User: "space"
You: science fiction films set in outer space

User: "love story"
You: romantic movies about love relationships and romance

User: "funny"
You: comedy films with humor and comedic elements

User: "Brad Pitt"
You: movies including Brad Pitt as staring or signifigant supporting role.
"#;

    let user_message = format!(
        "User: \"{}\"\nYou:",
        query
    );

    let model_name = model.unwrap_or(DEFAULT_SMALL_MODEL);
    debug!(
        "Using model for query enhancement: {}",
        model_name
    );

    let request = ChatMessageRequest::new(
        model_name.to_string(),
        vec![
            ChatMessage::system(system_prompt.to_string()),
            ChatMessage::user(user_message),
        ],
    );

    match ollama
        .send_chat_messages(request)
        .await
    {
        Ok(response) => {
            let original_enhanced = response
                .message
                .content
                .trim()
                .to_string();
            let mut enhanced = original_enhanced.clone();
            let mut was_cleaned = false;

            // 1. Keep only the first line (remove everything after newline)
            if let Some(first_line) = enhanced
                .lines()
                .next()
            {
                if enhanced
                    .lines()
                    .count()
                    > 1
                {
                    debug!("Removed content after first line");
                    was_cleaned = true;
                }
                enhanced = first_line
                    .trim()
                    .to_string();
            }

            // 2. Remove everything after opening parenthesis (including the parenthesis)
            if let Some(start) = enhanced.find('(') {
                let removed = &enhanced[start..];
                debug!(
                    "Stripped content after parenthesis: {}",
                    removed
                );
                enhanced = enhanced[..start]
                    .trim()
                    .to_string();
                was_cleaned = true;
            }

            // 3. Remove common explanation prefixes
            let prefixes = [
                "Note:",
                "Explanation:",
                "Justification:",
                "Reasoning:",
                "Because:",
                "This is",
            ];
            for prefix in &prefixes {
                if enhanced.starts_with(prefix) {
                    debug!(
                        "Stripped '{}' prefix from enhanced query",
                        prefix
                    );
                    enhanced = enhanced[prefix.len()..]
                        .trim()
                        .to_string();
                    was_cleaned = true;
                }
            }

            if was_cleaned {
                debug!(
                    "Cleaned enhanced query: {} -> {}",
                    original_enhanced, enhanced
                );
            } else {
                debug!(
                    "Enhanced query: {} -> {}",
                    query, enhanced
                );
            }

            enhanced
        }
        Err(e) => {
            error!(
                "Failed to enhance query: {}",
                e
            );
            query.to_string()
        }
    }
}
