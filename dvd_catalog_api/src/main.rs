use std::env;

use axum::{
    routing::{get, post},
    Router,
};
use dotenvy::dotenv;
use dvd_catalog::*;
extern crate pretty_env_logger;
#[macro_use]
extern crate log;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    dotenv()
        .ok()
        .expect("Failed to run env reader");

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
}
