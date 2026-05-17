use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, sse::{Event, Sse}},
    Json,
};
use tokio_stream::StreamExt;
use axum_macros::debug_handler;
use database::{
    traits::{
        GetAllMovies, GetMovieById, GetMoviesByReleaseYear, GetRecentMovies, GetUniqueLocations,
        GetUnknownLocationMovies, InsertMovie, MoviesByLocation, RandomMovies,
    },
    FullMovie, PostgresMovieRepository, SearchRequest,
};
use log::{error, info};
use models::{CsvInput, ScoredMovie};
use ollama_rs::error::OllamaError;
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Instant};
use tracing::instrument;

// Movie search functionality module
pub mod movie_search;
pub use movie_search::extract_entities;

#[derive(Clone, Debug)]
pub struct CacheState {
    pub movies: Arc<tokio::sync::RwLock<Vec<FullMovie>>>,
}

#[derive(Clone)]
pub struct DbState {
    pub pool: database::PostgresMovieRepository,
}

#[derive(Debug, Deserialize)]
pub struct RandomMoviesQuery {
    #[serde(default = "default_count")]
    pub count: i64,
}

fn default_count() -> i64 {
    3
}

#[derive(Debug, Deserialize)]
pub struct RecentReleasesQuery {
    #[serde(default = "default_min_year")]
    pub min_year: i32,
    #[serde(default = "default_max_year")]
    pub max_year: i32,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_min_year() -> i32 {
    2020
}

fn default_max_year() -> i32 {
    2026 // Update this periodically or make it configurable
}

fn default_limit() -> i64 {
    50
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
    let result = match PostgresMovieRepository::from_env().await {
        Ok(mut repo) => repo.get_by_id(id).await.ok(),
        Err(e) => {
            error!("Failed to get repo: {}", e);
            None
        }
    };
    Json(result)
}

#[instrument]
#[debug_handler]
pub async fn insert_dvd(Json(dvd): Json<FullMovie>) -> Json<Option<FullMovie>> {
    let result = match PostgresMovieRepository::from_env().await {
        Ok(mut repo) => repo.insert(dvd).await.ok(),
        Err(e) => {
            error!("Failed to get repo: {}", e);
            None
        }
    };
    Json(result)
}

/// Non-streaming chat endpoint with tool calling.
///
/// Uses `ollama_rs::coordinator::Coordinator` so the model can invoke
/// `ai_tools` (filter-by-actor / genre / director, get-movie-details) which
/// self-call this same API over HTTP. Session + history persisted to DB —
/// mirrors the streaming endpoint but produces a single JSON response.
#[instrument(skip(db_state, request), fields(messages = request.messages.len()))]
#[debug_handler]
pub async fn chat(
    State(db_state): State<DbState>,
    Json(request): Json<models::ChatRequest>,
) -> Result<Json<models::ChatResponse>, (StatusCode, String)> {
    info!(
        "Processing tool-enabled chat request with {} message(s)",
        request.messages.len()
    );

    // Get or create session
    let session_id = match request
        .session_id
        .as_ref()
        .and_then(|s| uuid::Uuid::parse_str(s).ok())
    {
        Some(id) => id,
        None => db_state.pool.create_chat_session().await.map_err(|e| {
            error!("Failed to create session: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create session: {}", e),
            )
        })?,
    };

    // Load existing history from DB and seed the Coordinator's history
    let history_messages = db_state
        .pool
        .get_chat_history(session_id)
        .await
        .unwrap_or_default();

    // Tool-mode system prompt: results/recommendations scoped to user's library,
    // but the AI may enrich answers with real-world knowledge about those movies.
    // External movie references only when user explicitly requests them.
    let system_prompt = "You are a helpful assistant for a personal DVD movie collection. \
        When the user asks about their movies, actors, genres, or directors, \
        ALWAYS call the appropriate tool first (filter_by_actor, filter_by_genre, \
        filter_by_director, get_movie_details) to retrieve data from their collection. \
        You may use your own knowledge to enrich answers — e.g. describe a movie's plot, \
        discuss a director's style, or explain an actor's career — but any specific movie \
        titles you mention or recommend must come from the tool results unless the user \
        explicitly asks for suggestions outside their collection \
        (e.g. 'recommend something I don't own' or 'what should I buy next'). \
        If a tool returns no results, tell the user the collection has no matching movies. \
        Format responses with HTML only (<p>, <strong>, <em>, <ul>, <li>, <br>). \
        Do NOT use markdown. Do NOT use <script>, <iframe>, <style>, <form>, or event handlers.";

    // Seed the Coordinator's history with prior turns from DB (system prompt first).
    // New messages are passed as the `chat()` argument — coordinator's
    // send_chat_messages_with_history merges them in. Do NOT add new messages to
    // this history vec or they will be sent twice.
    let mut history: Vec<ollama_rs::generation::chat::ChatMessage> =
        vec![ollama_rs::generation::chat::ChatMessage::system(
            system_prompt.to_string(),
        )];
    for m in &history_messages {
        let msg = match m.role.as_str() {
            "user" => ollama_rs::generation::chat::ChatMessage::user(m.content.clone()),
            _ => ollama_rs::generation::chat::ChatMessage::assistant(m.content.clone()),
        };
        history.push(msg);
    }

    // New messages for this turn — passed to coordinator, NOT pre-added to history.
    let new_messages: Vec<ollama_rs::generation::chat::ChatMessage> = request
        .messages
        .iter()
        .map(|msg| match msg.role {
            models::Role::User => {
                ollama_rs::generation::chat::ChatMessage::user(msg.message.clone())
            }
            models::Role::Ai => {
                ollama_rs::generation::chat::ChatMessage::assistant(msg.message.clone())
            }
        })
        .collect();

    // Capture user content for DB persistence after the call
    let user_content_for_db: Vec<String> = request
        .messages
        .iter()
        .filter(|m| m.role == models::Role::User)
        .map(|m| m.message.clone())
        .collect();

    let model = request
        .model
        .clone()
        .unwrap_or_else(|| "phi3.5".to_string());
    info!("Using model: {}", model);

    // Ollama client
    let ollama_host = std::env::var("OLLAMA_HOST").unwrap_or_else(|_| "ollama".to_string());
    let ollama_port = std::env::var("OLLAMA_PORT").unwrap_or_else(|_| "11434".to_string());
    let ollama_url = format!("http://{}:{}", ollama_host, ollama_port);
    let ollama = ollama_rs::Ollama::from_url(ollama_url.parse().unwrap());

    // Build coordinator with all tools — each tool shares one ApiClient (cheap clone)
    let api_client = ai_tools::ApiClient::from_env();
    let mut coordinator = ollama_rs::coordinator::Coordinator::new(ollama, model, history)
        .add_tool(ai_tools::FilterByActorTool::new(api_client.clone()))
        .add_tool(ai_tools::FilterByGenreTool::new(api_client.clone()))
        .add_tool(ai_tools::FilterByDirectorTool::new(api_client.clone()))
        .add_tool(ai_tools::GetMovieDetailsTool::new(api_client));

    match coordinator.chat(new_messages).await {
        Ok(response) => {
            let content = response.message.content;
            info!("Successfully generated AI response ({} chars)", content.len());

            // Persist user turn(s) then assistant response — only after successful LLM call
            for user_content in &user_content_for_db {
                if let Err(e) = db_state
                    .pool
                    .save_chat_message(models::NewChatMessage {
                        session_id,
                        role: "user".to_string(),
                        content: user_content.clone(),
                    })
                    .await
                {
                    error!("Failed to save user message: {:?}", e);
                }
            }

            if let Err(e) = db_state
                .pool
                .save_chat_message(models::NewChatMessage {
                    session_id,
                    role: "assistant".to_string(),
                    content: content.clone(),
                })
                .await
            {
                error!("Failed to save AI response: {:?}", e);
            }

            Ok(Json(models::ChatResponse {
                message: content,
                session_id: Some(session_id.to_string()),
            }))
        }
        Err(e) => {
            error!("Failed to generate AI response: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to generate AI response: {}", e),
            ))
        }
    }
}

/// Extract movie search query from user message using Ollama
async fn extract_movie_query(message: &str) -> Option<models::StructuredQuery> {
    let ollama_host = std::env::var("OLLAMA_HOST").unwrap_or_else(|_| "ollama".to_string());
    let ollama_port = std::env::var("OLLAMA_PORT").unwrap_or_else(|_| "11434".to_string());
    let ollama_url = format!("http://{}:{}", ollama_host, ollama_port);
    let ollama = ollama_rs::Ollama::from_url(ollama_url.parse().unwrap());

    let prompt = format!(
        r#"Extract movie search parameters from this message. Return JSON with these fields (all optional):
- title_keywords: array of title keyword strings
- actors: array of actor name strings  
- genres: array of genre strings
- directors: array of director name strings
- description_keywords: array of description/plot keyword strings

If message is NOT about searching/finding movies, return {{"is_movie_query": false}}.
If it IS about movies, return {{"is_movie_query": true, ...other fields...}}.

Message: "{}"

Return only valid JSON, no explanation."#,
        message
    );

    let chat_request = ollama_rs::generation::chat::request::ChatMessageRequest::new(
        "phi3.5".to_string(),
        vec![ollama_rs::generation::chat::ChatMessage::user(prompt)],
    );

    match ollama.send_chat_messages(chat_request).await {
        Ok(response) => {
            let json_str = response.message.content.trim();
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(json_str) {
                if value.get("is_movie_query").and_then(|v| v.as_bool()) == Some(false) {
                    return None;
                }
                
                // Parse into StructuredQuery
                let query = models::StructuredQuery {
                    title_keywords: value.get("title_keywords")
                        .and_then(|v| v.as_array())
                        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                        .unwrap_or_default(),
                    actors: value.get("actors")
                        .and_then(|v| v.as_array())
                        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                        .unwrap_or_default(),
                    genres: value.get("genres")
                        .and_then(|v| v.as_array())
                        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                        .unwrap_or_default(),
                    directors: value.get("directors")
                        .and_then(|v| v.as_array())
                        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                        .unwrap_or_default(),
                    description_keywords: value.get("description_keywords")
                        .and_then(|v| v.as_array())
                        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                        .unwrap_or_default(),
                };
                
                // Only return if has some criteria
                if !query.title_keywords.is_empty() || !query.actors.is_empty() || !query.genres.is_empty() 
                    || !query.directors.is_empty() || !query.description_keywords.is_empty() {
                    return Some(query);
                }
            }
            None
        }
        Err(e) => {
            error!("Failed to extract movie query: {:?}", e);
            None
        }
    }
}

/// SSE streaming chat endpoint with RAG
#[instrument(skip(db_state))]
pub async fn chat_stream(
    State(db_state): State<DbState>,
    Json(request): Json<models::ChatRequest>,
) -> impl IntoResponse {
    info!(
        "Processing streaming chat request with {} message(s)",
        request.messages.len()
    );

    // Get or create session
    let session_id = match request.session_id.and_then(|s| uuid::Uuid::parse_str(&s).ok()) {
        Some(id) => id,
        None => match db_state.pool.create_chat_session().await {
            Ok(id) => id,
            Err(e) => {
                error!("Failed to create session: {:?}", e);
                let error_stream = async_stream::stream! {
                    yield Ok::<Event, std::convert::Infallible>(Event::default()
                        .event("error")
                        .data("Failed to create session"));
                };
                return Sse::new(error_stream).keep_alive(axum::response::sse::KeepAlive::default()).into_response();
            }
        }
    };

    // Load history from DB
    let history_messages = db_state.pool.get_chat_history(session_id).await.unwrap_or_default();
    
    // RAG: Extract movie query from user message
    let user_message = request.messages.first().map(|m| m.message.as_str()).unwrap_or("");
    let movie_context = if let Some(query) = extract_movie_query(user_message).await {
        info!("Detected movie query: {:?}", query);
        
        // Search movies from DB using structured query  
        let results = db_state.pool.search_structured(&query).await.unwrap_or_default();
        
        if results.is_empty() {
            "\n\nNo movies found matching the query.".to_string()
        } else {
            let movie_list = results.iter()
                .take(10)
                .map(|m| format!("- {} ({}) - Directed by {}, Starring: {}", 
                    m.name, 
                    m.release_year,
                    m.director.as_ref().map(|d| d.name.as_str()).unwrap_or("Unknown"),
                    m.actors.iter().map(|a| a.name.as_str()).collect::<Vec<_>>().join(", ")
                ))
                .collect::<Vec<_>>()
                .join("\n");
            
            format!("\n\nAvailable movies matching your query:\n{}", movie_list)
        }
    } else {
        String::new()
    };
    
    // RAG-mode system prompt: collection-focused with injected movie context.
    // Allows discussion of movies outside the library only when context suggests it
    // (e.g. buy recommendations based on existing collection).
    let system_prompt = format!(
        "You are a helpful assistant for a personal DVD movie collection. \
         Answer questions about the user's collection based on the movie data provided below. \
         If asked about movies not in their collection (e.g. 'what should I buy next'), \
         you may make external recommendations informed by what they already own. \
         Be conversational and concise. Format responses using HTML tags ONLY. \
         Do NOT use markdown (no **, no *, no backticks, no # headings, no - bullet syntax). \
         Allowed tags: <p>, <br>, <strong>, <em>, <ul>, <ol>, <li>, <blockquote>, <code>, <pre>. \
         Forbidden tags: <script>, <iframe>, <object>, <embed>, <link>, <style>, <form>, <input>, <button>. \
         No event handlers (onclick, onerror, onload, etc.).{}",
        movie_context
    );
    
    // Convert chat history to Ollama format (system + history + new messages)
    let messages: Vec<ollama_rs::generation::chat::ChatMessage> = std::iter::once(
        ollama_rs::generation::chat::ChatMessage::system(system_prompt)
    )
    .chain(
        history_messages.iter().map(|msg| {
            match msg.role.as_str() {
                "user" => ollama_rs::generation::chat::ChatMessage::user(msg.content.clone()),
                _ => ollama_rs::generation::chat::ChatMessage::assistant(msg.content.clone()),
            }
        })
    )
    .chain(
        request.messages.iter().map(|msg| {
            match msg.role {
                models::Role::User => ollama_rs::generation::chat::ChatMessage::user(msg.message.clone()),
                models::Role::Ai => ollama_rs::generation::chat::ChatMessage::assistant(msg.message.clone()),
            }
        })
    )
    .collect();

    // Get model name from request or use default
    let model = request.model.unwrap_or_else(|| "phi3.5".to_string());
    info!("Using model: {} for streaming", model);

    // Create Ollama client
    let ollama_host = std::env::var("OLLAMA_HOST").unwrap_or_else(|_| "ollama".to_string());
    let ollama_port = std::env::var("OLLAMA_PORT").unwrap_or_else(|_| "11434".to_string());
    let ollama_url = format!("http://{}:{}", ollama_host, ollama_port);

    let ollama = ollama_rs::Ollama::from_url(ollama_url.parse().unwrap());

    // Create chat request
    let chat_request = ollama_rs::generation::chat::request::ChatMessageRequest::new(
        model.clone(),
        messages,
    );

    // Save user messages to DB
    for msg in &request.messages {
        if let Err(e) = db_state.pool.save_chat_message(models::NewChatMessage {
            session_id,
            role: "user".to_string(),
            content: msg.message.clone(),
        }).await {
            error!("Failed to save user message: {:?}", e);
        }
    }

    // Get Ollama stream
    let ollama_stream = match ollama.send_chat_messages_stream(chat_request).await {
        Ok(stream) => stream,
        Err(e) => {
            error!("Failed to start stream: {:?}", e);
            // Return error as single SSE event
            let error_stream = async_stream::stream! {
                yield Ok::<Event, std::convert::Infallible>(Event::default()
                    .event("error")
                    .data(format!("Failed to start stream: {}", e)));
            };
            return Sse::new(error_stream).keep_alive(axum::response::sse::KeepAlive::default()).into_response();
        }
    };

    // Clone for saving
    let db_pool = db_state.pool.clone();
    
    // Convert to SSE stream with response accumulation
    let sse_stream = async_stream::stream! {
        tokio::pin!(ollama_stream);
        let mut accumulated_response = String::new();
        
        while let Some(chunk) = ollama_stream.next().await {
            match chunk {
                Ok(response) => {
                    // Log chunk content for debugging
                    if response.message.content.is_empty() {
                        info!("Received empty chunk from Ollama");
                    } else {
                        info!("Chunk content: {:?}", response.message.content);
                    }
                    
                    // Accumulate content
                    if !response.message.content.is_empty() {
                        accumulated_response.push_str(&response.message.content);
                        
                        // Send content chunk as-is (markdown parser will handle formatting)
                        yield Ok::<Event, std::convert::Infallible>(Event::default()
                            .event("message")
                            .data(response.message.content));
                    }
                    
                    // Send done event on final chunk
                    if response.done {
                        // Save AI response to DB
                        if !accumulated_response.is_empty() {
                            if let Err(e) = db_pool.save_chat_message(models::NewChatMessage {
                                session_id,
                                role: "assistant".to_string(),
                                content: accumulated_response.clone(),
                            }).await {
                                error!("Failed to save AI response: {:?}", e);
                            }
                        }
                        
                        // Send session ID
                        yield Ok::<Event, std::convert::Infallible>(Event::default()
                            .event("session")
                            .data(session_id.to_string()));
                        
                        yield Ok::<Event, std::convert::Infallible>(Event::default()
                            .event("done")
                            .data(""));
                        break;
                    }
                }
                Err(e) => {
                    error!("Stream error: {:?}", e);
                    yield Ok::<Event, std::convert::Infallible>(Event::default()
                        .event("error")
                        .data(format!("Stream error: {:?}", e)));
                    break;
                }
            }
        }
    };

    Sse::new(sse_stream).keep_alive(axum::response::sse::KeepAlive::default()).into_response()
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
pub async fn preview_csv(
    Json(value): Json<CsvInput>,
) -> Result<
    impl IntoResponse,
    (
        StatusCode,
        String,
    ),
> {
    let movies = match csv_utils::parse_csv(
        value
            .input
            .as_bytes(),
    ) {
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

    Ok((
        StatusCode::OK,
        Json(movies),
    ))
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
    let movies = match csv_utils::parse_csv(
        value
            .input
            .as_bytes(),
    ) {
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
    let refresh_result = match PostgresMovieRepository::from_env().await {
        Ok(mut repo) => repo.get_all().await,
        Err(e) => Err(e),
    };
    match refresh_result {
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
        search_request
            .model
            .as_deref(),
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
    let refresh_result = match PostgresMovieRepository::from_env().await {
        Ok(mut repo) => repo.get_all().await,
        Err(e) => Err(e),
    };
    match refresh_result {
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
            Ok(Json(models::AvailableModelsResponse { models }))
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

    let result = match PostgresMovieRepository::from_env().await {
        Ok(mut repo) => repo.get_recent(50).await,
        Err(e) => Err(e),
    };

    match result {
        Ok(movies) => {
            info!(
                "Found {} recent movies",
                movies.len()
            );
            let scored_movies: Vec<ScoredMovie> = movies
                .into_iter()
                .map(
                    |movie| ScoredMovie {
                        movie,
                        vector_score: 0.0,
                    },
                )
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

/// Get movies by release year range
#[instrument]
#[debug_handler]
pub async fn get_recent_releases(
    Query(params): Query<RecentReleasesQuery>,
) -> Json<Vec<ScoredMovie>> {
    info!(
        "Getting movies released between {} and {} (limit: {})",
        params.min_year, params.max_year, params.limit
    );

    let result = match PostgresMovieRepository::from_env().await {
        Ok(mut repo) => {
            repo.get_by_release_year(
                params.min_year,
                params.max_year,
                params.limit,
            )
            .await
        }
        Err(e) => Err(e),
    };

    match result {
        Ok(movies) => {
            info!(
                "Found {} movies released between {} and {}",
                movies.len(),
                params.min_year,
                params.max_year
            );
            let scored_movies: Vec<ScoredMovie> = movies
                .into_iter()
                .map(
                    |movie| ScoredMovie {
                        movie,
                        vector_score: 0.0,
                    },
                )
                .collect();
            Json(scored_movies)
        }
        Err(e) => {
            error!(
                "Failed to get movies by release year: {}",
                e
            );
            Json(Vec::new())
        }
    }
}

/// Get random movies from the database
#[instrument]
#[debug_handler]
pub async fn get_random_movies(Query(params): Query<RandomMoviesQuery>) -> Json<Vec<ScoredMovie>> {
    info!(
        "Getting {} random movies",
        params.count
    );

    let result = match PostgresMovieRepository::from_env().await {
        Ok(mut repo) => repo.get_random(params.count).await,
        Err(e) => Err(e),
    };

    match result {
        Ok(movies) => {
            info!(
                "Found {} random movies",
                movies.len()
            );
            let scored_movies: Vec<ScoredMovie> = movies
                .into_iter()
                .map(
                    |movie| ScoredMovie {
                        movie,
                        vector_score: 0.0,
                    },
                )
                .collect();
            Json(scored_movies)
        }
        Err(e) => {
            error!(
                "Failed to get random movies: {}",
                e
            );
            Json(Vec::new())
        }
    }
}

/// Get movies with unknown location from the database
#[instrument]
#[debug_handler]
pub async fn get_unknown_location_movies() -> Json<Vec<ScoredMovie>> {
    info!("Getting movies with unknown location");

    let result = match PostgresMovieRepository::from_env().await {
        Ok(mut repo) => repo.get_unknown_location(50).await,
        Err(e) => Err(e),
    };

    match result {
        Ok(movies) => {
            info!(
                "Found {} movies with unknown location",
                movies.len()
            );
            let scored_movies: Vec<ScoredMovie> = movies
                .into_iter()
                .map(
                    |movie| ScoredMovie {
                        movie,
                        vector_score: 0.0,
                    },
                )
                .collect();
            Json(scored_movies)
        }
        Err(e) => {
            error!(
                "Failed to get movies with unknown location: {}",
                e
            );
            Json(Vec::new())
        }
    }
}

#[instrument]
#[debug_handler]
pub async fn unique_locations() -> Json<Vec<String>> {
    info!("Getting unique list of all locations");
    let result = match PostgresMovieRepository::from_env().await {
        Ok(mut repo) => repo.unique_locations().await,
        Err(e) => Err(e),
    };

    result
        .map(Json)
        .unwrap_or_else(|e| {
            error!(
                "Failed to get unique locations: {}",
                e
            );
            Json(Vec::new())
        })
}

#[instrument]
#[debug_handler]
pub async fn get_movies_by_location(Path(location): Path<String>) -> Json<Vec<FullMovie>> {
    info!("Getting movies for location: {}", location);
    let result = match PostgresMovieRepository::from_env().await {
        Ok(mut repo) => repo.movies_by_location(&location).await,
        Err(e) => Err(e),
    };

    match result {
        Ok(movies) => {
            info!("Found {} movies for location '{}'", movies.len(), location);
            Json(movies)
        }
        Err(e) => {
            error!("Failed to get movies by location: {}", e);
            Json(Vec::new())
        }
    }
}

/// Get stats overview
#[instrument]
#[debug_handler]
pub async fn stats_overview() -> Json<models::StatsOverview> {
    info!("Getting stats overview");
    let result = match PostgresMovieRepository::from_env().await {
        Ok(mut repo) => repo.get_all().await,
        Err(e) => {
            error!("Failed to create repository: {}", e);
            return Json(models::StatsOverview {
                total_movies: 0,
                total_directors: 0,
                total_actors: 0,
            });
        }
    };

    match result {
        Ok(movies) => {
            use std::collections::HashSet;
            
            let total_movies = movies.len();
            let total_directors = movies.iter()
                .filter_map(|m| m.director.as_ref())
                .map(|d| d.name.clone())
                .collect::<HashSet<_>>()
                .len();
            let total_actors = movies.iter()
                .flat_map(|m| &m.actors)
                .map(|a| a.name.clone())
                .collect::<HashSet<_>>()
                .len();
            
            Json(models::StatsOverview {
                total_movies,
                total_directors,
                total_actors,
            })
        }
        Err(e) => {
            error!("Failed to get movies: {}", e);
            Json(models::StatsOverview {
                total_movies: 0,
                total_directors: 0,
                total_actors: 0,
            })
        }
    }
}

/// Get movies by year data (top 15 years by count)
#[instrument]
#[debug_handler]
pub async fn stats_movies_by_year() -> Json<models::BarChartData> {
    info!("Getting movies by year stats");
    let result = match PostgresMovieRepository::from_env().await {
        Ok(mut repo) => repo.get_all().await,
        Err(e) => {
            error!("Failed to create repository: {}", e);
            return Json(models::BarChartData {
                labels: vec![],
                values: vec![],
            });
        }
    };

    match result {
        Ok(movies) => {
            use std::collections::HashMap;
            
            let mut year_counts: HashMap<i32, usize> = HashMap::new();
            for movie in movies {
                if movie.release_year > 0 {
                    *year_counts.entry(movie.release_year).or_insert(0) += 1;
                }
            }
            
            let mut year_data: Vec<(i32, usize)> = year_counts.into_iter().collect();
            // Sort by count descending and take top 15
            year_data.sort_by(|(_, a), (_, b)| b.cmp(a));
            year_data.truncate(15);
            // Re-sort by year for display
            year_data.sort_by_key(|(year, _)| *year);
            
            let labels: Vec<String> = year_data.iter().map(|(y, _)| y.to_string()).collect();
            let values: Vec<f64> = year_data.iter().map(|(_, c)| *c as f64).collect();
            
            Json(models::BarChartData { labels, values })
        }
        Err(e) => {
            error!("Failed to get movies: {}", e);
            Json(models::BarChartData {
                labels: vec![],
                values: vec![],
            })
        }
    }
}

/// Get genre distribution data (top 10)
#[instrument]
#[debug_handler]
pub async fn stats_genres() -> Json<models::PieChartData> {
    info!("Getting genre distribution stats");
    let result = match PostgresMovieRepository::from_env().await {
        Ok(mut repo) => repo.get_all().await,
        Err(e) => {
            error!("Failed to create repository: {}", e);
            return Json(models::PieChartData { data: vec![] });
        }
    };

    match result {
        Ok(movies) => {
            use std::collections::HashMap;
            
            let mut genre_counts: HashMap<String, usize> = HashMap::new();
            for movie in movies {
                for genre in &movie.genres {
                    *genre_counts.entry(genre.clone()).or_insert(0) += 1;
                }
            }
            
            let mut data: Vec<(String, f64)> = genre_counts
                .into_iter()
                .map(|(genre, count)| (genre, count as f64))
                .collect();
            
            // Sort by count descending and take top 10
            data.sort_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
            data.truncate(10);
            
            Json(models::PieChartData { data })
        }
        Err(e) => {
            error!("Failed to get movies: {}", e);
            Json(models::PieChartData { data: vec![] })
        }
    }
}

/// Chat session with first query preview
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSessionWithPreview {
    pub id: i32,
    pub session_id: String,
    pub created_at: String,
    pub updated_at: String,
    pub first_query: Option<String>,
}

/// List all chat sessions ordered by most recent
#[instrument(skip(db_state))]
#[debug_handler]
pub async fn list_chat_sessions(
    State(db_state): State<DbState>,
) -> Result<Json<Vec<ChatSessionWithPreview>>, (StatusCode, String)> {
    info!("Listing chat sessions");
    
    match db_state.pool.list_chat_sessions().await {
        Ok(sessions) => {
            info!("Found {} chat sessions", sessions.len());
            
            let mut sessions_with_preview = Vec::new();
            for session in sessions {
                let first_query = db_state.pool.get_chat_history(session.session_id)
                    .await
                    .ok()
                    .and_then(|messages| {
                        messages.iter()
                            .find(|m| m.role == "user")
                            .map(|m| m.content.clone())
                    });
                
                sessions_with_preview.push(ChatSessionWithPreview {
                    id: session.id,
                    session_id: session.session_id.to_string(),
                    created_at: session.created_at.to_string(),
                    updated_at: session.updated_at.to_string(),
                    first_query,
                });
            }
            
            Ok(Json(sessions_with_preview))
        }
        Err(e) => {
            error!("Failed to list chat sessions: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to list chat sessions: {}", e),
            ))
        }
    }
}

/// Get chat history for a specific session
#[instrument(skip(db_state))]
#[debug_handler]
pub async fn get_session_history(
    State(db_state): State<DbState>,
    Path(session_id): Path<String>,
) -> Result<Json<Vec<models::ChatMessage>>, (StatusCode, String)> {
    info!("Getting chat history for session: {}", session_id);
    
    let session_uuid = uuid::Uuid::parse_str(&session_id)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid session ID: {}", e)))?;
    
    match db_state.pool.get_chat_history(session_uuid).await {
        Ok(messages) => {
            info!("Found {} messages for session {}", messages.len(), session_id);
            Ok(Json(messages))
        }
        Err(e) => {
            error!("Failed to get chat history: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get chat history: {}", e),
            ))
        }
    }
}

/// Structured search endpoint — filter by actors, genres, directors, title/description keywords.
/// Used by `ai_tools` over HTTP so tools avoid diesel-async (which isn't Sync for ollama-rs).
#[instrument(skip(db_state, query), fields(actors = query.actors.len(), genres = query.genres.len(), directors = query.directors.len()))]
#[debug_handler]
pub async fn structured_search(
    State(db_state): State<DbState>,
    Json(query): Json<models::StructuredQuery>,
) -> Json<Vec<FullMovie>> {
    info!(
        "Structured search: actors={:?} genres={:?} directors={:?} title={:?} desc={:?}",
        query.actors, query.genres, query.directors, query.title_keywords, query.description_keywords
    );

    match db_state.pool.search_structured(&query).await {
        Ok(movies) => {
            info!("Structured search returned {} movies", movies.len());
            Json(movies)
        }
        Err(e) => {
            error!("Structured search failed: {}", e);
            Json(Vec::new())
        }
    }
}

/// Get top actors data
#[instrument]
#[debug_handler]
pub async fn stats_top_actors() -> Json<models::BarChartData> {
    info!("Getting top actors stats");
    let result = match PostgresMovieRepository::from_env().await {
        Ok(mut repo) => repo.get_all().await,
        Err(e) => {
            error!("Failed to create repository: {}", e);
            return Json(models::BarChartData {
                labels: vec![],
                values: vec![],
            });
        }
    };

    match result {
        Ok(movies) => {
            use std::collections::HashMap;
            
            let mut actor_counts: HashMap<String, usize> = HashMap::new();
            for movie in movies {
                for actor in &movie.actors {
                    *actor_counts.entry(actor.name.clone()).or_insert(0) += 1;
                }
            }
            
            let mut actor_data: Vec<(String, usize)> = actor_counts.into_iter().collect();
            actor_data.sort_by(|(_, a), (_, b)| b.cmp(a));
            actor_data.truncate(10);
            
            let labels: Vec<String> = actor_data.iter().map(|(name, _)| name.clone()).collect();
            let values: Vec<f64> = actor_data.iter().map(|(_, count)| *count as f64).collect();
            
            Json(models::BarChartData { labels, values })
        }
        Err(e) => {
            error!("Failed to get movies: {}", e);
            Json(models::BarChartData {
                labels: vec![],
                values: vec![],
            })
        }
    }
}
