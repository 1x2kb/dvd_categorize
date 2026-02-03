use ai_chat::OllamaClient;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use axum_macros::debug_handler;
use database::{question::AiAction, FullMovie, SearchRequest};
use log::{error, info, warn};
use models::ScoredMovie;
use models::{CsvInput, TextMatchScoring};
use ollama_rs::{error::OllamaError, Ollama};
use std::{sync::Arc, time::Instant};
use tracing::instrument;

// Movie search functionality module
pub mod movie_search;
pub use movie_search::extract_entities;

#[derive(Clone, Debug)]
pub struct CacheState {
    pub movies: Arc<tokio::sync::RwLock<Vec<FullMovie>>>,
}

#[instrument]
pub async fn hello_world() -> &'static str {
    "Hello from DVD_CATALOG_API!"
}

#[instrument(skip(state))]
#[debug_handler]
pub async fn get_dvds(State(state): State<CacheState>) -> Json<Option<Vec<FullMovie>>> {
    let movies = state
        .movies
        .read()
        .await;
    if movies.is_empty() {
        Json(None)
    } else {
        Json(Some((*movies).clone()))
    }
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
pub async fn chat() -> impl axum::response::IntoResponse {
    (
        axum::http::StatusCode::NOT_FOUND,
        "Chat endpoint temporarily disabled",
    )
}

#[instrument]
#[debug_handler]
pub async fn export_csv(State(cache_state): State<CacheState>) -> impl axum::response::IntoResponse {
    let movies = {
        let movies_guard = cache_state
            .movies
            .read()
            .await;
        (*movies_guard).clone()
    };

    match csv_utils::movies_to_csv(&movies) {
        Ok(csv) => {
            let headers = "Title,Description,Actors,Genres,Director\n";
            let csv_with_headers = format!("{}{}", headers, csv);
            (
                StatusCode::OK,
                [("Content-Type", "text/csv"), ("Content-Disposition", "attachment; filename=movies.csv")],
                csv_with_headers,
            )
        }
        Err(e) => {
            error!("Failed to export CSV: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                [("Content-Type", "text/plain"), ("Content-Disposition", "")],
                format!("Failed to export CSV: {}", e),
            )
        }
    }
}

#[instrument]
#[debug_handler]
pub async fn preview_csv(Json(value): Json<CsvInput>) -> impl axum::response::IntoResponse {
    let headers = ["Title", "Description", "Actors", "Genres", "Director"];
    let csv_with_headers = format!(
        "{}\n{}",
        headers.join(","),
        value.input
    );

    let movies = csv_utils::parse_csv(csv_with_headers.as_bytes()).unwrap_or_else(
        |e| {
            error!(
                "{}",
                e
            );
            Vec::new()
        },
    );

    (
        StatusCode::OK,
        Json(movies),
    )
}

use axum::response::IntoResponse;

#[instrument]
#[debug_handler]
pub async fn parse_csv(
    State(cache_state): State<CacheState>,
    Json(value): Json<CsvInput>,
) -> Result<
    impl IntoResponse,
    (
        StatusCode,
        String,
    ),
> {
    let headers = ["Title", "Description", "Actors", "Genres", "Director"];
    let csv_with_headers = format!(
        "{}\n{}",
        headers.join(","),
        value.input
    );

    let movies = match csv_utils::parse_csv(csv_with_headers.as_bytes()) {
        Ok(movies) => movies,
        Err(e) => {
            let error = format!(
                "Failed to parse CSV: {}",
                e
            );
            error!(
                "{}",
                error
            );
            return Err((
                StatusCode::BAD_REQUEST,
                error,
            ));
        }
    };

    if let Err(e) = database::insert_full_movies(movies).await {
        let error = format!(
            "Failed to insert movies: {}",
            e
        );
        error!(
            "{}",
            error
        );
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            error,
        ));
    }

    // Refresh the cache with the latest movies
    match database::get_movies().await {
        Ok(updated_movies) => {
            info!("Movies saved successfully");

            // Update the movies in the RwLock
            let mut movies = cache_state
                .movies
                .write()
                .await;
            *movies = updated_movies;
            let count = movies.len();
            info!(
                "Successfully refreshed movie cache with {} movies",
                count
            );
            Ok((
                StatusCode::OK,
                Json(()),
            ))
        }
        Err(e) => {
            let error = format!(
                "Failed to refresh movie cache: {}",
                e
            );
            error!(
                "{}",
                error
            );
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                error,
            ))
        }
    }
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
    let embedding_result = ai_chat::get_embedding(text).await;
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

/// Helper function to create Ollama client with configuration
fn _create_ollama_client(query: String) -> Result<Arc<OllamaClient>, String> {
    let ollama_host = std::env::var("OLLAMA_HOST").unwrap_or_else(|_| "ollama".to_string());
    let ollama_port = std::env::var("OLLAMA_PORT").unwrap_or_else(|_| "11434".to_string());

    let ollama_url = format!(
        "http://{}:{}",
        ollama_host, ollama_port
    );
    let parsed_url = ollama_url
        .parse()
        .map_err(
            |e| {
                format!(
                    "Invalid Ollama URL {}: {}",
                    ollama_url, e
                )
            },
        )?;

    let client = Arc::new(
        OllamaClient {
            ollama_client: Ollama::from_url(parsed_url),
            ai_action: AiAction {
                uuid: "movie_search".to_string(),
                action: query,
                model: Some("phi3.5".to_string()),
                temperature: None,
            },
        },
    );

    Ok(client)
}

/// Helper function to parse AI response into movie IDs
fn _parse_movie_ids_from_response(response: String) -> Result<Vec<i32>, String> {
    let ids: Vec<i32> = response
        .split(",")
        .filter_map(
            |id| {
                let trimmed = id.trim();
                match trimmed.parse::<i32>() {
                    Ok(parsed_id) if parsed_id > 0 => Some(parsed_id),
                    Ok(_) => {
                        warn!(
                            "Ignoring invalid movie ID: {}",
                            trimmed
                        );
                        None
                    }
                    Err(e) => {
                        warn!(
                            "Failed to parse movie ID '{}': {}",
                            trimmed, e
                        );
                        None
                    }
                }
            },
        )
        .collect();

    if ids.is_empty() {
        return Err("No valid movie IDs found in AI response".to_string());
    }

    Ok(ids)
}

/// Combines text and vector search results using parallel processing
async fn combined_search(
    query: &str,
    all_movies: Arc<Vec<FullMovie>>,
) -> Result<Vec<ScoredMovie>, String> {
    info!(
        "Starting combined search for: {}",
        query
    );

    // Extract entities from the movie list
    let (titles, actors, genres) = extract_entities(
        query,
        &all_movies,
    )
    .await;
    info!(
        "Extracted entities for combined search - titles: {:?}, actors: {:?}, genres: {:?}",
        titles, actors, genres
    );

    let embedding_start = Instant::now();
    // Generate query embedding once
    let query_embedding = match embedding(query).await {
        Ok(embedding) => Some(embedding),
        Err(e) => {
            warn!(
                "Failed to generate query embedding: {}",
                e
            );
            None
        }
    };

    info!(
        "Query embedding generated: {}",
        query_embedding.is_some()
    );

    if let Some(embedding) = &query_embedding {
        info!(
            "Query embedding length: {}",
            embedding.len()
        );
    }

    let embedding_duration = embedding_start.elapsed();
    info!(
        "Query embedding generated in {:.3}ms",
        embedding_duration.as_millis()
    );

    let start_time = Instant::now();

    let min_text_score = 0;
    
    // Get vector-similar movies from postgres pgvector if we have an embedding
    let vector_movie_scores: std::collections::HashMap<i32, f32> = if let Some(embedding) = query_embedding.as_ref() {
        match database::search_movies(embedding.clone(), 50).await {
            Ok(movies) => {
                info!("Retrieved {} movies from postgres pgvector", movies.len());
                // Assign scores based on rank (top result = 1.0, linearly decreasing)
                movies
                    .iter()
                    .enumerate()
                    .map(|(idx, movie)| {
                        let score = 1.0 - (idx as f32 / 50.0);
                        (movie.id, score)
                    })
                    .collect()
            }
            Err(e) => {
                warn!("Failed to get vector-similar movies from postgres: {}", e);
                std::collections::HashMap::new()
            }
        }
    } else {
        std::collections::HashMap::new()
    };

    // Calculate text scores for all movies and get vector scores from pgvector results
    let movies_with_scores: Vec<(
        &FullMovie,
        usize,
        f32,
    )> = all_movies
        .iter()
        .map(|movie| {
            let text_score = movie.text_match_score(&titles, &actors, &genres);
            let vector_score = vector_movie_scores.get(&movie.id).copied().unwrap_or(0.0);
            (movie, text_score, vector_score)
        })
        .filter(|(_, text_score, vector_score)| {
            *text_score > min_text_score || *vector_score > 0.0
        })
        .collect();

    info!(
        "Found {} movies with matches in combined search",
        movies_with_scores.len()
    );

    // Sort by combined score (text matches weighted more heavily)
    let combined: Vec<ScoredMovie> = {
        let mut sorted: Vec<_> = movies_with_scores;
        sorted.sort_unstable_by(
            |(_, score_a, sim_a), (_, score_b, sim_b)| {
                let combined_a = (*score_a as f32 * 2.0) + sim_a;
                let combined_b = (*score_b as f32 * 2.0) + sim_b;
                combined_b
                    .partial_cmp(&combined_a)
                    .unwrap_or(std::cmp::Ordering::Equal)
            },
        );
        sorted
            .into_iter()
            .map(|(movie, text_score, vector_score)| ScoredMovie {
                movie: movie.clone(),
                text_score,
                vector_score,
            })
            .collect()
    };

    info!(
        "Total combined results: {}",
        combined.len()
    );

    // Log top 5 results for debugging with match scores
    if !combined.is_empty() {
        info!(
            "Top {} search results:",
            combined
                .len()
                .min(5)
        );
        for (i, scored_movie) in combined
            .iter()
            .take(5)
            .enumerate()
        {
            info!(
                "  {}. {} (ID: {}, text score: {}, vector score: {:.3})",
                i + 1,
                scored_movie.movie.name,
                scored_movie.movie.id,
                scored_movie.text_score,
                scored_movie.vector_score
            );
        }
    } else {
        info!("No matching movies found");
    }

    let duration = start_time.elapsed();
    info!(
        "Completed combined search in {}ms",
        duration.as_millis()
    );

    Ok(combined)
}

/// Main endpoint for getting matching movies using combined search
#[instrument(
    skip(state, search_request),  // Skip both state and search_request from automatic logging
    fields(
        query = %search_request.query,
        query_len = search_request.query.len(),
    )
)]
#[debug_handler]
pub async fn get_matching_movies(
    State(state): State<CacheState>,
    Json(search_request): Json<SearchRequest>,
) -> Json<Option<Vec<ScoredMovie>>> {
    // Get a clone of the movies from the RwLock
    let movies = {
        let movies_guard = state
            .movies
            .read()
            .await;
        (*movies_guard).clone()
    };

    // Update the span with the movie count after acquiring the lock
    tracing::Span::current().record(
        "movie_count",
        tracing::field::display(movies.len()),
    );

    let query = search_request
        .query
        .trim();
    if query.is_empty() {
        return Json(Some(Vec::new()));
    }

    match combined_search(
        query,
        Arc::new(movies),
    )
    .await
    {
        Ok(movies) => Json(Some(movies)),
        Err(e) => {
            error!(
                "Search failed: {}",
                e
            );
            Json(None)
        }
    }

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
