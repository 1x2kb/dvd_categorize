use std::{error::Error, io::Read};

use csv::Reader;
use models::{Actor, Director, FullMovie};
use serde::{Deserialize, Serialize};

pub mod export;
pub use export::movies_to_csv;

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
    #[serde(alias = "AddedOn", alias = "ADDED_ON", alias = "added_on")]
    added_on: Option<String>,
    #[serde(alias = "Location", alias = "LOCATION")]
    location: Option<String>,
}

pub fn parse_csv(csv_data: impl Read) -> Result<Vec<FullMovie>, Box<dyn Error>> {
    let mut reader = Reader::from_reader(csv_data);
    let mut movies = Vec::new();
    let mut seen_names = std::collections::HashSet::new();

    for record in reader.deserialize() {
        let csv_record: CsvRecord = record?;
        let name = csv_record
            .title
            .trim()
            .to_string();

        // Skip if we've already seen this movie name
        if !seen_names.insert(name.clone()) {
            continue;
        }

        let movie = FullMovie {
            id: 0,
            name,
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
            #[cfg(any(feature = "postgres", feature = "vector-similarity"))]
            embedding: None,
            added_on: csv_record.added_on,
            location: csv_record
                .location
                .map(
                    |l| {
                        l.trim()
                            .to_string()
                    },
                ),
        };
        movies.push(movie);
    }

    Ok(movies)
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
            #[cfg(any(feature = "postgres", feature = "vector-similarity"))]
            embedding: None,
            added_on: None,
            location: None,
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
