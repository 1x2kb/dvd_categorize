use std::sync::Arc;

use ai_chat::OllamaClient;
use axum::{extract::Path, Json};
use axum_macros::debug_handler;
use database::{question::AiAction, FullMovie, SearchRequest};
use log::{error, info};
use ollama_rs::{error::OllamaError, Ollama};
use tracing::instrument;

#[instrument]
pub async fn hello_world() -> &'static str {
    "Hello from DVD_CATALOG_API!"
}

#[instrument]
#[debug_handler]
pub async fn get_dvds() -> Json<Option<Vec<FullMovie>>> {
    Json(
        database::get_movies()
            .await
            .ok(),
    )
}

#[instrument]
#[debug_handler]
pub async fn get_dvd(Path(id): Path<i32>) -> Json<Option<FullMovie>> {
    Json(
        database::get_movie(id)
            .await
            .ok(),
    )
}

#[instrument]
#[debug_handler]
pub async fn insert_dvd(Json(dvd): Json<FullMovie>) -> Json<Option<FullMovie>> {
    Json(
        database::insert_full_movie(dvd)
            .await
            .ok(),
    )
}

#[instrument]
#[debug_handler]
pub async fn chat(Json(action): Json<AiAction>) -> Json<AiAction> {
    let dvds = database::get_movies()
        .await
        .unwrap_or_else(|_| Vec::new());

    let (uuid, question, model) = (
        action.uuid,
        action.action,
        action.model,
    );

    let embedding = embedding(&question)
        .await
        .ok();

    // Search for movies using the embedding if available
    let movie_ids = match embedding {
        Some(embedding_vector) => {
            info!("Searching for movies using embedding");
            let search_result = database::search_movies(
                embedding_vector,
                15,
            )
            .await;

            if let Err(e) = &search_result {
                error!(
                    "Failed to search movies: {:#?}",
                    e
                );
            }

            search_result.ok()
        }
        None => None,
    };

    let full_movies = if let Some(movie_ids) = movie_ids {
        database::get_movies_by_ids(movie_ids)
            .await
            .unwrap_or(dvds)
    } else {
        dvds
    }; // For now fall back to all dvds

    info!(
        "Found {} matching movies",
        full_movies.len()
    );

    info!("Sending question to AI.");
    let result = ai_chat::ai_message(
        Arc::new(full_movies),
        Arc::new(
            OllamaClient {
                ollama_client: {
                    let ollama_host =
                        std::env::var("OLLAMA_HOST").unwrap_or_else(|_| "ollama".to_string());
                    let ollama_port =
                        std::env::var("OLLAMA_PORT").unwrap_or_else(|_| "11434".to_string());
                    let ollama_url = format!(
                        "http://{}:{}",
                        ollama_host, ollama_port
                    );
                    Ollama::from_url(
                        ollama_url
                            .parse()
                            .unwrap(),
                    )
                },
                ai_action: AiAction {
                    uuid: uuid.to_string(),
                    action: question,
                    model: model.clone(),
                },
            },
        ),
    )
    .await;
    info!("Response received.");

    let response = match result {
        Ok(response) => response,
        Err(e) => {
            error!(
                "Failed to get AI response: {:#?}",
                e
            );
            "There was an error that made communication with the AI impossible.".to_string()
        }
    };

    Json(
        AiAction {
            uuid,
            action: response,
            model,
        },
    )
}

/// Generates vector embeddings for a given text question using the Ollama AI service.
///
/// This function takes text input and converts it into a numerical vector representation
/// (embeddings) that can be used for semantic similarity comparisons and vector database
/// operations. The embeddings are generated asynchronously using the Ollama client.
///
/// # Arguments
///
/// * `text` - A string slice containing the text to generate embeddings for
///
/// # Returns
///
/// * `Ok(Vec<f32>)` - A vector of floating-point numbers representing the text embeddings
/// * `Err(OllamaError)` - An error if the embedding generation fails
///
/// # Errors
///
/// This function will return an error if:
/// - The Ollama service is unavailable
/// - Network connectivity issues occur
/// - The input text cannot be processed by the embedding model
/// - Model is not installed
#[instrument]
pub async fn embedding(text: &str) -> Result<Vec<f32>, OllamaError> {
    info!(
        "Getting embeddings for user query {}",
        text
    );
    // Get embedding for the user's question
    let embedding_result = ai_chat::get_embedding(&text).await;
    info!(
        "Got embeddings: {}",
        embedding_result.is_ok()
    );

    if let Ok(embedding) = embedding_result.as_ref() {
        info!(
            "Embeddings length: {}",
            embedding.len()
        );
    } else if let Err(e) = &embedding_result {
        error!(
            "Failed to get embeddings: {:#?}",
            e
        );
    };

    embedding_result
}

#[instrument]
#[debug_handler]
pub async fn get_matching_movies(Json(search_request): Json<SearchRequest>) -> Json<Option<Vec<FullMovie>>> {
    // let dvds = database::get_movies()
    //     .await
    //     .unwrap_or_else(|_| Vec::new());

    let embedding = embedding(&search_request.query)
        .await
        .ok();

    // Search for movies using the embedding if available
    // TODO: Refactor search_movies to return results for the movie. Why am I only returning the id????
    let movie_ids = match embedding {
        Some(embedding_vector) => {
            info!("Searching for movies using embedding");
            let search_result = database::search_movies(
                embedding_vector,
                15,
            )
            .await;

            if let Err(e) = &search_result {
                error!(
                    "Failed to search movies: {:#?}",
                    e
                );
            }

            search_result.ok()
        }
        None => None,
    };

    let full_movies = if let Some(movie_ids) = movie_ids {
        database::get_movies_by_ids(movie_ids)
            .await
            .unwrap_or(Vec::new())
    } else {
        Vec::new()
    }; // For now fall back to all dvds

    let ollama_client = Arc::new(
        OllamaClient {
            ollama_client: {
                let ollama_host =
                    std::env::var("OLLAMA_HOST").unwrap_or_else(|_| "ollama".to_string());
                let ollama_port =
                    std::env::var("OLLAMA_PORT").unwrap_or_else(|_| "11434".to_string());
                let ollama_url = format!(
                    "http://{}:{}",
                    ollama_host, ollama_port
                );
                Ollama::from_url(
                    ollama_url
                        .parse()
                        .unwrap(),
                )
            },
            ai_action: AiAction {
                uuid: "Some placeholder".to_string(),
                action: search_request.query,
                model: Some("mistral".to_string()),
            },
        },
    );

    let result = ai_chat::live_ui::get_matching_movies_with_ollama(
        Arc::new(&full_movies),
        ollama_client,
    )
    .await;

    let movie_ids = result.map(
        |value| {
            value
                .split(",")
                .map(
                    |id| {
                        id.trim()
                            .parse::<i32>()
                            .unwrap_or_default()
                    },
                )
                .filter(|id| id > &0)
                .collect::<Vec<i32>>()
        },
    );

    let full_movies = match movie_ids {
        Ok(ids) => database::get_movies_by_ids(ids)
            .await
            .ok(),
        Err(e) => {
            error!(
                "Failed to get movies by ids: {:#?}",
                e
            );
            None
        }
    };

    Json(full_movies)
}
