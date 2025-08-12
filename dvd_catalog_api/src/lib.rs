use std::sync::Arc;

use ai_chat::OllamaClient;
use axum::{
    extract::{Path, State},
    Json,
};
use axum_macros::debug_handler;
use database::{question::AiAction, FullMovie, SearchRequest};
use log::{debug, error, info, warn};
use models::VectorSimilarity;
use ollama_rs::{Ollama, error::OllamaError};
use tokio_rayon::rayon::prelude::*;
use tracing::instrument;

// Movie search functionality module
pub mod movie_search;
pub use movie_search::{extract_entities, search_movies_by_text};

#[derive(Clone, Debug)]
pub struct CacheState {
    pub movies: Arc<Vec<FullMovie>>,
}

#[instrument]
pub async fn hello_world() -> &'static str {
    "Hello from DVD_CATALOG_API!"
}

#[instrument]
#[debug_handler]
pub async fn get_dvds(State(state): State<CacheState>) -> Json<Option<Vec<FullMovie>>> {
    if state.movies.is_empty() {
        Json(None)
    } else {
        Json(Some((*state.movies).clone()))
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
    (axum::http::StatusCode::NOT_FOUND, "Chat endpoint temporarily disabled")
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


// process_movies_parallel has been moved to movie_search.rs

// SearchCriteria has been moved to movie_search.rs

// search_movies_by_text has been moved to movie_search.rs

/// Helper function to count how many conditions a movie matches with weighted scoring
fn count_matches(
    movie: &FullMovie,
    titles: &[String],
    actors: &[String],
    genres: &[String],
) -> usize {
    let mut score = 0;

    // Title matches are very specific, give them higher weight
    if !titles.is_empty()
        && titles
            .iter()
            .any(
                |t| {
                    movie
                        .name
                        .to_lowercase()
                        .contains(t)
                },
            )
    {
        score += 3; // Higher weight for title matches
    }

    // Check director matches
    if let Some(director) = &movie.director {
        let director_name = director
            .name
            .to_lowercase();
        if !titles.is_empty()
            && titles
                .iter()
                .any(|t| director_name.contains(t))
        {
            score += 3; // Title matched in director name
        }
        if !actors.is_empty()
            && actors
                .iter()
                .any(|a| director_name.contains(a))
        {
            score += 2; // Actor name matched in director name
        }
    }

    // Check actor matches
    let actor_matches = if !actors.is_empty() {
        let matches: Vec<_> = movie
            .actors
            .iter()
            .filter(
                |a| {
                    let actor_name = a
                        .name
                        .to_lowercase();
                    let found = actors
                        .iter()
                        .any(
                            |name| {
                                name.split_whitespace()
                                    .all(|part| actor_name.contains(&part.to_lowercase()))
                            },
                        );
                    if found {
                        info!(
                            "Actor match: {} in {}",
                            a.name, movie.name
                        );
                    }
                    found
                },
            )
            .collect();
        matches.len()
    } else {
        0
    };

    // Give extra points for each matching actor (up to 2 actors)
    score += actor_matches.min(2) * 2;

    // Check genre matches
    let genre_matches = if !genres.is_empty() {
        let matches: Vec<_> = genres
            .iter()
            .filter(
                |g| {
                    let found = movie
                        .genres
                        .iter()
                        .any(
                            |genre| {
                                genre
                                    .to_lowercase()
                                    .contains(&g.to_lowercase())
                            },
                        );
                    if found {
                        info!(
                            "Genre match: {} contains {}",
                            movie.name, g
                        );
                    }
                    found
                },
            )
            .collect();
        matches.len()
    } else {
        0
    };

    // Give points for genre matches
    score += genre_matches;

    // Bonus: If we have both actor and genre matches, give extra points
    if actor_matches > 0 && genre_matches > 0 {
        score += 2; // Bonus for matching both actor and genre
    }

    score
}

/// Helper function to search for movies using vector embeddings
async fn search_movies_by_embedding(query: &str) -> Result<Vec<FullMovie>, String> {
    let embedding = embedding(query)
        .await
        .map_err(
            |e| {
                format!(
                    "Failed to generate embedding: {}",
                    e
                )
            },
        )?;

    info!("Searching for movies using embedding");
    let movie_ids = database::search_movies(
        embedding, 15,
    )
    .await
    .map_err(
        |e| {
            error!(
                "Failed to search movies: {:#?}",
                e
            );
            format!(
                "Database search failed: {}",
                e
            )
        },
    )?;

    Ok(movie_ids)
}

/// Helper function to create Ollama client with configuration
fn create_ollama_client(query: String) -> Result<Arc<OllamaClient>, String> {
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
fn parse_movie_ids_from_response(response: String) -> Result<Vec<i32>, String> {
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

use tokio_rayon::rayon::prelude::*;

/// Combines text and vector search results using parallel processing
async fn combined_search(
    query: &str,
    all_movies: Arc<Vec<FullMovie>>,
) -> Result<Vec<FullMovie>, String> {
    info!("Starting combined search for: {}", query);

    // Extract entities from the movie list
    let (titles, actors, genres) = extract_entities(query, &all_movies).await;
    info!(
        "Extracted entities for combined search - titles: {:?}, actors: {:?}, genres: {:?}",
        titles, actors, genres
    );

    // Generate query embedding once
    let query_embedding = match embedding(query).await {
        Ok(embedding) => Some(embedding),
        Err(e) => {
            warn!("Failed to generate query embedding: {}", e);
            None
        }
    };

    // Process movies in parallel using tokio-rayon 2.1.0
    info!("Total movies to search: {}", all_movies.len());
    info!("Query embedding generated: {}", query_embedding.is_some());
    if let Some(embedding) = &query_embedding {
        info!("Query embedding length: {}", embedding.len());
    }

    let movies_with_scores: Vec<(FullMovie, usize, f32)> = all_movies
        .iter()
        .par_bridge()
        .filter_map(|movie| {
            // Calculate text score (reusing existing logic)
            let text_score = count_matches(movie, &titles, &actors, &genres);
            
            // Calculate vector similarity score if embedding is available
            let vector_score = query_embedding.as_ref()
                .and_then(|embedding| {
                    let score = movie.cosine_similarity(embedding);
                    if score.is_none() {
                        debug!("No embedding for movie: {}", movie.name);
                    }
                    score
                })
                .unwrap_or(0.0);
            
            // Only include movies that match at least one criterion
            if text_score > 0 || vector_score > 0.0 {
                debug!("Match found - Movie: {}, text_score: {}, vector_score: {}", movie.name, text_score, vector_score);
                Some((movie.clone(), text_score, vector_score))
            } else {
                debug!("No match - Movie: {}, text_score: {}, vector_score: {}", movie.name, text_score, vector_score);
                None
            }
        })
        .collect();

    info!(
        "Found {} movies with matches in combined search",
        movies_with_scores.len()
    );

    // Sort by combined score (text matches weighted more heavily) and take top 50
    let mut combined: Vec<FullMovie> = {
        let mut sorted: Vec<_> = movies_with_scores.into_iter().collect();
        sorted.par_sort_unstable_by(|(_, score_a, sim_a), (_, score_b, sim_b)| {
            let combined_a = (*score_a as f32 * 2.0) + sim_a;
            let combined_b = (*score_b as f32 * 2.0) + sim_b;
            combined_b.partial_cmp(&combined_a).unwrap_or(std::cmp::Ordering::Equal)
        });
        sorted.into_iter()
            .take(50)
            .map(|(movie, _, _)| movie)
            .collect()
    };

    // If we don't have enough results, include more movies with any matches
    if combined.len() < 10 {
        let additional_movies = all_movies
            .iter()
            .filter(|m| !combined.iter().any(|cm| cm.id == m.id))
            .filter(|movie| {
                // Include movies that match any criteria
                count_matches(movie, &titles, &actors, &genres) > 0 ||
                query_embedding.as_ref()
                    .and_then(|e| movie.cosine_similarity(e))
                    .map(|score| score > 0.5) // Threshold for similarity
                    .unwrap_or(false)
            })
            .take(10 - combined.len())
            .cloned()
            .collect::<Vec<_>>();
            
        combined.extend(additional_movies);
    }

    info!(
        "Total combined results: {}",
        combined.len()
    );

    // Log top 5 results for debugging with match scores
    if !combined.is_empty() {
        info!("Top {} search results:", combined.len().min(5));
        for (i, movie) in combined.iter().take(5).enumerate() {
            let score = count_matches(movie, &titles, &actors, &genres);
            info!(
                "  {}. {} (ID: {}, match score: {})",
                i + 1,
                movie.name,
                movie.id,
                score
            );
        }
    } else {
        info!("No matching movies found");
    }

    Ok(combined)
}

/// Main endpoint for getting matching movies using combined search
#[instrument(
    skip(state, search_request),  // Skip both state and search_request from automatic logging
    fields(
        query = %search_request.query,
        query_len = search_request.query.len(),
        movie_count = state.movies.len()
    )
)]
#[debug_handler]
pub async fn get_matching_movies(
    State(state): State<CacheState>,
    Json(search_request): Json<SearchRequest>,
) -> Json<Option<Vec<FullMovie>>> {
    let query = search_request.query.trim();
    if query.is_empty() {
        return Json(Some(Vec::new()));
    }

    match combined_search(
        query,
        Arc::clone(&state.movies),
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
