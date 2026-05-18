use database::FullMovie;
use rand::{rng, RngExt};

pub fn get_random(movies: &[FullMovie]) -> &FullMovie {
    let gen = rng().random_range(0..movies.len());

    &movies[gen]
}

/// Strips punctuation from a string for comparison purposes.
/// Replaces all non-alphanumeric characters with spaces, then normalizes whitespace.
///
/// # Examples
/// ```
/// use categorizer_utilities::strip_punctuation;
///
/// assert_eq!(strip_punctuation("Spider-Man"), "Spider Man");
/// assert_eq!(strip_punctuation("O'Brien"), "O Brien");
/// assert_eq!(strip_punctuation("RoboCop: The Movie!"), "RoboCop The Movie");
/// ```
pub fn strip_punctuation(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_punctuation_removes_hyphens() {
        assert_eq!(
            strip_punctuation("Spider-Man"),
            "Spider Man"
        );
        assert_eq!(
            strip_punctuation("X-Men"),
            "X Men"
        );
    }

    #[test]
    fn strip_punctuation_removes_apostrophes() {
        assert_eq!(
            strip_punctuation("O'Brien"),
            "O Brien"
        );
        assert_eq!(
            strip_punctuation("It's"),
            "It s"
        );
    }

    #[test]
    fn strip_punctuation_removes_colons_and_exclamation() {
        assert_eq!(
            strip_punctuation("RoboCop: The Movie!"),
            "RoboCop The Movie"
        );
        assert_eq!(
            strip_punctuation("Die Hard: With a Vengeance"),
            "Die Hard With a Vengeance"
        );
    }

    #[test]
    fn strip_punctuation_normalizes_whitespace() {
        assert_eq!(
            strip_punctuation("Too   many    spaces"),
            "Too many spaces"
        );
        assert_eq!(
            strip_punctuation("  Leading and trailing  "),
            "Leading and trailing"
        );
    }

    #[test]
    fn strip_punctuation_handles_mixed_punctuation() {
        assert_eq!(
            strip_punctuation("The Good, the Bad & the Ugly"),
            "The Good the Bad the Ugly"
        );
        assert_eq!(
            strip_punctuation("(500) Days of Summer"),
            "500 Days of Summer"
        );
    }

    #[test]
    fn strip_punctuation_preserves_alphanumeric() {
        assert_eq!(
            strip_punctuation("2001 A Space Odyssey"),
            "2001 A Space Odyssey"
        );
        assert_eq!(
            strip_punctuation("Se7en"),
            "Se7en"
        );
    }

    #[test]
    fn strip_punctuation_empty_string() {
        assert_eq!(
            strip_punctuation(""),
            ""
        );
        assert_eq!(
            strip_punctuation("   "),
            ""
        );
    }

    #[test]
    fn strip_punctuation_only_punctuation() {
        assert_eq!(
            strip_punctuation("!!!"),
            ""
        );
        assert_eq!(
            strip_punctuation("..."),
            ""
        );
    }
}
