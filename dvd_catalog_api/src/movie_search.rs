use std::collections::HashSet;
use std::sync::Arc;

use ai_chat::OllamaClient;
use axum::extract::Json;
use axum_macros::debug_handler;
use database::{question::AiAction, FullMovie};
use log::{error, info};
use ollama_rs::Ollama;
use tracing::instrument;

use crate::embedding;

// Search scoring constants
const RRF_K_VALUE: f32 = 60.0; // Standard reciprocal rank fusion constant
const EXACT_TITLE_MATCH_SCORE: f32 = 100.0;
const EXACT_MATCH_BOOST_MULTIPLIER: f32 = 10.0;

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
) -> (
    Vec<String>,
    Vec<String>,
    Vec<String>,
) {
    let start_time = std::time::Instant::now();

    // Extract all unique entities from the movie list
    info!(
        "Starting entity extraction for query: {}",
        query
    );
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
        let genre_plural = format!(
            "{}s",
            genre_lower
        );
        // Match whole word to avoid partial matches
        if query_lower
            .split_whitespace()
            .any(|w| w == genre_lower || w == genre_plural)
        {
            genres.insert(genre_lower);
        }
    }

    // Look for actors and directors
    let query_words: Vec<&str> = query_lower
        .split_whitespace()
        .collect();

    for entity in all_actors
        .iter()
        .chain(all_directors.iter())
    {
        let entity_lower = entity.to_lowercase();
        let name_parts: Vec<&str> = entity_lower
            .split_whitespace()
            .collect();

        // Check for exact match first (most reliable)
        let exact_match = query_lower == entity_lower
            || query_lower.contains(&format!(" {entity_lower} "))
            || query_lower.starts_with(&format!("{entity_lower} "))
            || query_lower.ends_with(&format!(" {entity_lower}"));

        if exact_match {
            actors.insert(entity_lower);
            continue;
        }

        // For multi-word names (e.g., "Brad Pitt")
        if name_parts.len() > 1 {
            // Count how many significant name parts match the query
            let matching_parts: Vec<&str> = name_parts
                .iter()
                .filter(|&&part| part.len() > 3 && query_words.contains(&part))
                .copied()
                .collect();

            // If we have multiple query words that could be a full name,
            // require all parts to match (e.g., "Adam Sandler" should NOT match "Adam Baldwin")
            let has_multiple_name_candidates = query_words
                .iter()
                .filter(|w| w.len() > 3)
                .count()
                >= 2;

            if has_multiple_name_candidates {
                // Full name query: require all significant parts to match
                let all_significant_parts_match = name_parts
                    .iter()
                    .filter(|p| p.len() > 3)
                    .all(|&part| query_words.contains(&part));

                if all_significant_parts_match && !matching_parts.is_empty() {
                    actors.insert(entity_lower);
                    continue;
                }
            } else {
                // Partial query (e.g., just "Adam"): match if any significant part matches
                if !matching_parts.is_empty() {
                    actors.insert(entity_lower);
                    continue;
                }
            }
        }

        // For single-word names or as a fallback, check for standalone word match
        if name_parts.len() == 1 && name_parts[0].len() > 2 {
            let word = name_parts[0];
            let word_match = query_lower
                .split_whitespace()
                .any(|w| w == word)
                || query_lower == word
                || query_lower.starts_with(&format!("{word} "))
                || query_lower.ends_with(&format!(" {word}"));

            if word_match {
                actors.insert(entity_lower);
            }
        }
    }

    // Extract potential title by removing matched entities
    let mut remaining_query = query_lower.clone();
    for genre in &genres {
        remaining_query = remaining_query.replace(
            genre, "",
        );
    }
    for actor in &actors {
        remaining_query = remaining_query.replace(
            actor, "",
        );
    }
    remaining_query = remaining_query
        .replace(
            "  ", " ",
        )
        .trim()
        .to_string();

    // Remove common stop words and phrases
    let stop_phrases = [
        "movies",
        "movie",
        "films",
        "film",
        "show",
        "shows",
        "about",
        "with",
        "starring",
        "featuring",
        "directed by",
        "that are",
        "which are",
        "that have",
        "which have",
    ];

    for phrase in stop_phrases {
        remaining_query = remaining_query.replace(
            phrase, "",
        );
    }

    // Clean up any extra spaces
    remaining_query = remaining_query
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    // If we have remaining text that doesn't match known entities, treat it as a title
    if !remaining_query
        .trim()
        .is_empty()
    {
        titles.push(
            remaining_query
                .trim()
                .to_string(),
        );
    }

    info!(
        "Extracted entities - titles: {:?}, actors: {:?}, genres: {:?}",
        titles, actors, genres
    );

    (
        titles,
        actors
            .into_iter()
            .collect(),
        genres
            .into_iter()
            .collect(),
    )
}

/// Scores a movie based on keyword matches (title, actors, genres)
fn score_movie_by_keywords(movie: &FullMovie, criteria: &SearchCriteria) -> f32 {
    let mut score = 0.0;

    // Title matching (highest weight)
    for title in &criteria.titles {
        let movie_title_lower = movie
            .name
            .to_lowercase();
        let title_lower = title.to_lowercase();

        if movie_title_lower == title_lower {
            score += EXACT_TITLE_MATCH_SCORE; // Exact match
        } else if movie_title_lower.contains(&title_lower) {
            score += 50.0; // Partial substring match
        } else if title_lower.contains(&movie_title_lower) {
            score += 30.0; // Query contains movie title
        } else {
            // Check if movie title starts with the query (e.g., "RoboCop" starts with "Robot")
            if movie_title_lower.starts_with(&title_lower) {
                score += 45.0;
            } else {
                // Check word-level matching for compound words
                // Split on common delimiters and check if any word starts with query
                let movie_words: Vec<&str> = movie_title_lower
                    .split(|c: char| !c.is_alphanumeric())
                    .filter(|w| !w.is_empty())
                    .collect();

                for word in movie_words {
                    if word.starts_with(&title_lower) {
                        score += 40.0;
                        break;
                    }
                }
            }
        }
    }

    // Actor matching
    for actor in &criteria.actors {
        let actor_lower = actor.to_lowercase();
        for movie_actor in &movie.actors {
            let movie_actor_lower = movie_actor
                .name
                .to_lowercase();
            if movie_actor_lower == actor_lower || movie_actor_lower.contains(&actor_lower) {
                score += 20.0;
                break;
            }
        }
    }

    // Director matching
    if let Some(director) = &movie.director {
        for actor in &criteria.actors {
            // Actors list includes directors from extraction
            let actor_lower = actor.to_lowercase();
            let director_lower = director
                .name
                .to_lowercase();
            if director_lower == actor_lower || director_lower.contains(&actor_lower) {
                score += 25.0;
                break;
            }
        }
    }

    // Genre matching
    for genre in &criteria.genres {
        let genre_lower = genre.to_lowercase();
        for movie_genre in &movie.genres {
            let movie_genre_lower = movie_genre.to_lowercase();
            if movie_genre_lower == genre_lower {
                score += 15.0;
                break;
            }
        }
    }

    score
}

/// Performs keyword-based search on movies
/// Returns Vec<(movie_id, keyword_score)> sorted by score descending
fn keyword_search(
    movies: &[FullMovie],
    criteria: &SearchCriteria,
    limit: usize,
) -> Vec<(
    i32,
    f32,
)> {
    let mut scored_movies: Vec<(
        i32,
        f32,
    )> = movies
        .iter()
        .map(
            |movie| {
                let score = score_movie_by_keywords(
                    movie, criteria,
                );
                (
                    movie.id, score,
                )
            },
        )
        .filter(|(_, score)| *score > 0.0)
        .collect();

    // Sort by score descending
    scored_movies.sort_by(
        |a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
        },
    );

    // Return top N with scores
    scored_movies
        .into_iter()
        .take(limit)
        .collect()
}

/// Reciprocal Rank Fusion: Combines multiple ranked lists
/// Formula: RRF_score = sum(1 / (k + rank)) where k=60 is standard
/// Exact title matches (keyword_score >= 100) get massive boost to ensure they rank first
/// Returns a vector of (movie_id, score) tuples sorted by score descending
fn reciprocal_rank_fusion(
    keyword_results: Vec<(
        i32,
        f32,
    )>,
    vector_ranks: Vec<i32>,
    k: f32,
) -> Vec<(
    i32,
    f32,
)> {
    use std::collections::HashMap;

    let mut scores: HashMap<i32, f32> = HashMap::new();

    // Add scores from keyword search with exact match detection
    for (rank, (movie_id, keyword_score)) in keyword_results
        .iter()
        .enumerate()
    {
        let mut rfr_score = 1.0 / (k + rank as f32 + 1.0);

        // BOOST: Exact title matches get multiplier boost
        if *keyword_score >= EXACT_TITLE_MATCH_SCORE {
            rfr_score *= EXACT_MATCH_BOOST_MULTIPLIER;
            info!(
                "Exact title match detected for movie ID {}, boosting score",
                movie_id
            );
        }

        *scores
            .entry(*movie_id)
            .or_insert(0.0) += rfr_score;
    }

    // Add scores from vector search
    for (rank, movie_id) in vector_ranks
        .iter()
        .enumerate()
    {
        let score = 1.0 / (k + rank as f32 + 1.0);
        *scores
            .entry(*movie_id)
            .or_insert(0.0) += score;
    }

    // Sort by RRF score descending
    let mut ranked: Vec<(
        i32,
        f32,
    )> = scores
        .into_iter()
        .collect();
    ranked.sort_by(
        |a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
        },
    );

    ranked
}

/// Performs hybrid search combining keyword and semantic vector search
/// Returns (results, enhanced_query) where results is Vec<(movie_id, rfr_score)>
#[instrument(skip(movies))]
pub async fn hybrid_search(
    query: &str,
    movies: Arc<Vec<FullMovie>>,
    limit: usize,
    disable_enhancement: bool,
) -> (
    Vec<(
        i32,
        f32,
    )>,
    String,
) {
    let start_time = std::time::Instant::now();

    // Phase 1: Run entity extraction and query enhancement in parallel
    let movies_clone = Arc::clone(&movies);
    let query_owned = query.to_string();
    let query_for_enhancement = query.to_string();

    let (entity_result, enhanced_query) = tokio::join!(
        // Entity extraction task
        async move {
            extract_entities(
                &query_owned,
                &movies_clone,
            )
            .await
        },
        // Query enhancement task
        async move {
            if disable_enhancement {
                info!("Query enhancement disabled, using original query");
                query_for_enhancement
            } else {
                ai_chat::query_enhancement::enhance_query_for_embedding(&query_for_enhancement)
                    .await
            }
        }
    );

    let (titles, actors, genres) = entity_result;
    info!(
        "Entity extraction results - titles: {:?}, actors: {:?}, genres: {:?}",
        titles, actors, genres
    );
    info!(
        "Enhanced query: {}",
        enhanced_query
    );

    let criteria = SearchCriteria {
        titles,
        actors,
        genres,
    };

    // Phase 2: Run keyword search and vector search in parallel
    let movies_for_keyword = Arc::clone(&movies);
    let enhanced_query_clone = enhanced_query.clone();

    let (keyword_results, vector_results) = tokio::join!(
        // Keyword search task
        async move {
            let keyword_start = std::time::Instant::now();
            let results = keyword_search(
                &movies_for_keyword,
                &criteria,
                limit * 2,
            );
            info!(
                "Keyword search found {} results in {:.2?}",
                results.len(),
                keyword_start.elapsed()
            );

            if results.is_empty() {
                info!("Keyword search returned no results - will rely on vector search only");
            } else {
                info!(
                    "Top keyword matches (ID, score): {:?}",
                    results
                        .iter()
                        .take(5)
                        .collect::<Vec<_>>()
                );
            }

            results
        },
        // Vector search task
        async move {
            let vector_start = std::time::Instant::now();
            let results = match embedding(&enhanced_query_clone).await {
                Ok(embedding_vec) => {
                    match database::search_movies(
                        embedding_vec,
                        (limit * 2) as i64,
                    )
                    .await
                    {
                        Ok(movies) => {
                            let ids: Vec<i32> = movies
                                .into_iter()
                                .map(|m| m.id)
                                .collect();
                            info!(
                                "Vector search found {} results in {:.2?}",
                                ids.len(),
                                vector_start.elapsed()
                            );
                            ids
                        }
                        Err(e) => {
                            error!(
                                "Vector search failed: {:#?}",
                                e
                            );
                            Vec::new()
                        }
                    }
                }
                Err(e) => {
                    error!(
                        "Failed to generate embedding: {:#?}",
                        e
                    );
                    Vec::new()
                }
            };
            results
        }
    );

    // Fuse results using RRF - just return IDs and scores
    let final_results = if !keyword_results.is_empty() && !vector_results.is_empty() {
        info!("Fusing keyword and vector results with RRF");
        let fused = reciprocal_rank_fusion(
            keyword_results,
            vector_results,
            RRF_K_VALUE,
        );
        fused
            .into_iter()
            .take(limit)
            .collect()
    } else if !keyword_results.is_empty() {
        info!("Using keyword-only results");
        // Calculate position-based scores (1.0 for rank 0, decreasing)
        // Keep keyword scores for exact match boosting
        keyword_results
            .into_iter()
            .take(limit)
            .enumerate()
            .map(
                |(rank, (id, keyword_score))| {
                    let mut score = 1.0 / (RRF_K_VALUE + rank as f32 + 1.0);
                    // Boost exact matches even in keyword-only mode
                    if keyword_score >= EXACT_TITLE_MATCH_SCORE {
                        score *= EXACT_MATCH_BOOST_MULTIPLIER;
                    }
                    (
                        id, score,
                    )
                },
            )
            .collect()
    } else if !vector_results.is_empty() {
        info!("Using vector-only results");
        // Calculate position-based scores (1.0 for rank 0, decreasing)
        vector_results
            .into_iter()
            .take(limit)
            .enumerate()
            .map(
                |(rank, id)| {
                    (
                        id,
                        1.0 / (RRF_K_VALUE + rank as f32 + 1.0),
                    )
                },
            )
            .collect()
    } else {
        info!("No search results found");
        Vec::new()
    };

    info!(
        "Hybrid search completed in {:.2?}, returning {} results",
        start_time.elapsed(),
        final_results.len()
    );

    (
        final_results,
        enhanced_query,
    )
}

/// Extracts all unique entities from a list of movies
async fn extract_entities_from_movies(
    movies: &[FullMovie],
) -> (
    Vec<String>,
    Vec<String>,
    Vec<String>,
) {
    use std::collections::HashSet;
    use std::time::Instant;

    let start_time = Instant::now();
    let total_movies = movies.len();
    info!(
        "Starting entity extraction for {} movies",
        total_movies
    );

    let mut all_genres = HashSet::new();
    let mut all_actors = HashSet::new();
    let mut all_directors = HashSet::new();

    for movie in movies {
        all_genres.extend(
            movie
                .genres
                .iter()
                .map(|g| g.to_lowercase()),
        );
        all_actors.extend(
            movie
                .actors
                .iter()
                .map(
                    |a| {
                        a.name
                            .to_lowercase()
                    },
                ),
        );
        if let Some(director) = &movie.director {
            all_directors.insert(
                director
                    .name
                    .to_lowercase(),
            );
        }
    }

    let elapsed = start_time.elapsed();
    info!(
        "Extracted entities from {} movies in {:.2?} ({:.0} movies/sec)",
        total_movies,
        elapsed,
        total_movies as f64 / elapsed.as_secs_f64()
    );

    (
        all_genres
            .into_iter()
            .collect(),
        all_actors
            .into_iter()
            .collect(),
        all_directors
            .into_iter()
            .collect(),
    )
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

    // Wrap in Arc for cheap cloning
    let arc_dvds = Arc::new(dvds);

    // Use hybrid search (keyword + vector with RRF fusion) - pass Arc::clone
    // Enable enhancement for AI chat (disable_enhancement = false)
    info!(
        "Performing hybrid search for query: {}",
        question
    );
    let (movie_results, _enhanced_query) = hybrid_search(
        &question,
        Arc::clone(&arc_dvds),
        15,
        false,
    )
    .await;

    let full_movies = if !movie_results.is_empty() {
        // Create lookup map
        let movies_map: std::collections::HashMap<i32, &FullMovie> = arc_dvds
            .iter()
            .map(
                |movie| {
                    (
                        movie.id, movie,
                    )
                },
            )
            .collect();

        // Look up movies by ID (clone only for AI processing)
        movie_results
            .into_iter()
            .filter_map(
                |(id, _score)| {
                    movies_map
                        .get(&id)
                        .map(|&m| m.clone())
                },
            )
            .collect()
    } else {
        info!("No search results, using all movies");
        Arc::try_unwrap(arc_dvds).unwrap_or_else(|arc| (*arc).clone())
    };

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
                    temperature,
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
