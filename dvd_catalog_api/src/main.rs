use std::{env, sync::Arc};

use axum::{
    http::{self, Method},
    routing::{get, post},
    Router,
};
use database::{traits::GetAllMovies, FullMovie, PostgresMovieRepository};
use dotenvy::dotenv;
use dvd_catalog::*;
use log::{error, info, warn};
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() {
    // Initialize JSON logging
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,dvd_catalog=debug,database=debug,ai_chat=debug")),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .json()
                .with_current_span(false),
        )
        .init();

    info!("Starting DVD Catalog API");

    // Only load .env file in debug mode (development)
    #[cfg(debug_assertions)]
    {
        dotenv().expect("Failed to run env reader");
    }

    // Load movies into cache state on startup
    let db_pool = PostgresMovieRepository::from_env()
        .await
        .expect("Failed to create DB pool");
    let load_result = db_pool
        .get_all()
        .await;
    let movies = match load_result {
        Ok(movies) => {
            info!(
                "Successfully loaded {} movies into cache",
                movies.len()
            );
            movies
        }
        Err(e) => {
            error!(
                "Failed to load movies: {}",
                e
            );
            Vec::new()
        }
    };

    // Create the router with the initial movie data
    let app = init_router(
        movies, db_pool,
    );

    let connection = get_host();
    info!(
        "Starting server on {}",
        &connection
    );

    let listener = tokio::net::TcpListener::bind(&connection)
        .await
        .unwrap();
    axum::serve(
        listener, app,
    )
    .await
    .expect("Failed to serve api!");
}

fn get_host() -> String {
    let host = match env::var("server_host") {
        Ok(host) => host,
        Err(_) => {
            warn!("server_host env var not defined defaulting to 0.0.0.0");
            "0.0.0.0".to_string()
        }
    };

    let port = match env::var("server_port") {
        Ok(port) => port,
        Err(_) => {
            warn!("server_port env var not defined, defaulting to 3000");
            "3000".to_string()
        }
    };

    format!("{host}:{port}")
}

fn init_router(movies: Vec<FullMovie>, db_pool: PostgresMovieRepository) -> Router {
    // Create state with the provided movies wrapped in Arc<RwLock<>>
    let state = CacheState {
        movies: Arc::new(tokio::sync::RwLock::new(movies)),
    };

    // Create DB state for chat history
    let db_state = DbState { pool: db_pool };

    // Create a router for endpoints that need CacheState
    let stateful_router = Router::new()
        .route(
            "/dvd",
            get(get_dvds),
        )
        .route(
            "/ai/dvd-match",
            post(get_matching_movies),
        )
        .route(
            "/movie/location",
            post(update_movie_location),
        )
        .route(
            "/csv/parse",
            post(parse_csv),
        )
        .route(
            "/csv/export",
            get(export_csv),
        )
        .route(
            "/ai/validate-titles",
            post(validate_titles),
        )
        .route(
            "/ai/generate-movies-stream",
            post(generate_movies_stream),
        )
        .with_state(state);

    // Create router for chat (tool-enabled non-streaming) and streaming, both need DbState
    let chat_stream_router = Router::new()
        .route(
            "/ai/chat",
            post(chat),
        )
        .route(
            "/ai/chat/stream",
            post(chat_stream),
        )
        .route(
            "/ai/chat/sessions",
            get(list_chat_sessions),
        )
        .route(
            "/ai/chat/sessions/{session_id}",
            get(get_session_history),
        )
        .route(
            "/dvd/structured-search",
            post(structured_search),
        )
        .route(
            "/saveMovie",
            post(save_movie),
        )
        .with_state(db_state);

    // Create a router for stateless endpoints
    let stateless_router = Router::new()
        .route(
            "/uniqueLocations()",
            get(unique_locations),
        )
        .route(
            "/location/{location}",
            get(get_movies_by_location),
        )
        .route(
            "/dvd/{id}",
            get(get_dvd),
        )
        .route(
            "/stats/overview",
            get(stats_overview),
        )
        .route(
            "/stats/movies-by-year",
            get(stats_movies_by_year),
        )
        .route(
            "/stats/genres",
            get(stats_genres),
        )
        .route(
            "/stats/top-actors",
            get(stats_top_actors),
        )
        .route(
            "/csv/preview",
            post(preview_csv),
        )
        .route(
            "/ai/pull-model",
            post(pull_ollama_model),
        )
        .route(
            "/ai/models",
            get(list_available_models),
        )
        .route(
            "/ai/generate-movies",
            post(generate_movies),
        )
        .route(
            "/ai/recent",
            get(get_recent_movies),
        )
        .route(
            "/dvd/recent-releases",
            get(get_recent_releases),
        )
        .route(
            "/dvd/random",
            get(get_random_movies),
        )
        .route(
            "/dvd/unknown-location",
            get(get_unknown_location_movies),
        )
        .route(
            "/",
            get(hello_world),
        );

    // Merge the routers
    Router::new()
        .merge(stateful_router)
        .merge(chat_stream_router)
        .merge(stateless_router)
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_headers([http::header::CONTENT_TYPE])
                .allow_methods([Method::GET, Method::POST]),
        )
}
