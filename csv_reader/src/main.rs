use std::{error::Error, fs::File, io::Read};

use csv::Reader;
use database::{Actor, Director, FullMovie};
use dotenvy::dotenv;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct CsvRecord {
    title: String,
    description: Option<String>,
    actors: Option<String>,
    director: Option<String>,
    genres: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv()
        .ok()
        .expect("Failed to run env reader");
    let reader = File::open("./test.csv")?;
    let movies = parse_csv(reader)?;

    for movie in movies {
        let result = database::insert_full_movie(movie).await?;
        println!(
            "{:#?}",
            result
        );
    }

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
                            .into_iter()
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
                .unwrap_or(Vec::new()),
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
                            .into_iter()
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
                .unwrap_or_else(Vec::new),
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
        };

        let full_movies = parse_csv(csv);
        assert!(full_movies.is_ok());

        let full_movie = full_movies.unwrap()[0].clone();
        assert_eq!(
            full_movie,
            expected
        );
    }
}
