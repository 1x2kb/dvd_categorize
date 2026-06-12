use std::collections::HashSet;
use std::sync::Arc;

use ai_chat::OllamaClient;
use axum::extract::{Json, State};
use axum_macros::debug_handler;
use categorizer_utilities::strip_punctuation;
use database::{
    question::AiAction,
    traits::GetAllMovies,
    FullMovie, PostgresMovieRepository,
};
#[cfg(feature = "ai")]
use database::traits::SearchMoviesByEmbedding;
use log::{error, info};
use tracing::instrument;

use crate::embedding;

// Search scoring constants
const RRF_K_VALUE: f32 = 60.0; // Standard reciprocal rank fusion constant
const EXACT_TITLE_MATCH_SCORE: f32 = 100.0;
const EXACT_MATCH_BOOST_MULTIPLIER: f32 = 10.0;

// Stop words for entity extraction
const STOP_PHRASES: &[&str] = &[
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
    let query_normalized = strip_punctuation(&query_lower);
    let mut titles = Vec::new();
    let mut actors = HashSet::new();
    let mut genres = HashSet::new();

    // First, look for exact matches of known entities in the query
    for genre in &all_genres {
        let genre_normalized = strip_punctuation(&genre.to_lowercase());
        let genre_plural = format!(
            "{}s",
            genre_normalized
        );
        // Match whole word to avoid partial matches
        if query_normalized
            .split_whitespace()
            .any(|w| w == genre_normalized || w == genre_plural)
        {
            genres.insert(genre_normalized);
        }
    }

    // Look for actors and directors
    let query_words: Vec<&str> = query_normalized
        .split_whitespace()
        .collect();

    for entity in all_actors
        .iter()
        .chain(all_directors.iter())
    {
        let entity_normalized = strip_punctuation(&entity.to_lowercase());
        let name_parts: Vec<&str> = entity_normalized
            .split_whitespace()
            .collect();

        // Check for exact match first (most reliable)
        let exact_match = if query_normalized == entity_normalized {
            true
        } else {
            // Check if entity appears as whole word(s) in query
            let padded_query = format!(
                " {} ",
                query_normalized
            );
            let padded_entity = format!(
                " {} ",
                entity_normalized
            );
            padded_query.contains(&padded_entity)
        };

        if exact_match {
            actors.insert(entity_normalized.clone());
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
                    actors.insert(entity_normalized.clone());
                    continue;
                }
            } else {
                // Partial query (e.g., just "Adam"): match if any significant part matches
                if !matching_parts.is_empty() {
                    actors.insert(entity_normalized.clone());
                    continue;
                }
            }
        }

        // For single-word names or as a fallback, check for standalone word match
        if name_parts.len() == 1 && name_parts[0].len() > 2 {
            let word = name_parts[0];
            let word_match = query_normalized
                .split_whitespace()
                .any(|w| w == word)
                || query_normalized == word
                || query_normalized.starts_with(&format!("{word} "))
                || query_normalized.ends_with(&format!(" {word}"));

            if word_match {
                actors.insert(entity_normalized.clone());
            }
        }
    }

    // Extract potential title by removing matched entities
    let mut remaining_query = query_normalized.clone();
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
    for phrase in STOP_PHRASES {
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
    let remaining_trimmed = remaining_query.trim();
    if !remaining_trimmed.is_empty() {
        titles.push(remaining_trimmed.to_string());
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

        let movie_title_normalized = strip_punctuation(&movie_title_lower);
        let title_normalized = strip_punctuation(&title_lower);

        if movie_title_normalized == title_normalized {
            score += EXACT_TITLE_MATCH_SCORE; // Exact match
        } else if movie_title_normalized.contains(&title_normalized) {
            score += 50.0; // Partial substring match
        } else if title_normalized.contains(&movie_title_normalized) {
            score += 30.0; // Query contains movie title
        } else {
            // Check if movie title starts with the query (e.g., "RoboCop" starts with "Robot")
            if movie_title_normalized.starts_with(&title_normalized) {
                score += 45.0;
            } else {
                // Check word-level matching for compound words
                // Split on common delimiters and check if any word starts with query
                let movie_words: Vec<&str> = movie_title_normalized
                    .split_whitespace()
                    .collect();

                for word in movie_words {
                    if word.starts_with(&title_normalized) {
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
        let actor_normalized = strip_punctuation(&actor_lower);
        for movie_actor in &movie.actors {
            let movie_actor_lower = movie_actor
                .name
                .to_lowercase();
            let movie_actor_normalized = strip_punctuation(&movie_actor_lower);
            if movie_actor_normalized == actor_normalized
                || movie_actor_normalized.contains(&actor_normalized)
            {
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
            let actor_normalized = strip_punctuation(&actor_lower);
            let director_lower = director
                .name
                .to_lowercase();
            let director_normalized = strip_punctuation(&director_lower);
            if director_normalized == actor_normalized
                || director_normalized.contains(&actor_normalized)
            {
                score += 25.0;
                break;
            }
        }
    }

    // Genre matching
    for genre in &criteria.genres {
        let genre_lower = genre.to_lowercase();
        let genre_normalized = strip_punctuation(&genre_lower);
        for movie_genre in &movie.genres {
            let movie_genre_lower = movie_genre.to_lowercase();
            let movie_genre_normalized = strip_punctuation(&movie_genre_lower);
            if movie_genre_normalized == genre_normalized {
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

/// Performs text-only keyword search without AI enhancement or vector operations
/// Uses database-level ILIKE matching with scoring
#[instrument(skip(repo))]
async fn text_only_search(
    query: &str,
    repo: &PostgresMovieRepository,
    limit: usize,
) -> (
    Vec<(
        i32,
        f32,
    )>,
    String,
) {
    let _start_time = std::time::Instant::now();
    info!("Text-only search mode");

    // Use database-level text search
    let db_start = std::time::Instant::now();
    let matching_movies = match repo
        .search_movies_by_text(
            query,
            limit as i64,
        )
        .await
    {
        Ok(results) => {
            info!(
                "DB text search found {} results in {:.2?}",
                results.len(),
                db_start.elapsed()
            );
            results
                .into_iter()
                .map(
                    |(movie, score)| {
                        (
                            movie.id, score,
                        )
                    },
                )
                .collect()
        }
        Err(e) => {
            error!(
                "DB text search failed: {:?}",
                e
            );
            Vec::new()
        }
    };

    (
        matching_movies,
        query.to_string(),
    )
}

/// Performs vector-only search with query enhancement
#[cfg(feature = "ai")]
#[instrument(skip(repo))]
async fn vector_only_search(
    query: &str,
    repo: &PostgresMovieRepository,
    limit: usize,
    disable_enhancement: bool,
    model: Option<&str>,
) -> (
    Vec<(
        i32,
        f32,
    )>,
    String,
) {
    let start_time = std::time::Instant::now();
    info!("Vector-only search mode");

    // Run query enhancement
    let enhanced_query = if disable_enhancement {
        info!("Query enhancement disabled, using original query");
        query.to_string()
    } else {
        ai_chat::query_enhancement::enhance_query_for_embedding(
            query, model,
        )
        .await
    };

    info!(
        "Enhanced query: {}",
        enhanced_query
    );

    // Run vector search only
    let vector_start = std::time::Instant::now();
    let vector_results = match embedding(&enhanced_query).await {
        Ok(embedding_vec) => {
            match repo
                .search_by_embedding(
                    embedding_vec,
                    (limit * 2) as i64,
                )
                .await
            {
                Ok(movies_from_db) => {
                    let ids: Vec<i32> = movies_from_db
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

    // Return normalized vector scores
    let final_results: Vec<(
        i32,
        f32,
    )> = vector_results
        .into_iter()
        .take(limit)
        .enumerate()
        .map(
            |(rank, id)| {
                let score = 1.0 / (1.0 + rank as f32);
                (
                    id, score,
                )
            },
        )
        .collect();

    info!(
        "Vector search completed in {:.2?}, returning {} results",
        start_time.elapsed(),
        final_results.len()
    );

    (
        final_results,
        enhanced_query,
    )
}

/// Performs hybrid search combining both keyword and vector search with RRF fusion
#[cfg(feature = "ai")]
#[instrument(skip(movies, repo))]
async fn hybrid_both_search(
    query: &str,
    movies: Arc<Vec<FullMovie>>,
    repo: &PostgresMovieRepository,
    limit: usize,
    disable_enhancement: bool,
    model: Option<&str>,
) -> (
    Vec<(
        i32,
        f32,
    )>,
    String,
) {
    let start_time = std::time::Instant::now();
    info!("Hybrid search mode (both text and vector)");

    let movies_clone = Arc::clone(&movies);
    let query_owned = query.to_string();
    let query_for_enhancement = query.to_string();

    // Run entity extraction and query enhancement in parallel
    let (entity_result, enhanced_query) = tokio::join!(
        async move {
            extract_entities(
                &query_owned,
                &movies_clone,
            )
            .await
        },
        async move {
            if disable_enhancement {
                info!("Query enhancement disabled, using original query");
                query_for_enhancement
            } else {
                ai_chat::query_enhancement::enhance_query_for_embedding(
                    &query_for_enhancement,
                    model,
                )
                .await
            }
        }
    );

    let (titles, actors, genres) = entity_result;
    info!(
        "Entity extraction - titles: {:?}, actors: {:?}, genres: {:?}",
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
    let movies_for_keyword = Arc::clone(&movies);
    let enhanced_query_clone = enhanced_query.clone();

    // Run keyword and vector search in parallel
    let (keyword_results, vector_results) = tokio::join!(
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
            if !results.is_empty() {
                info!(
                    "Top keyword matches: {:?}",
                    results
                        .iter()
                        .take(5)
                        .collect::<Vec<_>>()
                );
            }
            results
        },
        async move {
            let vector_start = std::time::Instant::now();
            match embedding(&enhanced_query_clone).await {
                Ok(embedding_vec) => {
                    match repo
                        .search_by_embedding(
                            embedding_vec,
                            (limit * 2) as i64,
                        )
                        .await
                    {
                        Ok(movies_from_db) => {
                            let ids: Vec<i32> = movies_from_db
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
            }
        }
    );

    // Fuse results using RRF
    let final_results: Vec<(
        i32,
        f32,
    )> = if !keyword_results.is_empty() && !vector_results.is_empty() {
        info!("Fusing keyword and vector results with RRF");
        reciprocal_rank_fusion(
            keyword_results,
            vector_results,
            RRF_K_VALUE,
        )
        .into_iter()
        .take(limit)
        .collect()
    } else if !keyword_results.is_empty() {
        info!("Using keyword-only results (vector search failed)");
        keyword_results
            .into_iter()
            .take(limit)
            .enumerate()
            .map(
                |(rank, (id, keyword_score))| {
                    let mut score = 1.0 / (RRF_K_VALUE + rank as f32 + 1.0);
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
        info!("Using vector-only results (keyword search failed)");
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

/// Performs structured query search using AI to parse the query and dynamic Diesel queries
#[instrument]
async fn structured_query_search(
    query: &str,
    limit: usize,
    model: Option<&str>,
) -> (
    Vec<(
        i32,
        f32,
    )>,
    String,
) {
    let start_time = std::time::Instant::now();
    info!("Structured query search mode");

    match ai_chat::structured_query_parser::parse_query_to_structured(
        query, model,
    )
    .await
    {
        Ok(structured_query) => {
            info!(
                "Parsed structured query: {:?}",
                structured_query
            );

            let structured_query_str = format!(
                "{:?}",
                structured_query
            );

            let pool = match database::get_connection_pool().await {
                Ok(pool) => pool,
                Err(e) => {
                    error!(
                        "Failed to get database pool: {:?}",
                        e
                    );
                    return (
                        Vec::new(),
                        query.to_string(),
                    );
                }
            };
            match pool
                .get()
                .await
            {
                Ok(mut conn) => {
                    match database::structured_search::search_movies_structured(
                        &structured_query,
                        &mut conn,
                    )
                    .await
                    {
                        Ok(movies) => {
                            info!(
                                "Structured search found {} movies in {:.2?}",
                                movies.len(),
                                start_time.elapsed()
                            );

                            let results: Vec<(
                                i32,
                                f32,
                            )> = movies
                                .into_iter()
                                .take(limit)
                                .enumerate()
                                .map(
                                    |(rank, movie)| {
                                        let score = 1.0 / (1.0 + rank as f32);
                                        (
                                            movie.id, score,
                                        )
                                    },
                                )
                                .collect();

                            (
                                results,
                                structured_query_str,
                            )
                        }
                        Err(e) => {
                            error!(
                                "Structured search database error: {:?}",
                                e
                            );
                            (
                                Vec::new(),
                                query.to_string(),
                            )
                        }
                    }
                }
                Err(e) => {
                    error!(
                        "Failed to get database connection: {:?}",
                        e
                    );
                    (
                        Vec::new(),
                        query.to_string(),
                    )
                }
            }
        }
        Err(e) => {
            error!(
                "Failed to parse query to structured format: {}",
                e
            );
            (
                Vec::new(),
                query.to_string(),
            )
        }
    }
}

/// Dispatches to the appropriate search function based on search mode
/// Returns (results, enhanced_query) where results is Vec<(movie_id, score)>
/// Score interpretation depends on search_mode:
/// - Text: raw keyword score
/// - Vector: cosine similarity (0-1, higher is better)
/// - Both: RRF score combining both methods
/// - Structured: AI-parsed query with dynamic Diesel queries
#[instrument(skip(movies, repo))]
pub async fn hybrid_search(
    query: &str,
    movies: Arc<Vec<FullMovie>>,
    repo: &PostgresMovieRepository,
    limit: usize,
    disable_enhancement: bool,
    search_mode: models::SearchMode,
    model: Option<&str>,
) -> (
    Vec<(
        i32,
        f32,
    )>,
    String,
) {
    match search_mode {
        models::SearchMode::Text => {
            text_only_search(
                query, repo, limit,
            )
            .await
        }
        models::SearchMode::Vector => {
            #[cfg(feature = "ai")]
            {
                vector_only_search(
                    query,
                    repo,
                    limit,
                    disable_enhancement,
                    model,
                )
                .await
            }
            #[cfg(not(feature = "ai"))]
            {
                error!("Vector search requires ai feature but it was not enabled");
                (Vec::new(), query.to_string())
            }
        }
        models::SearchMode::Both => {
            #[cfg(feature = "ai")]
            {
                hybrid_both_search(
                    query,
                    movies,
                    repo,
                    limit,
                    disable_enhancement,
                    model,
                )
                .await
            }
            #[cfg(not(feature = "ai"))]
            {
                error!("Hybrid search requires ai feature but it was not enabled");
                text_only_search(query, repo, limit).await
            }
        }
        models::SearchMode::Structured => {
            structured_query_search(
                query, limit, model,
            )
            .await
        }
    }
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
#[instrument(skip(state))]
#[debug_handler]
pub async fn chat(
    State(state): State<crate::DbState>,
    Json(action): Json<AiAction>,
) -> Json<AiAction> {
    let repo = &state.movie_repo;

    let dvds = repo
        .get_all()
        .await
        .unwrap_or_default();

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
        repo,
        15,
        false,
        models::SearchMode::Both,
        None,
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
    let ollama_client = match ai_chat::make_ollama_client() {
        Ok(o) => o,
        Err(e) => {
            error!(
                "Failed to create Ollama client: {}",
                e
            );
            return Json(
                AiAction {
                    uuid,
                    action: "There was an error connecting to the AI service.".to_string(),
                    model,
                    temperature: None,
                },
            );
        }
    };
    let result = ai_chat::ai_message(
        Arc::new(full_movies),
        Arc::new(
            OllamaClient {
                ollama_client,
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
