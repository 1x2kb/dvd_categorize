use csv::Writer;
use models::FullMovie;
use std::error::Error;

/// Converts a list of FullMovie objects to CSV format
/// Format: Title, Description, Actors, Genres, Director, AddedOn
pub fn movies_to_csv(movies: &[FullMovie]) -> Result<String, Box<dyn Error>> {
    let mut writer = Writer::from_writer(vec![]);

    // Write header row
    writer.write_record([
        "Title",
        "Year",
        "Description",
        "Actors",
        "Genres",
        "Director",
        "AddedOn",
        "Location",
    ])?;

    for movie in movies {
        let actors = movie
            .actors
            .iter()
            .map(
                |a| {
                    a.name
                        .as_str()
                },
            )
            .collect::<Vec<_>>()
            .join(" | ");

        let genres = movie
            .genres
            .join(" | ");

        let director = movie
            .director
            .as_ref()
            .map(
                |d| {
                    d.name
                        .as_str()
                },
            )
            .unwrap_or("");

        let added_on = movie
            .added_on
            .as_deref()
            .unwrap_or("");

        let location = movie
            .location
            .as_deref()
            .unwrap_or("");

        let year = movie.release_year.to_string();

        writer.write_record([
            &movie.name,
            &year,
            movie
                .description
                .as_deref()
                .unwrap_or(""),
            &actors,
            &genres,
            director,
            added_on,
            location,
        ])?;
    }

    let csv_bytes = writer.into_inner()?;
    Ok(String::from_utf8(csv_bytes)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use models::{Actor, Director};

    #[test]
    fn test_movies_to_csv() {
        let movies = vec![FullMovie {
            id: 1,
            key_hash: FullMovie::generate_key_hash("Test Movie"),
            name: "Test Movie".to_string(),
            description: Some("A test description".to_string()),
            actors: vec![
                Actor {
                    id: 1,
                    name: "Actor One".to_string(),
                },
                Actor {
                    id: 2,
                    name: "Actor Two".to_string(),
                },
            ],
            director: Some(
                Director {
                    id: 1,
                    name: "Test Director".to_string(),
                },
            ),
            genres: vec!["Action".to_string(), "Drama".to_string()],
            embedding: None,
            added_on: None,
            location: None,
            release_year: 2024,
        }];

        let csv = movies_to_csv(&movies).unwrap();
        assert!(csv.contains("Test Movie"));
        assert!(csv.contains("2024"));
        assert!(csv.contains("A test description"));
        assert!(csv.contains("Actor One | Actor Two"));
        assert!(csv.contains("Action | Drama"));
        assert!(csv.contains("Test Director"));
        
        // Verify header includes Year column
        let lines: Vec<&str> = csv.lines().collect();
        assert!(lines[0].contains("Year"));
    }
}
