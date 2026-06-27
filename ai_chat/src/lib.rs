//! AI Chat Integration Module
//!
//! This crate provides integration with Ollama for AI-powered features including:
//! - Chat conversations about the movie collection
//! - Vector embeddings generation for semantic search
//! - Query enhancement for better search results
//! - Structured query parsing from natural language
//!
//! # Features
//!
//! - **Embeddings**: Generate vector embeddings using `nomic-embed-text` model
//! - **Chat**: Conversational AI for answering questions about movies
//! - **Query Enhancement**: Expand short queries for better semantic search
//! - **Structured Parsing**: Convert natural language to structured search criteria
//!
//! # Example
//!
//! ```no_run
//! use ai_chat::{get_embedding, list_models};
//!
//! #[tokio::main]
//! async fn main() {
//!     // Generate embeddings for a text
//!     let embedding = get_embedding("science fiction movies").await.unwrap();
//!     
//!     // List available models
//!     let models = list_models().await.unwrap();
//! }
//! ```

pub mod embedding;
pub mod live_ui;
pub mod query_enhancement;
pub mod rag;
pub mod schema;
pub mod structured_query_parser;
#[cfg(feature = "internet")]
pub mod web_scraper;

// Re-export all prompts from the prompts crate
pub use prompts::*;

use std::sync::Arc;

use log::{debug, error, info};
use models::{dvd_filters::DvdFilters, question::AiAction, AiMovieData, FullMovie};
use ollama_rs::{
    generation::{
        chat::{request::ChatMessageRequest, ChatMessage},
        embeddings::request::GenerateEmbeddingsRequest,
    },
    models::ModelOptions,
    Ollama,
};
use tokio_stream::StreamExt;
use tracing::{instrument, Level};

pub use embedding::*;
pub use query_enhancement::*;

// Re-export the embedding model constant for easy access
pub use embedding::EMBEDDING_MODEL;

// AI Model constants
pub const DEFAULT_CHAT_MODEL: &str = "qwen2.5:7b";
const DEFAULT_SMALL_MODEL: &str = DEFAULT_CHAT_MODEL;
const DEFAULT_OLLAMA_HOST: &str = "ollama";
const DEFAULT_OLLAMA_PORT: &str = "11434";
const DEFAULT_CONTEXT_WINDOW: u64 = 64000;

/// Builds an Ollama client from `OLLAMA_HOST` / `OLLAMA_PORT` env vars.
/// Falls back to `ollama:11434` if the variables are not set.
///
/// # Errors
/// Returns `Err` if the constructed URL is not a valid HTTP URL.
pub fn make_ollama_client() -> Result<Ollama, String> {
    let host = std::env::var("OLLAMA_HOST").unwrap_or_else(|_| DEFAULT_OLLAMA_HOST.to_string());
    let port = std::env::var("OLLAMA_PORT").unwrap_or_else(|_| DEFAULT_OLLAMA_PORT.to_string());
    let url = format!(
        "http://{}:{}",
        host, port
    );
    url.parse()
        .map(Ollama::from_url)
        .map_err(
            |e| {
                format!(
                    "Failed to parse Ollama URL '{}': {}",
                    url, e
                )
            },
        )
}

/// Client for interacting with Ollama AI services
///
/// Wraps the Ollama client with additional context about the AI action being performed.
#[derive(Debug)]
pub struct OllamaClient {
    pub ollama_client: Ollama,
    pub ai_action: AiAction,
}

/// Generates an AI response about the movie collection
///
/// # Arguments
///
/// * `dvds` - Arc-wrapped vector of movies to query about
/// * `ollama` - Ollama client with AI action context
///
/// # Returns
///
/// * `Ok(String)` - AI-generated response
/// * `Err(String)` - Error message if AI response generation fails
#[instrument(level = Level::DEBUG)]
pub async fn ai_message(
    dvds: Arc<Vec<FullMovie>>,
    ollama: Arc<OllamaClient>,
) -> Result<String, String> {
    bot_message(
        &dvds,
        Arc::clone(&ollama),
    )
    .await
    .or_else(|_| Ok(String::from("There was a problem creating an AI response to the question")))
}

/// Extracts structured search filters from a natural language question
///
/// Uses AI to parse the question and identify relevant actors, directors, genres, and other
/// search criteria that can be used to filter the movie collection.
///
/// # Arguments
///
/// * `question` - Natural language question about movies
///
/// # Returns
///
/// * `Ok(DvdFilters)` - Structured filters extracted from the question
/// * `Err(String)` - Error message if parsing fails
#[instrument(level = Level::DEBUG)]
pub async fn find_related_keys(
    question: impl AsRef<str> + std::fmt::Debug,
) -> Result<DvdFilters, String> {
    let ollama = Ollama::default();
    let question_message = format!(
        "User question: {}",
        question.as_ref()
    );

    let messages = vec![
        ChatMessage::system(prompts::DATA_POINTS_PROMPT.to_string()),
        ChatMessage::user(question_message),
    ];

    let chat_request = ChatMessageRequest::new(
        DEFAULT_SMALL_MODEL.to_string(),
        messages,
    )
    .format(schema::dvd_filters_schema());

    let response = ollama
        .send_chat_messages(chat_request)
        .await
        .map_err(|e| e.to_string())?;

    info!(
        "Response from datapoints AI: {}",
        response
            .message
            .content
    );

    serde_json::from_str(
        &response
            .message
            .content,
    )
    .map_err(|e| e.to_string())
}

async fn bot_message(dvds: &[FullMovie], ollama: Arc<OllamaClient>) -> Result<String, String> {
    let prompt = USER_LIBRARY_PROMPT.replace(
        "{USER_MOVIE_LIBRARY}",
        &serde_json::to_string(&dvds).unwrap_or_default(),
    );

    let messages = vec![
        ChatMessage::system(prompt),
        ChatMessage::user(
            ollama
                .ai_action
                .action
                .to_string(),
        ),
    ];

    let model = ollama
        .ai_action
        .model
        .as_ref()
        .map(|m| m.to_string())
        .unwrap_or_else(|| DEFAULT_CHAT_MODEL.to_string());

    let chat_request = ChatMessageRequest::new(
        model, messages,
    )
    .options(ModelOptions::default().num_ctx(DEFAULT_CONTEXT_WINDOW));

    let response = ollama
        .ollama_client
        .send_chat_messages(chat_request)
        .await
        .map_err(|e| e.to_string())?;

    info!(
        "Response from AI: {}",
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

#[instrument(level = Level::INFO)]
pub async fn get_embedding(text: &str) -> Result<Vec<f32>, ollama_rs::error::OllamaError> {
    debug!(
        "Generating embedding for text: {}",
        text
    );

    let ollama = make_ollama_client().map_err(ollama_rs::error::OllamaError::Other)?;

    let request = GenerateEmbeddingsRequest::new(
        EMBEDDING_MODEL.to_string(),
        text.into(),
    )
    .options(ModelOptions::default().num_ctx(8192));

    let response = ollama
        .generate_embeddings(request)
        .await?
        .embeddings
        .into_iter()
        .flatten()
        .collect();

    Ok(response)
}

/// Lists all available Ollama models
///
/// # Returns
///
/// * `Ok(Vec<models::AvailableModel>)` - List of available models with their names and sizes
/// * `Err(String)` - Error message if listing fails
#[instrument(level = Level::INFO)]
pub async fn list_models() -> Result<Vec<models::AvailableModel>, String> {
    info!("Fetching available Ollama models");

    let ollama = make_ollama_client().map_err(
        |e| {
            let msg = format!(
                "Ollama client error: {}",
                e
            );
            log::error!("{}", msg);
            msg
        },
    )?;

    match ollama
        .list_local_models()
        .await
    {
        Ok(models_list) => {
            let available_models: Vec<models::AvailableModel> = models_list
                .into_iter()
                .map(
                    |model| models::AvailableModel {
                        name: model.name,
                        size: model.size,
                    },
                )
                .collect();

            info!(
                "Found {} available models",
                available_models.len()
            );
            Ok(available_models)
        }
        Err(e) => {
            let error_msg = format!(
                "Failed to list models: {:?}",
                e
            );
            log::error!("{}", error_msg);
            Err(error_msg)
        }
    }
}

/// Pulls an Ollama model to make it available for use
///
/// # Arguments
///
/// * `model_name` - The name of the model to pull (e.g., "qwen2.5:7b", "nomic-embed-text")
///
/// # Returns
///
/// * `Ok(String)` - Success message with model name
/// * `Err(String)` - Error message if pull fails
#[instrument(level = Level::INFO)]
pub async fn pull_model(model_name: &str) -> Result<String, String> {
    info!(
        "Attempting to pull Ollama model: {}",
        model_name
    );

    let ollama = make_ollama_client().map_err(
        |e| {
            let msg = format!(
                "Ollama client error: {}",
                e
            );
            log::error!("{}", msg);
            msg
        },
    )?;

    let mut stream = match ollama
        .pull_model_stream(
            model_name.to_string(),
            false,
        )
        .await
    {
        Ok(s) => s,
        Err(e) => {
            let error_msg = format!(
                "Failed to start pull for model {}: {:?}",
                model_name, e
            );
            log::error!("{}", error_msg);
            return Err(error_msg);
        }
    };

    let mut last_status = String::new();
    while let Some(status) = stream.next().await {
        match status {
            Ok(s) => {
                info!("Pull status for {}: {}", model_name, s.message);
                last_status = s.message;
            }
            Err(e) => {
                let error_msg = format!(
                    "Error while pulling model {}: {:?}",
                    model_name, e
                );
                log::error!("{}", error_msg);
                return Err(error_msg);
            }
        }
    }

    info!("Successfully pulled model: {}", model_name);
    Ok(format!("Successfully pulled model: {} ({})", model_name, last_status))
}


/// Correct a batch of movie titles in a single Ollama call.
/// Returns one corrected title per input, in the same order.
/// If a title is already correct, the model returns it unchanged.
#[instrument(level = Level::DEBUG, skip(titles, model), fields(title_count = titles.len()))]
pub async fn correct_movie_titles(
    titles: &[String],
    model: Option<&str>,
) -> Result<Vec<String>, String> {
    debug!(
        "Starting title correction for {} titles: {:?}",
        titles.len(),
        titles
    );

    let ollama = make_ollama_client().map_err(
        |e| {
            error!(
                "{}",
                e
            );
            e
        },
    )?;

    let numbered = titles
        .iter()
        .enumerate()
        .map(
            |(i, t)| {
                format!(
                    "{}. {}",
                    i + 1,
                    t
                )
            },
        )
        .collect::<Vec<_>>()
        .join("\n");

    let prompt = format!(
        "You are a movie title normalizer. For each input, return the exact canonical title of the well-known movie or TV show it refers to.\n\
Rules:\n\
- Fix spelling errors\n\
- Restore missing words (articles like \"The\", \"A\", trailing words)\n\
- Fix capitalization\n\
- Return the full official release title as it appears in movie databases\n\
- If the input does not resemble any known title, return it UNCHANGED\n\
- Do NOT add, remove, or reorder entries\n\
- Respond with ONLY a JSON array of strings, one per input, in the same order\n\n\
Examples:\n\
Input:  [\"Gaurdians of the Galaxy\", \"Matrix Reloaded\", \"Dark Kight\", \"Shawsank Redemption\", \"Termiantor 2\", \"Happy Glmore\", \"Naked GUn\", \"Matrix Revolution\", \"Figt Club\", \"Shindlers List\"]\n\
Output: [\"Guardians of the Galaxy\", \"The Matrix Reloaded\", \"The Dark Knight\", \"The Shawshank Redemption\", \"Terminator 2: Judgment Day\", \"Happy Gilmore\", \"The Naked Gun\", \"The Matrix Revolutions\", \"Fight Club\", \"Schindler's List\"]\n\n\
Input titles:\n{}\n\nJSON array only, no explanation:",
        numbered
    );

    debug!(
        "Sending title correction prompt to AI (model: {:?})",
        model
    );

    let request = ChatMessageRequest::new(
        model
            .unwrap_or(DEFAULT_CHAT_MODEL)
            .to_string(),
        vec![ChatMessage::user(prompt)],
    );

    match ollama
        .send_chat_messages(request)
        .await
    {
        Ok(response) => {
            let content = response
                .message
                .content
                .trim();
            debug!(
                "Raw AI response for title correction: '{}'",
                content
            );

            // Strip markdown code fences if present
            let json = content
                .trim_start_matches("```json")
                .trim_start_matches("```")
                .trim_end_matches("```")
                .trim();

            debug!(
                "Stripped JSON for parsing: '{}'",
                json
            );

            match serde_json::from_str::<Vec<String>>(json) {
                Ok(corrected) => {
                    debug!(
                        "Successfully parsed {} corrected titles: {:?}",
                        corrected.len(),
                        corrected
                    );

                    // Log each individual correction for detailed debugging
                    for (i, (orig, corr)) in titles
                        .iter()
                        .zip(corrected.iter())
                        .enumerate()
                    {
                        if orig != corr {
                            debug!(
                                "Title {}: '{}' -> '{}' (CHANGED)",
                                i, orig, corr
                            );
                        } else {
                            debug!(
                                "Title {}: '{}' (unchanged)",
                                i, orig
                            );
                        }
                    }

                    if corrected.len() != titles.len() {
                        error!(
                            "Title correction count mismatch: sent {}, received {}",
                            titles.len(),
                            corrected.len()
                        );
                        return Err(
                            format!(
                                "Response count mismatch: expected {} titles, got {}",
                                titles.len(),
                                corrected.len()
                            ),
                        );
                    }

                    Ok(corrected)
                }
                Err(e) => {
                    error!(
                        "Failed to parse title corrections JSON: {}. Raw content: '{}'",
                        e, json
                    );
                    Err(
                        format!(
                            "Failed to parse title corrections: {}. Content: {}",
                            e, json
                        ),
                    )
                }
            }
        }
        Err(e) => {
            error!(
                "Failed to get AI response for title correction: {}",
                e
            );
            Err(
                format!(
                    "Failed to get AI response: {}",
                    e
                ),
            )
        }
    }
}

/// Generate data for a single movie title using structured Ollama output.
/// Used for streaming generation where we call Ollama once per movie.
#[instrument(level = Level::INFO)]
pub async fn generate_movie_single(
    title: &str,
    model: Option<&str>,
) -> Result<AiMovieData, String> {
    let ollama = make_ollama_client().map_err(
        |e| {
            error!(
                "{}",
                e
            );
            e
        },
    )?;

    let system_prompt = r#"You are a movie database assistant. Generate complete movie information for the title provided.

Provide:
- title: The exact movie title
- year: Release year as integer (0 if unknown)
- description: A comprehensive plot description (4-5 sentences). Focus on the story, themes, and key plot points. DO NOT mention actors or cast members in the description - we have a separate field for that.
- actors: Top 6 billed actors as an array of strings
- genres: Array of genres in order of relevance (primary genre first)
- director: Director name (empty string for TV shows or if unknown)

CRITICAL: Respond with a single valid JSON object matching the exact schema."#;

    let request = ChatMessageRequest::new(
        model
            .unwrap_or(DEFAULT_CHAT_MODEL)
            .to_string(),
        vec![
            ChatMessage::system(system_prompt.to_string()),
            ChatMessage::user(
                format!(
                    "Generate movie data for: {}",
                    title
                ),
            ),
        ],
    )
    .format(crate::schema::ai_movie_data_schema());

    match ollama
        .send_chat_messages(request)
        .await
    {
        Ok(response) => {
            let content = response
                .message
                .content
                .trim();
            serde_json::from_str::<AiMovieData>(content).map_err(
                |e| {
                    format!(
                        "Failed to parse JSON: {}. Content: {}",
                        e, content
                    )
                },
            )
        }
        Err(e) => Err(
            format!(
                "Failed to get AI response: {}",
                e
            ),
        ),
    }
}

/// Scrape Wikipedia context for a single title. Returns the raw context string.
/// Separated from the Ollama call so callers can pipeline scraping alongside inference.
#[cfg(feature = "internet")]
pub async fn scrape_single(title: &str) -> String {
    let scraped = web_scraper::scrape_movie_contexts(&[title.to_string()]).await;
    scraped
        .into_iter()
        .next()
        .filter(
            |ctx| {
                !ctx.context
                    .is_empty()
            },
        )
        .map(|ctx| ctx.context)
        .unwrap_or_else(|| "No web context found. Use your best knowledge.".to_string())
}

/// Run Ollama inference for a single title with a pre-scraped RAG context string.
/// Use this together with `scrape_single` to pipeline scraping alongside inference.
#[cfg(feature = "internet")]
#[instrument(level = Level::INFO)]
pub async fn generate_movie_with_context(
    title: &str,
    rag_context: String,
    model: Option<&str>,
) -> Result<AiMovieData, String> {
    let ollama = make_ollama_client().map_err(
        |e| {
            error!(
                "{}",
                e
            );
            e
        },
    )?;

    let system_prompt = format!(
        r#"You are a movie database assistant. Generate complete movie information for the title provided.

IMPORTANT: Use the Wikipedia reference data below for accuracy.

=== WIKIPEDIA REFERENCE DATA ===
{}
=== END REFERENCE DATA ===

Provide:
- title: The exact movie title
- year: Release year as integer (0 if unknown)
- description: A comprehensive plot description (4-5 sentences) using the reference data. Focus on the story, themes, and key plot points. DO NOT mention actors or cast members in the description - we have a separate field for that.
- actors: Top 6 billed actors from the reference data
- genres: Array of genres in order of relevance
- director: Director name (empty string if unknown)

CRITICAL: Respond with a single valid JSON object matching the exact schema."#,
        rag_context
    );

    let request = ChatMessageRequest::new(
        model
            .unwrap_or(DEFAULT_CHAT_MODEL)
            .to_string(),
        vec![
            ChatMessage::system(system_prompt),
            ChatMessage::user(
                format!(
                    "Generate movie data for: {}",
                    title
                ),
            ),
        ],
    )
    .format(crate::schema::ai_movie_data_schema());

    match ollama
        .send_chat_messages(request)
        .await
    {
        Ok(response) => {
            let content = response
                .message
                .content
                .trim();
            serde_json::from_str::<AiMovieData>(content).map_err(
                |e| {
                    format!(
                        "Failed to parse JSON: {}. Content: {}",
                        e, content
                    )
                },
            )
        }
        Err(e) => Err(
            format!(
                "Failed to get AI response: {}",
                e
            ),
        ),
    }
}


#[cfg(test)]
mod tests {}
