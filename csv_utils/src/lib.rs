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

const REQUIRED_HEADERS: [&str; 7] = [
    "Title",
    "Description",
    "Actors",
    "Genres",
    "Director",
    "AddedOn",
    "Location",
];

fn validate_headers(headers: &csv::StringRecord) -> Result<(), String> {
    if headers.len() != REQUIRED_HEADERS.len() {
        return Err(
            format!(
                "Expected {} headers, but found {}. Required headers: {}",
                REQUIRED_HEADERS.len(),
                headers.len(),
                REQUIRED_HEADERS.join(", ")
            ),
        );
    }

    // Convert to sets for order-independent comparison
    let required_set: std::collections::HashSet<&str> = REQUIRED_HEADERS
        .iter()
        .copied()
        .collect();
    let actual_set: std::collections::HashSet<&str> = headers
        .iter()
        .collect();

    // Check for missing headers
    let missing: Vec<&str> = required_set
        .difference(&actual_set)
        .copied()
        .collect();
    if !missing.is_empty() {
        return Err(
            format!(
                "Missing required headers: {}. Required headers: {}",
                missing.join(", "),
                REQUIRED_HEADERS.join(", ")
            ),
        );
    }

    // Check for extra/incorrect headers
    let extra: Vec<&str> = actual_set
        .difference(&required_set)
        .copied()
        .collect();
    if !extra.is_empty() {
        return Err(
            format!(
                "Invalid headers found: {}. Required headers: {}",
                extra.join(", "),
                REQUIRED_HEADERS.join(", ")
            ),
        );
    }

    Ok(())
}

pub fn parse_csv(csv_data: impl Read) -> Result<Vec<FullMovie>, Box<dyn Error>> {
    let mut reader = Reader::from_reader(csv_data);

    // Validate headers
    let headers = reader.headers()?;
    validate_headers(headers).map_err(
        |e| {
            format!(
                "CSV header validation failed: {}",
                e
            )
        },
    )?;

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
            key_hash: FullMovie::generate_key_hash(&name),
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
        let csv = "Title,Description,Actors,Genres,Director,AddedOn,Location
t_title,t_description,Actor1 | Actor2 | Actor3,Western | Action | Adventure | Comedy,Randolph Smith,2024-01-01,Shelf A";

        let expected = FullMovie {
            id: 0,
            key_hash: FullMovie::generate_key_hash("t_title"),
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
            added_on: Some("2024-01-01".to_string()),
            location: Some("Shelf A".to_string()),
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

    #[test]
    fn it_should_reject_wrong_number_of_headers() {
        let csv_too_few = "Title,Description,Actors,Genres,Director
t_title,t_description,Actor1,Western,Director";

        let result = parse_csv(csv_too_few.as_bytes());
        assert!(result.is_err());
        let err = result
            .unwrap_err()
            .to_string();
        assert!(err.contains("Expected 7 headers, but found 5"));
    }

    #[test]
    fn it_should_reject_wrong_header_names() {
        let csv_wrong_name = "Title,Description,Actors,Genres,Director,Updated,Location
t_title,t_description,Actor1,Western,Director,2024-01-01,Shelf A";

        let result = parse_csv(csv_wrong_name.as_bytes());
        assert!(result.is_err());
        let err = result
            .unwrap_err()
            .to_string();
        println!(
            "Error message: {}",
            err
        );
        assert!(err.contains("Invalid headers") || err.contains("Missing required headers"));
        assert!(err.contains("AddedOn") || err.contains("Updated"));
    }

    #[test]
    fn it_should_accept_different_header_order() {
        let csv_different_order = "Director,Genres,Actors,Description,Title,Location,AddedOn
Randolph Smith,Western | Action,Actor1 | Actor2,t_description,t_title,Shelf A,2024-01-01";

        let result = parse_csv(csv_different_order.as_bytes());
        assert!(
            result.is_ok(),
            "Should accept headers in any order"
        );

        let movies = result.unwrap();
        assert_eq!(
            movies.len(),
            1
        );
        assert_eq!(
            movies[0].name,
            "t_title"
        );
        assert_eq!(
            movies[0].description,
            Some("t_description".to_string())
        );
        assert_eq!(
            movies[0]
                .actors
                .len(),
            2
        );
        assert_eq!(
            movies[0]
                .genres
                .len(),
            2
        );
    }
}
