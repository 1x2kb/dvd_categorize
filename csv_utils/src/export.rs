use csv::{QuoteStyle, WriterBuilder};
use models::FullMovie;
use std::error::Error;

/// Converts a list of FullMovie objects to CSV format
/// Format: Title, Description, Actors, Genres, Director, AddedOn
pub fn movies_to_csv(movies: &[FullMovie]) -> Result<String, Box<dyn Error>> {
    let mut writer = WriterBuilder::new()
        .quote_style(QuoteStyle::Necessary)
        .double_quote(true)
        .from_writer(vec![]);

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

        let year = movie
            .release_year
            .to_string();

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
        let lines: Vec<&str> = csv
            .lines()
            .collect();
        assert!(lines[0].contains("Year"));
    }

    #[test]
    fn test_csv_quote_escaping() {
        let movies = vec![FullMovie {
            id: 1,
            key_hash: FullMovie::generate_key_hash("Movie with \"Quotes\""),
            name: "Movie with \"Quotes\"".to_string(),
            description: Some("Description with \"quotes\" and commas, here".to_string()),
            actors: vec![Actor {
                id: 1,
                name: "Actor \"Nickname\" Name".to_string(),
            }],
            director: Some(
                Director {
                    id: 1,
                    name: "Director \"The\" Name".to_string(),
                },
            ),
            genres: vec!["Action".to_string()],
            embedding: None,
            added_on: None,
            location: None,
            release_year: 2024,
        }];

        let csv = movies_to_csv(&movies).unwrap();

        // Verify CSV uses "" for escaping quotes (CSV standard), not \"
        println!(
            "Raw CSV output:\n{}",
            csv
        );
        assert!(
            csv.contains("\"Movie with \"\"Quotes\"\"\""),
            "CSV should escape quotes as \"\" not \\\""
        );
        assert!(
            csv.contains("\"Description with \"\"quotes\"\" and commas, here\""),
            "CSV should escape quotes as \"\" not \\\""
        );

        // Parse back to verify proper escaping
        let parsed = crate::parse_csv(csv.as_bytes()).unwrap();
        assert_eq!(
            parsed.len(),
            1
        );
        assert_eq!(
            parsed[0].name,
            "Movie with \"Quotes\""
        );
        assert_eq!(
            parsed[0].description,
            Some("Description with \"quotes\" and commas, here".to_string())
        );
        assert_eq!(
            parsed[0].actors[0].name,
            "Actor \"Nickname\" Name"
        );
        assert_eq!(
            parsed[0]
                .director
                .as_ref()
                .unwrap()
                .name,
            "Director \"The\" Name"
        );
    }
}
