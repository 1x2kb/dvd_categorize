use std::{error::Error, fs::File};

use database::FullMovie;
use dotenvy::dotenv;
use log::{debug, error, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    info!("Starting csv reader");

    #[cfg(debug_assertions)]
    dotenv()
        .ok()
        .expect("Failed to run env reader");
    #[cfg(debug_assertions)]
    debug!("Loaded env variables");

    info!("Opening file");
    // Try Docker mount path first, then local development path
    let csv_path = if std::path::Path::new("/app/movies.csv").exists() {
        "/app/movies.csv"
    } else {
        "./movies.csv"
    };
    info!(
        "Using CSV file: {}",
        csv_path
    );
    let reader = File::open(csv_path)?;
    info!("File opened");

    info!("Parsing csv");
    let full_movies: Vec<FullMovie> = csv_utils::parse_csv(reader)?;
    info!("Parsed csv");
    debug!(
        "{} movies read in",
        full_movies.len()
    );

    let result = database::insert_full_movies(full_movies).await;

    match result {
        Ok(_) => info!("Finished insert"),
        Err(e) => error!(
            "{}",
            e
        ),
    }

    Ok(())
}
