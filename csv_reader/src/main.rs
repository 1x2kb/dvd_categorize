use std::{collections::HashSet, error::Error, fs::File, io::Read};

use csv::Reader;
use database::{director, Actor, Director, FullMovie};
use dotenvy::dotenv;
use log::{debug, info};
use models::NewMovie;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all(serialize = "camelCase"))]
struct CsvRecord {
    #[serde(alias = "Title", alias = "TITLE")]
    title: String,
    #[serde(alias = "Description", alias = "DESCRIPTION")]
    description: Option<String>,
    #[serde(alias = "Actors", alias = "ACTORS")]
    actors: Option<String>,
    #[serde(alias = "Director", alias = "DIRECTOR")]
    director: Option<String>,
    #[serde(alias = "Genres", alias = "GENRES")]
    genres: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    info!("Starting csv reader");
    dotenv()
        .ok()
        .expect("Failed to run env reader");
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
    let full_movies: Vec<FullMovie> = parse_csv(reader)?;
    info!("Parsed csv");
    debug!(
        "{} movies read in",
        full_movies.len()
    );

    let (actors, genres, directors) = (
        get_unique_actors(&full_movies),
        get_unique_genres(&full_movies),
        get_unique_directors(&full_movies),
    );

    let mut connection = database::get_database_connection().await?;
    let actors = database::insert_actors(
        actors
            .into_iter()
            .map(|s| s.to_string()),
        &mut connection,
    )
    .await?;

    let directors = database::insert_directors(
        directors,
        &mut connection,
    )
    .await?;

    let embeddings: Vec<String> = full_movies
        .iter()
        .map(|movie| (movie.embedding_str()))
        .collect();

    let embeddings = ai_chat::get_embeddings(
        embeddings,
        "nomic-embed-text",
    )
    .await?;

    let movies = full_movies
        .iter()
        .enumerate()
        .map(
            |(i, movie)| {
                NewMovie {
                    name: movie
                        .name
                        .to_string(),
                    director_id: directors
                        .iter()
                        .find(
                            |(_, director_name)| {
                                movie
                                    .director
                                    .as_deref()
                                    .is_some_and(|d| &d.name == director_name)
                            },
                        ),
                    description: movie
                        .description
                        .clone(),
                    embedding: Some(embeddings[i]),
                };
            },
        )
        .collect();



    Ok(())
}

pub fn parse_csv(csv_data: impl Read) -> Result<Vec<FullMovie>, Box<dyn Error>> {
    let mut reader = Reader::from_reader(csv_data);
    let mut movies = Vec::new();

    for record in reader.deserialize() {
        let csv_record: CsvRecord = record?;
        let movie = FullMovie {
            id: 0,
            name: csv_record
                .title
                .trim()
                .to_string(),
            description: csv_record
                .description
                .map(
                    |description| {
                        description
                            .trim()
                            .to_string()
                    },
                ),
            actors: csv_record
                .actors
                .map(
                    |actor_names| {
                        actor_names
                            .split("|")
                            .map(
                                |actor_name| {
                                    Actor::from(
                                        actor_name
                                            .trim()
                                            .to_string(),
                                    )
                                },
                            )
                            .collect()
                    },
                )
                .unwrap_or_default(),
            director: csv_record
                .director
                .map(
                    |director_name| {
                        Director::from(
                            director_name
                                .trim()
                                .to_string(),
                        )
                    },
                ),
            genres: csv_record
                .genres
                .map(
                    |genres| {
                        genres
                            .split("|")
                            .map(
                                |genre| {
                                    genre
                                        .trim()
                                        .to_string()
                                },
                            )
                            .collect()
                    },
                )
                .unwrap_or_default(),
            embedding: None,
        };
        movies.push(movie);
    }

    Ok(movies)
}

fn get_unique_actors(movies: &[FullMovie]) -> HashSet<&str> {
    movies
        .into_iter()
        .flat_map(
            |movie| {
                movie
                    .actors
                    .iter()
                    .map(
                        |actor| {
                            actor
                                .name
                                .as_str()
                        },
                    )
            },
        )
        .collect()
}

fn get_unique_genres(movies: &[FullMovie]) -> HashSet<&str> {
    movies
        .into_iter()
        .flat_map(
            |movie| {
                movie
                    .genres
                    .iter()
                    .map(|genre| genre.as_str())
            },
        )
        .collect()
}

fn get_unique_directors(movies: &[FullMovie]) -> HashSet<&str> {
    movies
        .into_iter()
        .flat_map(
            |movie| {
                movie
                    .director
                    .as_ref()
                    .map(
                        |d| {
                            Some(
                                d.name
                                    .as_str(),
                            )
                        },
                    )
                    .unwrap_or(None)
            },
        )
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_parse_csv() {
        let csv = "title,description,actors,genres,director
                   t_title,t_description,Actor1 | Actor2 | Actor3,Western | Action | Adventure | Comedy, Randolph Smith";

        let expected = FullMovie {
            id: 0,
            name: "t_title".to_string(),
            description: Some("t_description".to_string()),
            actors: vec![
                Actor::from("Actor1".to_string()),
                Actor::from("Actor2".to_string()),
                Actor::from("Actor3".to_string()),
            ],
            director: Some(Director::from("Randolph Smith".to_string())),
            genres: vec![
                "Western".to_string(),
                "Action".to_string(),
                "Adventure".to_string(),
                "Comedy".to_string(),
            ],
            embedding: None,
        };

        let full_movies = parse_csv(csv.as_bytes());
        println!(
            "{:#?}",
            full_movies
        );
        assert!(full_movies.is_ok());

        let full_movie = full_movies.unwrap()[0].clone();
        assert_eq!(
            full_movie,
            expected
        );
    }
}
