use std::env;

use axum::{
    http::{self, HeaderValue, Method},
    routing::{get, post},
    Router,
};
use dotenvy::dotenv;
use dvd_catalog::*;
use tower_http::cors::{Any, CorsLayer};
use log::{info, warn};

#[tokio::main]
async fn main() {
    // Initialize logger with timestamp and module info
    let env = env_logger::Env::default()
        .filter_or("RUST_LOG", "info")
        .write_style_or("RUST_LOG_STYLE", "always");
    
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

    let app = init_router();

    let connection = get_host();
    let listener = tokio::net::TcpListener::bind(connection)
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

fn init_router() -> Router {
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
            "/dvd",
            post(insert_dvd),
        )
        // .route(
        //     "/ai/chat",
        //     post(chat),
        // )
        .route(
            "/ai/dvd-match",
            post(get_matching_movies),
        )
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_headers([http::header::CONTENT_TYPE])
                .allow_methods([Method::GET, Method::POST]),
        )
}
