use std::sync::Arc;

use ai_chat::OllamaClient;
use axum::{
    extract::{Path, State},
    Json,
};
use axum_macros::debug_handler;
use database::{question::AiAction, FullMovie, SearchRequest};
use itertools::Itertools;
use log::{error, info, warn};
use ollama_rs::{self, Ollama, error::OllamaError};
use std::collections::{HashMap, HashSet};
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

/// Combines text and vector search results, boosting movies that appear in both
async fn combined_search(
    query: &str,
    all_movies: Arc<Vec<FullMovie>>,
) -> Result<Vec<FullMovie>, String> {
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

    // Run both searches in parallel
    let (text_result, vector_result) = tokio::join!(
        search_movies_by_text(query),
        search_movies_by_embedding(query)
    );

    let text_movies = text_result.unwrap_or_default();
    let vector_movies = vector_result.unwrap_or_default();

    info!(
        "Text search found {} movies",
        text_movies.len()
    );
    info!(
        "Vector search found {} movies",
        vector_movies.len()
    );

    // Create a map of movie IDs to their scores and occurrence counts
    let mut movie_scores: HashMap<
        i32,
        (
            FullMovie,
            usize,
            usize,
        ),
    > = HashMap::new();

    // Add text search results (with higher weight)
    let text_movies_len = text_movies.len();
    for (i, movie) in text_movies
        .into_iter()
        .enumerate()
    {
        let score = text_movies_len - i; // Higher score for better ranking
        let entry = movie_scores
            .entry(movie.id)
            .or_insert((
                movie, 0, 0,
            ));
        entry.1 += score * 2; // Give text matches higher weight
        entry.2 += 1;
    }

    // Add vector search results
    let vector_movies_len = vector_movies.len();
    for (i, movie) in vector_movies
        .into_iter()
        .enumerate()
    {
        let score = vector_movies_len - i;
        let entry = movie_scores
            .entry(movie.id)
            .or_insert((
                movie, 0, 0,
            ));
        entry.1 += score;
        entry.2 += 1;
    }

    // Convert to vec and sort by score (descending)
    let mut combined: Vec<FullMovie> = movie_scores
        .into_values()
        .map(
            |(movie, score, count)| {
                (
                    movie,
                    score * count,
                )
            },
        ) // Boost score by number of occurrences
        .collect::<Vec<_>>()
        .into_iter()
        .sorted_by(|(_, score_a), (_, score_b)| score_b.cmp(score_a))
        .map(|(movie, _)| movie)
        .collect();

    // If we don't have enough results, fill with text search results first, then vector
    if combined.len() < 10 {
        let text_movies = search_movies_by_text(query)
            .await
            .unwrap_or_default();
        let mut text_set: HashSet<i32> = combined
            .iter()
            .map(|m| m.id)
            .collect();

        // Add text search results that aren't already in the combined results
        for movie in text_movies {
            let movie_id = movie.id;
            if !text_set.contains(&movie_id) {
                combined.push(movie);
                text_set.insert(movie_id);
                if combined.len() >= 10 {
                    break;
                }
            }
        }

        // If still not enough, add vector search results
        if combined.len() < 10 {
            let vector_movies = search_movies_by_embedding(query)
                .await
                .unwrap_or_default();
            let mut vector_set: HashSet<i32> = combined
                .iter()
                .map(|m| m.id)
                .collect();

            for movie in vector_movies {
                let movie_id = movie.id;
                if !vector_set.contains(&movie_id) {
                    combined.push(movie);
                    vector_set.insert(movie_id);
                    if combined.len() >= 10 {
                        break;
                    }
                }
            }
        }
    }

    info!(
        "Total combined results: {}",
        combined.len()
    );

    // Log top 5 results for debugging with match scores
    for (i, movie) in combined
        .iter()
        .take(5)
        .enumerate()
    {
        let score = count_matches(
            movie, &titles, &actors, &genres,
        );
        info!(
            "Result #{}: {} (match score: {})",
            i + 1,
            movie.name,
            score
        );
        info!(
            "  Genres: {:?}",
            movie.genres
        );
        info!(
            "  Actors: {}",
            movie
                .actors
                .iter()
                .map(
                    |a| a
                        .name
                        .as_str()
                )
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    Ok(combined)
}

/// Main endpoint for getting matching movies using combined search
#[instrument]
#[debug_handler]
pub async fn get_matching_movies(
    State(state): State<CacheState>,
    Json(search_request): Json<SearchRequest>,
) -> Json<Option<Vec<FullMovie>>> {
    let query = search_request
        .query
        .trim();
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
