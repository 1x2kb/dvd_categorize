use log::{debug, error};
use ollama_rs::{
    generation::chat::{request::ChatMessageRequest, ChatMessage},
    Ollama,
};

/// Enhances a user query for better semantic search by expanding it into a more descriptive phrase
/// 
/// # Arguments
/// * `query` - The original user query
/// 
/// # Returns
/// An enhanced query string suitable for embedding, or the original query if enhancement fails
pub async fn enhance_query_for_embedding(query: &str) -> String {
    debug!("Enhancing query for embedding: {}", query);
    
    // Skip enhancement for already descriptive queries (3+ words)
    if query.split_whitespace().count() >= 3 {
        debug!("Query already descriptive, skipping enhancement");
        return query.to_string();
    }
    
    let ollama_host = std::env::var("OLLAMA_HOST").unwrap_or_else(|_| "ollama".to_string());
    let ollama_port = std::env::var("OLLAMA_PORT").unwrap_or_else(|_| "11434".to_string());
    let ollama_url = format!("http://{}:{}", ollama_host, ollama_port);
    
    let ollama = match ollama_url.parse() {
        Ok(url) => Ollama::from_url(url),
        Err(e) => {
            error!("Failed to parse Ollama URL: {}", e);
            return query.to_string();
        }
    };
    
    let system_prompt = r#"You are a query enhancement assistant for movie search. 
Your job is to take a short user query and expand it into a descriptive phrase that will work better for semantic search.

Rules:
- Keep it concise (1-2 sentences max)
- Focus on the theme, genre, or content
- Use movie-related context
- Do NOT add movies titles, actor names, or specific examples
- Return ONLY the enhanced query, nothing else

Examples:
User: "Robot"
You: movies about robots and robotic characters

User: "space"
You: science fiction films set in outer space

User: "love story"
You: romantic movies about love relationships and romance

User: "funny"
You: comedy films with humor and comedic elements"#;

    let user_message = format!("User: \"{}\"\nYou:", query);
    
    let request = ChatMessageRequest::new(
        "phi3.5".to_string(),
        vec![
            ChatMessage::system(system_prompt.to_string()),
            ChatMessage::user(user_message),
        ],
    );
    
    match ollama.send_chat_messages(request).await {
        Ok(response) => {
            let enhanced = response.message.content.trim().to_string();
            debug!("Enhanced query: {} -> {}", query, enhanced);
            enhanced
        }
        Err(e) => {
            error!("Failed to enhance query: {}", e);
            query.to_string()
        }
    }
}
