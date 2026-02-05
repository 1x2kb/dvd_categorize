#[cfg(feature = "text-matching")]
/// Trait for calculating text-based match scores for movies.
/// This allows for different text matching implementations and makes the code more testable.
/// For vector-based matching, see the `VectorSimilarity` trait.
pub trait TextMatchScoring {
    /// Calculates a weighted text match score for the movie based on provided criteria.
    /// This performs text-based matching on titles, actors, and genres.
    ///
    /// # Arguments
    /// * `titles` - Slice of title strings to match against the movie's title
    /// * `actors` - Slice of actor names to match against the movie's actors
    /// * `genres` - Slice of genre names to match against the movie's genres
    ///
    /// # Returns
    /// A score representing how well the movie matches the text criteria, where higher scores
    /// indicate better matches. The scoring is weighted as follows:
    /// - Title match: 3 points per match
    /// - Actor match: 2 points per match
    /// - Genre match: 1 point per match
    fn text_match_score(&self, titles: &[String], actors: &[String], genres: &[String]) -> usize;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Actor, FullMovie};

    #[test]
    fn test_calculate_match_score() {
        let movie = FullMovie {
            id: 1,
            name: "The Matrix".to_string(),
            description: Some(
                "A computer hacker learns about the true nature of reality".to_string(),
            ),
            actors: vec![
                Actor {
                    id: 1,
                    name: "Keanu Reeves".to_string(),
                },
                Actor {
                    id: 2,
                    name: "Laurence Fishburne".to_string(),
                },
            ],
            director: None,
            genres: vec!["Sci-Fi".to_string(), "Action".to_string()],
            #[cfg(any(feature = "postgres", feature = "vector-similarity"))]
            embedding: None,
            added_on: None,
        };

        // Test title match
        assert_eq!(
            movie.text_match_score(
                &["Matrix".to_string()],
                &[],
                &[]
            ),
            3,
            "Should match on title"
        );

        // Test actor match
        assert_eq!(
            movie.text_match_score(
                &[],
                &["Reeves".to_string()],
                &[]
            ),
            2,
            "Should match on actor"
        );

        // Test genre match
        assert_eq!(
            movie.text_match_score(
                &[],
                &[],
                &["Sci-Fi".to_string()]
            ),
            1,
            "Should match on genre"
        );

        // Test multiple matches
        let title = "The".to_string();
        let actor = "Keanu".to_string();
        let genre = "Action".to_string();
        assert_eq!(
            movie.text_match_score(
                &[title.to_string()],
                &[actor.clone()],
                &[genre.clone()]
            ),
            6, // 3 (title) + 2 (actor) + 1 (genre)
            "Should combine scores from multiple matches"
        );

        // Test no matches
        assert_eq!(
            movie.text_match_score(
                &[],
                &[],
                &[]
            ),
            0,
            "Should return 0 for no matches"
        );
    }
}
