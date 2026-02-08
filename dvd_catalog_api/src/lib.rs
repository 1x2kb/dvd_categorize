use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use axum_macros::debug_handler;
use database::{FullMovie, SearchRequest};
use log::{error, info};
use models::{CsvInput, ScoredMovie};
use ollama_rs::error::OllamaError;
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
        Json(Some(movies.clone()))
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

#[instrument(skip(cache_state), fields(movie_count))]
#[debug_handler]
pub async fn export_csv(
    State(cache_state): State<CacheState>,
) -> impl axum::response::IntoResponse {
    let movies = cache_state
        .movies
        .read()
        .await
        .clone();

    let movie_count = movies.len();
    tracing::Span::current().record(
        "movie_count",
        movie_count,
    );

    info!(
        "Starting CSV export for {} movies",
        movie_count
    );

    match csv_utils::movies_to_csv(&movies) {
        Ok(csv) => {
            let csv_size = csv.len();
            info!(
                "CSV export successful: {} movies exported, {} bytes",
                movie_count, csv_size
            );
            (
                StatusCode::OK,
                [
                    (
                        "Content-Type",
                        "text/csv",
                    ),
                    (
                        "Content-Disposition",
                        "attachment; filename=movies.csv",
                    ),
                ],
                csv,
            )
        }
        Err(e) => {
            error!(
                "Failed to export CSV for {} movies: {}",
                movie_count, e
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                [
                    (
                        "Content-Type",
                        "text/plain",
                    ),
                    (
                        "Content-Disposition",
                        "",
                    ),
                ],
                format!(
                    "Failed to export CSV: {}",
                    e
                ),
            )
        }
    }
}

#[instrument]
#[debug_handler]
pub async fn preview_csv(Json(value): Json<CsvInput>) -> impl axum::response::IntoResponse {
    let movies = csv_utils::parse_csv(value.input.as_bytes()).unwrap_or_else(
        |e| {
            error!(
                "Failed to parse CSV: {}",
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
    let movies = match csv_utils::parse_csv(value.input.as_bytes()) {
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
        "Getting embeddings for user query: {}",
        text
    );

    let embedding_result = ai_chat::get_embedding(text).await;

    match &embedding_result {
        Ok(embedding) => {
            info!(
                "Successfully generated embeddings (length: {})",
                embedding.len()
            );
        }
        Err(e) => {
            error!(
                "Failed to get embeddings: {:#?}",
                e
            );
        }
    }

    embedding_result
}

/// Combines text and vector search results using Reciprocal Rank Fusion (RRF)
async fn combined_search(
    query: &str,
    all_movies: &Arc<Vec<FullMovie>>,
    disable_enhancement: bool,
    search_mode: models::SearchMode,
    model: Option<&str>,
) -> Result<
    (
        Vec<ScoredMovie>,
        String,
    ),
    String,
> {
    info!(
        "Starting search (mode: {:?}) for: {}",
        search_mode, query
    );
    let start_time = Instant::now();

    // Use the hybrid search function - pass Arc::clone (cheap pointer increment)
    let (movie_results, enhanced_query) = movie_search::hybrid_search(
        query,
        Arc::clone(all_movies),
        50,
        disable_enhancement,
        search_mode,
        model,
    )
    .await;

    if movie_results.is_empty() {
        info!("No matching movies found");
        return Ok((
            Vec::new(),
            enhanced_query,
        ));
    }

    // Create lookup map from our Arc
    let movies_map: std::collections::HashMap<i32, &FullMovie> = all_movies
        .iter()
        .map(
            |movie| {
                (
                    movie.id, movie,
                )
            },
        )
        .collect();

    // Look up movies by ID and create ScoredMovie (clone for owned response)
    let scored_movies: Vec<ScoredMovie> = movie_results
        .into_iter()
        .filter_map(
            |(id, rfr_score)| {
                movies_map
                    .get(&id)
                    .map(
                        |&movie| ScoredMovie {
                            movie: movie.clone(),
                            vector_score: rfr_score,
                        },
                    )
            },
        )
        .collect();

    info!(
        "Hybrid search returned {} results",
        scored_movies.len()
    );

    // Log top 5 results
    if !scored_movies.is_empty() {
        info!(
            "Top {} search results:",
            scored_movies
                .len()
                .min(5)
        );
        for (i, scored_movie) in scored_movies
            .iter()
            .take(5)
            .enumerate()
        {
            info!(
                "  {}. {} (ID: {}, RRF Score: {:.4})",
                i + 1,
                scored_movie
                    .movie
                    .name,
                scored_movie
                    .movie
                    .id,
                scored_movie.vector_score
            );
        }
    }

    info!(
        "Hybrid search took {:.2?}",
        start_time.elapsed()
    );

    Ok((
        scored_movies,
        enhanced_query,
    ))
}

/// Main endpoint for getting matching movies using combined search
#[instrument(
    skip(state, search_request),
    fields(
        query = %search_request.query,
        query_len = search_request.query.len(),
    )
)]
#[debug_handler]
pub async fn get_matching_movies(
    State(state): State<CacheState>,
    Json(search_request): Json<SearchRequest>,
) -> Json<Option<models::SearchResponse>> {
    let movies = Arc::new(
        state
            .movies
            .read()
            .await
            .clone(),
    );

    tracing::Span::current().record(
        "movie_count",
        tracing::field::display(movies.len()),
    );

    let query = search_request
        .query
        .trim();
    if query.is_empty() {
        return Json(
            Some(
                models::SearchResponse {
                    results: Vec::new(),
                    original_query: query.to_string(),
                    enhanced_query: query.to_string(),
                },
            ),
        );
    }

    match combined_search(
        query,
        &movies,
        search_request.disable_enhancement,
        search_request.search_mode,
        search_request.model.as_deref(),
    )
    .await
    {
        Ok((results, enhanced_query)) => Json(
            Some(
                models::SearchResponse {
                    results,
                    original_query: query.to_string(),
                    enhanced_query,
                },
            ),
        ),
        Err(e) => {
            error!(
                "Search failed: {}",
                e
            );
            Json(None)
        }
    }
}

/// Update the location of a movie
#[instrument(skip(state))]
#[debug_handler]
pub async fn update_movie_location(
    State(state): State<CacheState>,
    Json(request): Json<models::UpdateLocationRequest>,
) -> Result<
    Json<()>,
    (
        StatusCode,
        String,
    ),
> {
    info!(
        "Updating location for movie ID {} to '{}'",
        request.movie_id, request.location
    );

    // Update the database
    database::update_movie_location(
        request.movie_id,
        request.location,
    )
    .await
    .map_err(
        |e| {
            error!(
                "Failed to update movie location: {}",
                e
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!(
                    "Failed to update movie location: {}",
                    e
                ),
            )
        },
    )?;

    // Refresh the cache with updated movies
    match database::get_movies().await {
        Ok(updated_movies) => {
            let mut movies = state
                .movies
                .write()
                .await;
            *movies = updated_movies;
            info!("Successfully updated movie location and refreshed cache");
            Ok(Json(()))
        }
        Err(e) => {
            error!(
                "Failed to refresh movie cache after location update: {}",
                e
            );
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!(
                    "Location updated but failed to refresh cache: {}",
                    e
                ),
            ))
        }
    }
}

/// Pull an Ollama model to make it available for use
#[instrument]
#[debug_handler]
pub async fn pull_ollama_model(
    Json(request): Json<models::PullModelRequest>,
) -> Result<
    Json<models::PullModelResponse>,
    (
        StatusCode,
        String,
    ),
> {
    info!(
        "Received request to pull Ollama model: {}",
        request.model_name
    );

    match ai_chat::pull_model(&request.model_name).await {
        Ok(message) => {
            info!(
                "Successfully pulled model: {}",
                request.model_name
            );
            Ok(
                Json(
                    models::PullModelResponse {
                        success: true,
                        message,
                    },
                ),
            )
        }
        Err(e) => {
            error!(
                "Failed to pull model {}: {}",
                request.model_name, e
            );
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                e,
            ))
        }
    }
}

/// List available Ollama models
#[instrument]
#[debug_handler]
pub async fn list_available_models() -> Result<
    Json<models::AvailableModelsResponse>,
    (
        StatusCode,
        String,
    ),
> {
    info!("Listing available Ollama models");

    match ai_chat::list_models().await {
        Ok(models) => {
            info!(
                "Successfully retrieved {} models",
                models.len()
            );
            Ok(
                Json(
                    models::AvailableModelsResponse {
                        models,
                    },
                ),
            )
        }
        Err(e) => {
            error!(
                "Failed to list models: {}",
                e
            );
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                e,
            ))
        }
    }
}

/// Get recent movies ordered by added_on descending
#[instrument]
#[debug_handler]
pub async fn get_recent_movies() -> Json<Vec<ScoredMovie>> {
    info!("Getting recent movies");

    match database::get_recent_movies(50).await {
        Ok(movies) => {
            info!(
                "Found {} recent movies",
                movies.len()
            );
            let scored_movies: Vec<ScoredMovie> = movies
                .into_iter()
                .map(|movie| ScoredMovie {
                    movie,
                    vector_score: 0.0,
                })
                .collect();
            Json(scored_movies)
        }
        Err(e) => {
            error!(
                "Failed to get recent movies: {}",
                e
            );
            Json(Vec::new())
        }
    }
}
