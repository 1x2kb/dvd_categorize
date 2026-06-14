use std::env;

use axum::{
    http::{self, Method},
    routing::{get, post},
    Router,
};
use database::MovieRepo;
use dotenvy::dotenv;
use dvd_catalog::*;
use log::{info, warn};
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() {
    // Initialize JSON logging
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env().unwrap_or_else(
                |_| EnvFilter::new("info,dvd_catalog=debug,database=debug,ai_chat=debug"),
            ),
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

    // Create DB pool
    let db_pool = MovieRepo::from_env()
        .await
        .expect("Failed to create DB pool");
    info!("Database pool created successfully");

    // Create the router
    let app = init_router(db_pool);

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

fn init_router(db_pool: MovieRepo) -> Router {
    let state = DbState { movie_repo: db_pool };

    Router::new()
        .route(
            "/",
            get(hello_world),
        )
        .route(
            "/dvd",
            get(get_dvds),
        )
        .route(
            "/dvd/{id}",
            get(get_dvd),
        )
        .route(
            "/saveMovie",
            post(save_movie),
        )
        .route(
            "/saveMovies",
            post(save_movies),
        )
        .route(
            "/movie/location",
            post(update_movie_location),
        )
        .route(
            "/location/{location}",
            get(get_movies_by_location),
        )
        .route(
            "/uniqueLocations()",
            get(unique_locations),
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
            "/ai/recent",
            get(get_recent_movies),
        )
        .route(
            "/ai/dvd-match",
            post(get_matching_movies),
        )
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
            "/ai/validate-titles",
            post(validate_titles),
        )
        .route(
            "/ai/generate-movies",
            post(generate_movies),
        )
        .route(
            "/ai/generate-movies-stream",
            post(generate_movies_stream),
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
            "/ai/structured-search",
            post(structured_search),
        )
        .route(
            "/csv/preview",
            post(preview_csv),
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
        .with_state(state)
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_headers([http::header::CONTENT_TYPE])
                .allow_methods([Method::GET, Method::POST]),
        )
}
