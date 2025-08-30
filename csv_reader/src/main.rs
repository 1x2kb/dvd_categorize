use std::{collections::HashSet, error::Error, fs::File, io::Read};

use csv::Reader;
use database::{director, Actor, Director, FullMovie};
use dotenvy::dotenv;
use log::{debug, info};
use models::{MovieActor, NewActor, NewDirector, NewMovie, NewMovieActor, NewMovieGenre};
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
    let mut full_movies: Vec<FullMovie> = parse_csv(reader)?;
    info!("Parsed csv");
    debug!(
        "{} movies read in",
        full_movies.len()
    );

    let (mut actors, mut genres, mut directors) = (
        get_unique_actors(&full_movies),
        get_unique_genres(&full_movies),
        get_unique_directors(&full_movies),
    );

    actors.sort_by(
        |a, b| {
            a.name
                .cmp(&b.name)
        },
    ); // Sort results for binary search.
    directors.sort_by(
        |a, b| {
            a.name
                .cmp(&b.name)
        },
    ); // Sort results for binary search

    let mut connection = database::get_database_connection().await?;
    let actors = database::insert_actors(
        &actors,
        &mut connection,
    )
    .await?;

    let directors = database::insert_directors(
        &directors,
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

    let movies: Vec<NewMovie> = full_movies
        .iter_mut()
        .zip(embeddings.into_iter())
        .map(
            |(movie, embedding)| NewMovie {
                name: movie
                    .name
                    .to_string(),
                director_id: movie
                    .director
                    .iter()
                    .flat_map(
                        |director| {
                            directors
                                .binary_search_by(
                                    |(_, director_name)| director_name.cmp(&director.name),
                                )
                                .ok()
                                .and_then(
                                    |index| {
                                        directors
                                            .get(index)
                                            .map(|(id, _)| *id)
                                    },
                                )
                        },
                    )
                    .next(),
                description: movie
                    .description
                    .clone(),
                embedding: Some(embedding.into()),
            },
        )
        .collect();

    let mut movie_inserts = database::insert_movies(
        &movies,
        &mut connection,
    )
    .await?;

    movie_inserts.sort_by(
        |a, b| {
            a.1.cmp(&b.1)
        },
    );

    let movie_actors: Vec<NewMovieActor> = full_movies
        .iter()
        .flat_map(
            |movie| {
                let movie_id = movie_inserts
                    .binary_search_by(|(_, movie_name)| movie_name.cmp(&movie.name))
                    .ok()
                    .and_then(
                        |found_index| {
                            movie_inserts
                                .get(found_index)
                                .map(|(id, _)| *id)
                        },
                    );

                if let Some(movie_id) = movie_id {
                    return movie
                        .actors
                        .iter()
                        .filter_map(
                            |actor| {
                                actors
                                    .binary_search_by(|(_, name)| name.cmp(&actor.name))
                                    .ok()
                                    .and_then(|index| actors.get(index))
                                    .map(|(actor_id, _)| NewMovieActor {
                                        movie_id,
                                        actor_id: *actor_id,
                                    })
                            },
                        )
                        .collect();
                }

                Vec::new()
            },
        )
        .collect();

    let movie_actors = database::insert_movie_actors(
        &movie_actors,
        &mut connection,
    )
    .await?;

    let movie_genres: Vec<NewMovieGenre> = full_movies
        .iter()
        .flat_map(
            |movie| {
                let movie_id = movie_inserts
                    .binary_search_by(|(_, movie_name)| movie_name.cmp(&movie.name))
                    .ok()
                    .and_then(
                        |found_index| {
                            movie_inserts
                                .get(found_index)
                                .map(|(id, _)| *id)
                        },
                    );

                if let Some(movie_id) = movie_id {
                    return movie
                        .genres
                        .iter()
                        .map(
                            |genre| NewMovieGenre {
                                movie_id,
                                genre: genre.clone(),
                            },
                        )
                        .collect();
                }

                Vec::new()
            },
        )
        .collect();

    let movie_genres = database::insert_movie_genres(
        &movie_genres,
        &mut connection,
    )
    .await?;

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

fn get_unique_actors(movies: &[FullMovie]) -> Vec<NewActor> {
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
        .collect::<HashSet<&str>>()
        .into_iter()
        .map(
            |actor_name| NewActor {
                name: actor_name.to_string(),
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

fn get_unique_directors(movies: &[FullMovie]) -> Vec<NewDirector> {
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
        .collect::<HashSet<&str>>()
        .into_iter()
        .map(
            |director_name| NewDirector {
                name: director_name.to_string(),
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
