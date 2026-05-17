//! Retrieval-Augmented Generation (RAG) Two-Step Flow
//!
//! This module implements a 2-step RAG process:
//! 1. **Query Extraction**: AI extracts structured `StructuredQuery` from natural language
//! 2. **Answer Generation**: AI generates response based on retrieved movies
//!
//! The flow: User Query → AI extracts StructuredQuery → Query DB → AI answers with results

use log::{error, info};
use models::{FullMovie, StructuredQuery};
use ollama_rs::{
    generation::chat::{request::ChatMessageRequest, ChatMessage},
    Ollama,
};

use prompts::RAG_ANSWER_GENERATION_PROMPT;

use crate::{
    structured_query_parser::parse_query_to_structured, DEFAULT_OLLAMA_HOST, DEFAULT_OLLAMA_PORT,
    DEFAULT_SMALL_MODEL,
};

/// Result of the RAG query extraction step
#[derive(Debug)]
pub struct ExtractedQuery {
    pub structured_query: StructuredQuery,
    pub is_movie_query: bool,
}

/// Complete RAG result containing both the answer and the source movies
#[derive(Debug)]
pub struct RagResult {
    pub answer: String,
    pub source_movies: Vec<FullMovie>,
    pub structured_query: StructuredQuery,
}

/// Step 1: Extract StructuredQuery from natural language user query
///
/// Uses AI to analyze the user's question and generate structured search parameters.
/// Returns None if the query is not about movies.
pub async fn extract_rag_query(user_message: &str, model: Option<&str>) -> Option<ExtractedQuery> {
    info!(
        "RAG Step 1: Extracting structured query from: {}",
        user_message
    );

    // Use the existing structured query parser
    match parse_query_to_structured(
        user_message,
        model,
    )
    .await
    {
        Ok(structured_query) => {
            // Check if this is actually a movie query (has any search criteria)
            let is_movie_query = !structured_query
                .actors
                .is_empty()
                || !structured_query
                    .directors
                    .is_empty()
                || !structured_query
                    .genres
                    .is_empty()
                || !structured_query
                    .title_keywords
                    .is_empty()
                || !structured_query
                    .description_keywords
                    .is_empty();

            info!(
                "RAG structured query extracted: is_movie_query={}, actors={}, directors={}, genres={}, title_kw={}, desc_kw={}",
                is_movie_query,
                structured_query.actors.len(),
                structured_query.directors.len(),
                structured_query.genres.len(),
                structured_query.title_keywords.len(),
                structured_query.description_keywords.len()
            );

            Some(
                ExtractedQuery {
                    structured_query,
                    is_movie_query,
                },
            )
        }
        Err(e) => {
            error!(
                "Failed to extract structured query: {}",
                e
            );
            None
        }
    }
}

/// Step 2: Execute database search using the extracted StructuredQuery
///
/// This function should be called by the API layer which has access to the database.
/// The StructuredQuery is passed to the database layer for execution.
pub async fn execute_rag_search(
    structured_query: &StructuredQuery,
    search_fn: impl FnOnce(
        &StructuredQuery,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Vec<FullMovie>, String>> + Send>,
    >,
) -> Result<Vec<FullMovie>, String> {
    info!("RAG Step 2: Executing structured search");

    let start_time = std::time::Instant::now();

    let results = search_fn(structured_query).await?;

    info!(
        "RAG search completed in {:?}, found {} movies",
        start_time.elapsed(),
        results.len()
    );

    Ok(results)
}

/// Step 3: Generate final answer based on retrieved movies
///
/// Uses AI to craft a natural language response that answers the user's question
/// using ONLY the movies that were retrieved from the database.
pub async fn generate_rag_answer(
    user_question: &str,
    movies: &[FullMovie],
    model: Option<&str>,
) -> Result<String, String> {
    info!(
        "RAG Step 3: Generating answer for {} movies",
        movies.len()
    );

    if movies.is_empty() {
        return Ok(
            "I couldn't find any movies in your collection that match your query.".to_string(),
        );
    }

    let ollama_host =
        std::env::var("OLLAMA_HOST").unwrap_or_else(|_| DEFAULT_OLLAMA_HOST.to_string());
    let ollama_port =
        std::env::var("OLLAMA_PORT").unwrap_or_else(|_| DEFAULT_OLLAMA_PORT.to_string());
    let ollama_url = format!(
        "http://{}:{}",
        ollama_host, ollama_port
    );

    let ollama = match ollama_url.parse() {
        Ok(url) => Ollama::from_url(url),
        Err(e) => {
            return Err(
                format!(
                    "Failed to parse Ollama URL: {}",
                    e
                ),
            );
        }
    };

    // Format movies for the prompt
    let movies_json = match serde_json::to_string(movies) {
        Ok(json) => json,
        Err(e) => {
            return Err(
                format!(
                    "Failed to serialize movies: {}",
                    e
                ),
            )
        }
    };

    let system_prompt = RAG_ANSWER_GENERATION_PROMPT
        .replace(
            "{RETRIEVED_MOVIES}",
            &movies_json,
        )
        .replace(
            "{USER_QUESTION}",
            user_question,
        );

    let model_name = model.unwrap_or(DEFAULT_SMALL_MODEL);
    let request = ChatMessageRequest::new(
        model_name.to_string(),
        vec![
            ChatMessage::system(system_prompt),
            ChatMessage::user(user_question.to_string()),
        ],
    );

    match ollama
        .send_chat_messages(request)
        .await
    {
        Ok(response) => {
            let answer = response
                .message
                .content
                .trim()
                .to_string();
            info!(
                "RAG answer generated ({} chars)",
                answer.len()
            );
            Ok(answer)
        }
        Err(e) => {
            error!(
                "Failed to generate RAG answer: {}",
                e
            );
            Err(
                format!(
                    "Failed to generate answer: {}",
                    e
                ),
            )
        }
    }
}

/// Convenience function that runs the complete 2-step RAG flow
///
/// This is the main entry point for RAG-based chat responses.
/// It extracts the query, executes the search, and generates the answer.
///
/// # Arguments
/// * `user_message` - The user's natural language question
/// * `model` - Optional model override
/// * `search_fn` - Closure that executes the database search
///
/// # Returns
/// * `Ok(RagResult)` - The generated answer with source movies
/// * `Err(String)` - Error message if any step fails
pub async fn run_rag_flow(
    user_message: &str,
    model: Option<&str>,
    search_fn: impl FnOnce(
        &StructuredQuery,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Vec<FullMovie>, String>> + Send>,
    >,
) -> Result<RagResult, String> {
    // Step 1: Extract query
    let extracted = extract_rag_query(
        user_message,
        model,
    )
    .await
    .ok_or_else(|| "Failed to extract search query".to_string())?;

    if !extracted.is_movie_query {
        return Err("Not a movie query".to_string());
    }

    // Step 2: Execute search
    let movies = execute_rag_search(
        &extracted.structured_query,
        search_fn,
    )
    .await?;

    // Step 3: Generate answer
    let answer = generate_rag_answer(
        user_message,
        &movies,
        model,
    )
    .await?;

    Ok(
        RagResult {
            answer,
            source_movies: movies,
            structured_query: extracted.structured_query,
        },
    )
}

/// Format movies as a readable list for context injection
///
/// This is a lightweight alternative to the full AI answer generation
/// when you just want to format the movie list for a system prompt.
pub fn format_movies_for_context(movies: &[FullMovie]) -> String {
    if movies.is_empty() {
        return "\n\nNo movies found matching the query.".to_string();
    }

    let movie_list = movies
        .iter()
        .take(15)
        .map(
            |m| {
                format!(
                    "- {} ({}) - Directed by {}, Starring: {}",
                    m.name,
                    m.release_year,
                    m.director
                        .as_ref()
                        .map(
                            |d| d
                                .name
                                .as_str()
                        )
                        .unwrap_or("Unknown"),
                    m.actors
                        .iter()
                        .map(
                            |a| a
                                .name
                                .as_str()
                        )
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            },
        )
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "\n\nAvailable movies matching your query:\n{}\n\nImportant: Answer based ONLY on these movies. Do not mention movies not in this list.",
        movie_list
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_movies_for_context_empty() {
        let movies: Vec<FullMovie> = vec![];
        let result = format_movies_for_context(&movies);
        assert!(result.contains("No movies found"));
    }

    #[tokio::test]
    async fn test_generate_rag_answer_empty_movies() {
        let result = generate_rag_answer(
            "test question",
            &[],
            None,
        )
        .await;
        assert!(result.is_ok());
        assert!(
            result
                .unwrap()
                .contains("couldn't find any movies")
        );
    }
}
