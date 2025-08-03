use std::sync::Arc;

use ai_chat::OllamaClient;
use axum::{extract::Path, Json};
use axum_macros::debug_handler;
use database::{question::AiAction, FullMovie};
use log::{debug, error, info};
use ollama_rs::Ollama;
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

    // Don't delete. should be extracted into another option for the user to select. Vector search, key matching
    // let movie_ids = match ai_chat::find_related_keys(question.as_str()).await {
    //     Ok(dvd_filters) => database::run_dvd_filters(dvd_filters)
    //         .await
    //         .ok()
    //         .filter(|movies| !movies.is_empty()),
    //     Err(e) => {
    //         error!(
    //             "{:#?}",
    //             e
    //         );
    //         None
    //     }
    // };

    info!(
        "Getting embeddings for user query {}",
        &question
    );
    // Get embedding for the user's question
    let embedding_result = ai_chat::get_embedding(&question).await;
    info!("Got embeddings: {}", embedding_result.is_ok());

    if let Err(e) = &embedding_result {
        error!("Failed to get embeddings: {:#?}", e);
    }
    
    let embedding = embedding_result.ok();
    
    // Search for movies using the embedding if available
    let movie_ids = match embedding {
        Some(embedding_vector) => {
            info!("Searching for movies using embedding");
            let search_result = database::search_movies(embedding_vector, 15).await;

            if let Err(e) = &search_result {
                error!("Failed to search movies: {:#?}", e);
            }

            search_result.ok()
        },
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
                    let ollama_host = std::env::var("OLLAMA_HOST").unwrap_or_else(|_| "ollama".to_string());
                    let ollama_port = std::env::var("OLLAMA_PORT").unwrap_or_else(|_| "11434".to_string());
                    let ollama_url = format!("http://{}:{}", ollama_host, ollama_port);
                    Ollama::from_url(ollama_url.parse().unwrap())
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
