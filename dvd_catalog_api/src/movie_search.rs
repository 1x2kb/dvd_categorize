use std::sync::Arc;
use std::collections::HashSet;

use ai_chat::OllamaClient;
use axum::{
    extract::Json,
};
use axum_macros::debug_handler;
use database::{question::AiAction, FullMovie};
use models::TextMatchScoring;
use log::{error, info};
use ollama_rs::Ollama;
use tracing::instrument;

use crate::embedding;

/// Search criteria for movie matching
#[derive(Clone)]
struct SearchCriteria {
    titles: Vec<String>,
    actors: Vec<String>,
    genres: Vec<String>,
}

/// Extracts entities (titles, actors, genres) from a search query by comparing against a list of movies.
/// 
/// # Arguments
/// * `query` - The search query to parse
/// * `movies` - List of all movies to extract entities from
#[instrument(skip(movies))]
pub async fn extract_entities(
    query: &str,
    movies: &[FullMovie],
) -> (Vec<String>, Vec<String>, Vec<String>) {
    let start_time = std::time::Instant::now();

    // Extract all unique entities from the movie list
    info!("Starting entity extraction for query: {}", query);
    let (all_genres, all_actors, all_directors) = extract_entities_from_movies(movies).await;

    info!(
        "Entity extraction completed in {:.2?} ({} unique genres, {} unique actors, {} unique directors)",
        start_time.elapsed(),
        all_genres.len(),
        all_actors.len(),
        all_directors.len()
    );

    let query_lower = query.to_lowercase();
    let mut titles = Vec::new();
    let mut actors = HashSet::new();
    let mut genres = HashSet::new();

    // First, look for exact matches of known entities in the query
    for genre in &all_genres {
        let genre_lower = genre.to_lowercase();
        // Match whole word to avoid partial matches (e.g., "comedy" in "comedy" but not "comedy" in "comedycentral")
        if query_lower
            .split_whitespace()
            .any(|w| w == genre_lower)
            || query_lower
                .split_whitespace()
                .any(|w| w == format!("{}s", genre_lower))
        {
            genres.insert(genre_lower);
        }
    }

    // Look for actors and directors
    for entity in all_actors.iter().chain(all_directors.iter()) {
        let entity_lower = entity.to_lowercase();
        let name_parts: Vec<&str> = entity_lower.split_whitespace().collect();

        // Check for exact match first (most reliable)
        if query_lower == entity_lower
            || query_lower.contains(&format!(" {entity_lower} "))
            || query_lower.starts_with(&format!("{entity_lower} "))
            || query_lower.ends_with(&format!(" {entity_lower}"))
        {
            actors.insert(entity_lower);
            continue;
        }

        // For multi-word names, check if all parts appear in order in the query
        if name_parts.len() > 1 {
            let mut query_words = query_lower.split_whitespace();
            let all_parts_found = name_parts
                .iter()
                .all(|&part| {
                    // Skip very short words in the name to avoid false positives
                    if part.len() <= 2 {
                        return true;
                    }
                    query_words.any(|w| w == part)
                });

            // If all parts found in order, it's a match
            if all_parts_found && name_parts.iter().all(|p| p.len() > 2) {
                actors.insert(entity_lower);
                continue;
            }
        }

        // For single-word names or as a fallback, check for standalone word match
        if name_parts.len() == 1 && name_parts[0].len() > 2 {
            // Match whole word only (with word boundaries)
            let word = name_parts[0];
            if query_lower.split_whitespace().any(|w| w == word)
                || query_lower == word
                || query_lower.starts_with(&format!("{word} "))
                || query_lower.ends_with(&format!(" {word}"))
            {
                actors.insert(entity_lower);
            }
        }
    }

    // Extract potential title by removing matched entities
    let mut remaining_query = query_lower.clone();
    for genre in &genres {
        remaining_query = remaining_query
            .replace(genre, "")
            .replace("  ", " ")
            .trim()
            .to_string();
    }
    for actor in &actors {
        remaining_query = remaining_query
            .replace(actor, "")
            .replace("  ", " ")
            .trim()
            .to_string();
    }

    // Remove common stop words and phrases
    let stop_phrases = [
        "movies", "movie", "films", "film", "show", "shows", "about",
        "with", "starring", "featuring", "directed by", "that are",
        "which are", "that have", "which have",
    ];

    for phrase in stop_phrases {
        remaining_query = remaining_query.replace(phrase, "");
    }

    // Clean up any extra spaces
    remaining_query = remaining_query.split_whitespace().collect::<Vec<_>>().join(" ");

    // If we have remaining text that doesn't match known entities, treat it as a title
    if !remaining_query.trim().is_empty() {
        titles.push(remaining_query.trim().to_string());
    }

    info!(
        "Extracted entities - titles: {:?}, actors: {:?}, genres: {:?}",
        titles, actors, genres
    );

    (
        titles,
        actors.into_iter().collect(),
        genres.into_iter().collect(),
    )
}

/// Extracts all unique entities from a list of movies
async fn extract_entities_from_movies(movies: &[FullMovie]) -> (Vec<String>, Vec<String>, Vec<String>) {
    use std::collections::HashSet;
    use std::time::Instant;

    let start_time = Instant::now();
    let total_movies = movies.len();
    info!("Starting entity extraction for {} movies", total_movies);

    let mut all_genres = HashSet::new();
    let mut all_actors = HashSet::new();
    let mut all_directors = HashSet::new();

    // Process each movie sequentially
    for movie in movies {
        // Process genres
        for genre in &movie.genres {
            all_genres.insert(genre.to_lowercase());
        }

        // Process actors
        for actor in &movie.actors {
            all_actors.insert(actor.name.to_lowercase());
        }

        // Process director if present
        if let Some(director) = &movie.director {
            all_directors.insert(director.name.to_lowercase());
        }
    }

    let elapsed = start_time.elapsed();
    info!(
        "Extracted entities from {} movies in {:.2?} ({} movies/sec)",
        total_movies,
        elapsed,
        total_movies as f64 / elapsed.as_secs_f64()
    );

    (
        all_genres.into_iter().collect(),
        all_actors.into_iter().collect(),
        all_directors.into_iter().collect(),
    )
}

/// Processes movies in parallel to find matches against search criteria.
///
/// # Arguments
/// * `movies` - Vector of movies to search through
/// * `criteria` - Search criteria (titles, actors, genres)
///
/// # Returns
/// Vector of tuples containing matching movies and their match scores
async fn process_movies_parallel(
    movies: Vec<FullMovie>,
    criteria: SearchCriteria,
) -> Result<Vec<(FullMovie, usize)>, String> {
    use std::sync::Arc;
    use std::time::Instant;

    let start_time = Instant::now();
    let total_movies = movies.len();
    info!("Starting parallel processing of {} movies", total_movies);

    // Wrap criteria in Arc to share between tasks
    let criteria = Arc::new(criteria);

    // Process movies in parallel
    let tasks = movies.into_iter().map(|movie| {
        let criteria = Arc::clone(&criteria);
        tokio::task::spawn(async move {
            let score = count_matches(
                &movie,
                &criteria.titles,
                &criteria.actors,
                &criteria.genres,
            );
            if score > 0 {
                Some((movie, score))
            } else {
                None
            }
        })
    }).collect::<Vec<_>>();

    info!("Spawned {} parallel tasks", tasks.len());

    // Process results
    let mut results = Vec::with_capacity(tasks.len());
    let mut processed = 0;
    let log_percent = 10;
    let log_interval = (total_movies / log_percent).max(1); // Log every log_percent

    for task in tasks {
        if let Some(scored_movie) = task.await.map_err(|e| format!("Task failed: {}", e))? {
            results.push(scored_movie);
        }

        // Log progress
        processed += 1;
        if processed % log_interval == 0 || processed == total_movies {
            let progress = (processed as f64 / total_movies as f64 * 100.0) as u32;
            info!("Processed {}/{} movies ({}%)", processed, total_movies, progress);
        }
    }

    let duration = start_time.elapsed();
    info!(
        "Processed {} movies in {:.2?} ({} matches found)",
        total_movies,
        duration,
        results.len()
    );

    Ok(results)
}

/// Helper function to count how many conditions a movie matches with weighted scoring
/// 
/// This is now a thin wrapper around the `text_match_score` method from the `TextMatchScoring` trait.
pub fn count_matches(
    movie: &FullMovie,
    titles: &[String],
    actors: &[String],
    genres: &[String],
) -> usize {
    movie.text_match_score(titles, actors, genres)
}

/// Searches for movies matching the given query text using parallel processing.
///
/// # Arguments
/// * `query` - Search query string
///
/// # Returns
/// Vector of movies matching the query, sorted by relevance
pub async fn search_movies_by_text(query: &str) -> Result<Vec<FullMovie>, String> {
    info!("Starting text search for: {}", query);
    let start_time = std::time::Instant::now();

    // Fetch movies
    let all_movies = database::get_movies()
        .await
        .map_err(|e| format!("Failed to fetch movies: {}", e))?;
    
    info!(
        "Fetched {} movies in {:.2?}",
        all_movies.len(),
        start_time.elapsed()
    );

    // Extract search criteria from the movie list
    let (titles, actors, genres) = extract_entities(query, &all_movies).await;
    info!(
        "Extracted entities - titles: {:?}, actors: {:?}, genres: {:?}",
        titles, actors, genres
    );

    // Process movies in parallel
    let criteria = SearchCriteria { titles, actors, genres };
    let mut results = process_movies_parallel(all_movies, criteria).await?;

    // Sort by score (highest first)
    results.sort_by(|(_, score_a), (_, score_b)| score_b.cmp(score_a));

    // Log top 10 results
    info!("Top 10 text search results:");
    for (i, (movie, score)) in results.iter().take(10).enumerate() {
        info!("  {}. {} (score: {})", i + 1, movie.name, score);
        info!("     Genres: {:?}", movie.genres);
        info!(
            "     Actors: {}",
            movie.actors.iter().map(|a| a.name.as_str()).collect::<Vec<_>>().join(", ")
        );
    }

    // Return just the movies, without the scores
    Ok(results.into_iter().map(|(movie, _)| movie).collect())
}

/// Handles chat interactions with the AI, processing movie-related queries.
/// 
/// This endpoint takes an AI action (containing a question/query), searches for relevant movies,
/// and generates a response using the AI model.
/// 
/// # Arguments
/// * `action` - JSON payload containing the chat action details (question, model, temperature, etc.)
/// 
/// # Returns
/// JSON response containing the AI's response to the query
#[instrument]
#[debug_handler]
pub async fn chat(Json(action): Json<AiAction>) -> Json<AiAction> {
    let dvds = database::get_movies()
        .await
        .unwrap_or_else(|_| Vec::new());

    let (uuid, question, model, temperature) = (
        action.uuid,
        action.action,
        action.model,
        action.temperature,
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

    let full_movies = if let Some(movie_embeddings) = movie_ids {
        // Extract just the movie IDs from the embeddings
        let movie_ids: Vec<i32> = movie_embeddings.into_iter().map(|m| m.id).collect();
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
                    temperature: temperature,
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
            temperature: None,
        },
    )
}
