use std::sync::Arc;

use ai_chat::OllamaClient;
use axum::{extract::Path, Json};
use axum_macros::debug_handler;
use database::{question::AiAction, FullMovie, SearchRequest};
use log::{error, info, warn};
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

// #[instrument]
// #[debug_handler]
// pub async fn chat(Json(action): Json<AiAction>) -> Json<AiAction> {
//     let dvds = database::get_movies()
//         .await
//         .unwrap_or_else(|_| Vec::new());

//     let (uuid, question, model, temperature) = (
//         action.uuid,
//         action.action,
//         action.model,
//         action.temperature
//     );

//     let embedding = embedding(&question)
//         .await
//         .ok();

//     // Search for movies using the embedding if available
//     let movie_ids = match embedding {
//         Some(embedding_vector) => {
//             info!("Searching for movies using embedding");
//             let search_result = database::search_movies(
//                 embedding_vector,
//                 15,
//             )
//             .await;

//             if let Err(e) = &search_result {
//                 error!(
//                     "Failed to search movies: {:#?}",
//                     e
//                 );
//             }

//             search_result.ok()
//         }
//         None => None,
//     };

//     let full_movies = if let Some(movie_ids) = movie_ids {
//         database::get_movies_by_ids(movie_ids)
//             .await
//             .unwrap_or(dvds)
//     } else {
//         dvds
//     }; // For now fall back to all dvds

//     info!(
//         "Found {} matching movies",
//         full_movies.len()
//     );

//     info!("Sending question to AI.");
//     let result = ai_chat::ai_message(
//         Arc::new(full_movies),
//         Arc::new(
//             OllamaClient {
//                 ollama_client: {
//                     let ollama_host =
//                         std::env::var("OLLAMA_HOST").unwrap_or_else(|_| "ollama".to_string());
//                     let ollama_port =
//                         std::env::var("OLLAMA_PORT").unwrap_or_else(|_| "11434".to_string());
//                     let ollama_url = format!(
//                         "http://{}:{}",
//                         ollama_host, ollama_port
//                     );
//                     Ollama::from_url(
//                         ollama_url
//                             .parse()
//                             .unwrap(),
//                     )
//                 },
//                 ai_action: AiAction {
//                     uuid: uuid.to_string(),
//                     action: question,
//                     model: model.clone(),
//                     temperature: temperature,
//                 },
//             },
//         ),
//     )
//     .await;
//     info!("Response received.");

//     let response = match result {
//         Ok(response) => response,
//         Err(e) => {
//             error!(
//                 "Failed to get AI response: {:#?}",
//                 e
//             );
//             "There was an error that made communication with the AI impossible.".to_string()
//         }
//     };

//     Json(
//         AiAction {
//             uuid,
//             action: response,
//             model,
//             temperature: None,
//         },
//     )
// }

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

/// Helper function to search for movies using vector embeddings
async fn search_movies_by_embedding(query: &str) -> Result<Vec<FullMovie>, String> {
    let embedding = embedding(query).await
        .map_err(|e| format!("Failed to generate embedding: {}", e))?;
    
    info!("Searching for movies using embedding");
    let movie_ids = database::search_movies(embedding, 15).await
        .map_err(|e| {
            error!("Failed to search movies: {:#?}", e);
            format!("Database search failed: {}", e)
        })?;
    
    Ok(movie_ids)
}

/// Helper function to create Ollama client with configuration
fn create_ollama_client(query: String) -> Result<Arc<OllamaClient>, String> {
    let ollama_host = std::env::var("OLLAMA_HOST")
        .unwrap_or_else(|_| "ollama".to_string());
    let ollama_port = std::env::var("OLLAMA_PORT")
        .unwrap_or_else(|_| "11434".to_string());
    
    let ollama_url = format!("http://{}:{}", ollama_host, ollama_port);
    let parsed_url = ollama_url.parse()
        .map_err(|e| format!("Invalid Ollama URL {}: {}", ollama_url, e))?;
    
    let client = Arc::new(OllamaClient {
        ollama_client: Ollama::from_url(parsed_url),
        ai_action: AiAction {
            uuid: "movie_search".to_string(),
            action: query,
            model: Some("phi3.5".to_string()),
            temperature: None,
        },
    });
    
    Ok(client)
}

/// Helper function to parse AI response into movie IDs
fn parse_movie_ids_from_response(response: String) -> Result<Vec<i32>, String> {
    let ids: Vec<i32> = response
        .split(",")
        .filter_map(|id| {
            let trimmed = id.trim();
            match trimmed.parse::<i32>() {
                Ok(parsed_id) if parsed_id > 0 => Some(parsed_id),
                Ok(_) => {
                    warn!("Ignoring invalid movie ID: {}", trimmed);
                    None
                }
                Err(e) => {
                    warn!("Failed to parse movie ID '{}': {}", trimmed, e);
                    None
                }
            }
        })
        .collect();
    
    if ids.is_empty() {
        return Err("No valid movie IDs found in AI response".to_string());
    }
    
    Ok(ids)
}

/// Main endpoint for getting matching movies using AI
#[instrument]
#[debug_handler]
pub async fn get_matching_movies(Json(search_request): Json<SearchRequest>) -> Json<Option<Vec<FullMovie>>> {
    // Step 1: Search for candidate movies using vector embeddings
    let movie_result = search_movies_by_embedding(&search_request.query).await;

    let move_result = match movie_result {
        Ok(movies) => movies,
        Err(e) => {
            error!("Failed to fetch candidate movies: {:#?}", e);
            return Json(None);
        }
    };

    Json(Some(move_result))
    
    
    // // Step 2: Get full movie details for candidates
    // let candidate_movies = match database::get_movies_by_ids(candidate_movie_ids).await {
    //     Ok(movies) => movies,
    //     Err(e) => {
    //         error!("Failed to fetch candidate movies: {:#?}", e);
    //         return Json(None);
    //     }
    // };
    
    // if candidate_movies.is_empty() {
    //     info!("No candidate movies found for query: {}", search_request.query);
    //     return Json(Some(Vec::new()));
    // }
    
    // // Step 3: Create Ollama client for AI processing
    // let ollama_client = match create_ollama_client(search_request.query.clone()) {
    //     Ok(client) => client,
    //     Err(e) => {
    //         error!("Failed to create Ollama client: {}", e);
    //         return Json(None);
    //     }
    // };
    
    // // Step 4: Use AI to refine the movie selection
    // let ai_response = match ai_chat::live_ui::get_matching_movies_with_ollama(
    //     Arc::new(candidate_movies),
    //     ollama_client,
    // ).await {
    //     Ok(response) => response,
    //     Err(e) => {
    //         error!("AI processing failed: {}", e);
    //         return Json(None);
    //     }
    // };
    
    // // Step 5: Parse AI response to get final movie IDs
    // let final_movie_ids = match parse_movie_ids_from_response(ai_response) {
    //     Ok(ids) => ids,
    //     Err(e) => {
    //         error!("Failed to parse AI response: {}", e);
    //         return Json(None);
    //     }
    // };
    
    // // Step 6: Get final movie details
    // let final_movies = match database::get_movies_by_ids(final_movie_ids).await {
    //     Ok(movies) => Some(movies),
    //     Err(e) => {
    //         error!("Failed to fetch final movies: {:#?}", e);
    //         None
    //     }
    // };
    
    // Json(final_movies)
}
