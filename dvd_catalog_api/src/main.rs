use std::{env, sync::Arc};

use axum::{
    http::{self, Method},
    routing::{get, post},
    Router,
};
use database::FullMovie;
use dotenvy::dotenv;
use dvd_catalog::*;
use log::{error, info, warn};
use tower_http::cors::{Any, CorsLayer};

#[tokio::main]
async fn main() {
    // Initialize logger with timestamp and module info
    let env = env_logger::Env::default()
        .filter_or(
            "RUST_LOG", "info",
        )
        .write_style_or(
            "RUST_LOG_STYLE",
            "always",
        );

    env_logger::Builder::from_env(env)
        .format_timestamp(Some(env_logger::TimestampPrecision::Millis))
        .format_module_path(false)
        .init();

    info!("Starting DVD Catalog API");

    // Only load .env file in debug mode (development)
    #[cfg(debug_assertions)]
    {
        dotenv()
            .ok()
            .expect("Failed to run env reader");
    }

    // Load movies into cache state on startup
    let movies = match database::get_movies().await {
        Ok(movies) => {
            info!("Successfully loaded {} movies into cache", movies.len());
            movies
        }
        Err(e) => {
            error!("Failed to load movies: {}", e);
            Vec::new()
        }
    };

    // Create the router with the initial movie data
    let app = init_router(movies);

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

fn init_router(movies: Vec<FullMovie>) -> Router {
    // Create state with the provided movies wrapped in Arc<RwLock<>>
    let state = CacheState { 
        movies: Arc::new(tokio::sync::RwLock::new(movies)) 
    };

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
            "/csv/parse",
            post(parse_csv)
        )
        .with_state(state);

    // Create a router for stateless endpoints
    let stateless_router = Router::new()
        .route(
            "/dvd/{id}",
            get(get_dvd),
        )
        .route(
            "/csv/preview",
            post(preview_csv),
        )
        .route(
            "/",
            get(hello_world),
        );

    // Merge the routers
    Router::new()
        .merge(stateful_router)
        .merge(stateless_router)
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_headers([http::header::CONTENT_TYPE])
                .allow_methods([Method::GET, Method::POST]),
        )
}
