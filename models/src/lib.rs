#[cfg(feature = "ai")]
pub mod ai_state;

#[cfg(feature = "ai")]
pub use ai_state::*;

#[cfg(feature = "text-matching")]
pub mod text_match_scoring;

#[cfg(feature = "text-matching")]
pub use text_match_scoring::TextMatchScoring;

#[cfg(feature = "postgres")]
use pgvector::Vector;

#[cfg(feature = "postgres")]
pub mod schema;

#[cfg(feature = "testing")]
use rand::{thread_rng, Rng};

#[cfg(feature = "testing")]
pub trait Random {
    fn random() -> Self;
}

#[cfg(feature = "postgres")]
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "postgres")]
use chrono::NaiveDateTime;

#[cfg(feature = "vector-similarity")]
pub mod vector_similarity;

#[cfg(feature = "vector-similarity")]
pub use vector_similarity::VectorSimilarity;

pub mod roled_message;
pub use roled_message::*;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CsvInput {
    pub input: String,
}

#[cfg_attr(feature="postgres", derive(Queryable, Selectable, Identifiable), diesel(table_name = schema::actor, check_for_backend(diesel::pg::Pg)))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Actor {
    pub id: i32,
    pub name: String,
}

impl From<String> for Actor {
    fn from(value: String) -> Self {
        Self { id: 0, name: value }
    }
}

impl
    From<(
        i32,
        String,
    )> for Actor
{
    fn from(
        (id, name): (
            i32,
            String,
        ),
    ) -> Self {
        Self { id, name }
    }
}

#[cfg_attr(feature="postgres", derive(Insertable), diesel(table_name = schema::actor, check_for_backend(diesel::pg::Pg)))]
#[derive(Debug, Clone)]
pub struct NewActor {
    pub name: String,
}

#[cfg_attr(feature="postgres", derive(Queryable, Insertable, Identifiable), diesel(table_name = schema::director, check_for_backend(diesel::pg::Pg)))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Director {
    pub id: i32,
    pub name: String,
}

#[cfg_attr(feature="postgres", derive(Insertable), diesel(table_name = schema::director, check_for_backend(diesel::pg::Pg)))]
pub struct NewDirector {
    pub name: String,
}

impl From<String> for Director {
    fn from(name: String) -> Self {
        Self { id: 0, name }
    }
}
#[cfg_attr(feature="postgres", derive(Insertable, Identifiable, Queryable), diesel(table_name = schema::movie, check_for_backend(diesel::pg::Pg)))]
pub struct Movie {
    pub id: i32,
    pub name: String,
    pub director_id: Option<i32>,
    pub description: Option<String>,
    #[cfg(feature = "postgres")]
    pub embedding: Option<Vector>,
    #[cfg(feature = "postgres")]
    pub added_on: NaiveDateTime,
    pub location: String,
}

#[cfg_attr(feature="postgres", derive(Insertable), diesel(table_name = schema::movie, check_for_backend(diesel::pg::Pg)))]
#[derive(Debug, Clone)]
pub struct NewMovie {
    pub name: String,
    pub director_id: Option<i32>,
    pub description: Option<String>,
    #[cfg(feature = "postgres")]
    pub embedding: Option<Vector>,
    #[cfg(feature = "postgres")]
    pub added_on: Option<NaiveDateTime>,
    pub location: Option<String>,
}

#[cfg_attr(feature="postgres", derive(Insertable, Identifiable, Queryable), diesel(table_name = schema::movie_actor, check_for_backend(diesel::pg::Pg)))]
#[cfg_attr(feature = "postgres", derive(Associations))]
#[cfg_attr(feature = "postgres", diesel(belongs_to(Movie), belongs_to(Actor)))]
#[derive(Debug)]
pub struct MovieActor {
    pub id: i32,
    pub movie_id: i32,
    pub actor_id: i32,
}

#[cfg_attr(feature="postgres", derive(Insertable), diesel(table_name = schema::movie_actor, check_for_backend(diesel::pg::Pg)))]
#[cfg(feature = "postgres")]
pub struct NewMovieActor {
    pub movie_id: i32,
    pub actor_id: i32,
}

#[cfg_attr(feature="postgres", derive(Queryable, Identifiable), diesel(table_name = schema::movie_genre, check_for_backend(diesel::pg::Pg)))]
#[cfg_attr(feature = "postgres", derive(Associations))]
#[cfg_attr(feature = "postgres", diesel(belongs_to(Movie)))]
#[derive(Debug)]
pub struct MovieGenre {
    pub id: i32,
    pub movie_id: i32,
    pub genre: String,
}

#[cfg_attr(feature="postgres", derive(Insertable), diesel(table_name = schema::movie_genre, check_for_backend(diesel::pg::Pg)))]
#[derive(Debug, Clone)]
pub struct NewMovieGenre {
    pub movie_id: i32,
    pub genre: String,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct FullMovie {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,

    pub actors: Vec<Actor>,
    pub director: Option<Director>,
    pub genres: Vec<String>,
    #[serde(skip)]
    #[cfg(any(feature = "postgres", feature = "vector-similarity"))]
    pub embedding: Option<Vec<f32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub added_on: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct ScoredMovie {
    #[serde(flatten)]
    pub movie: FullMovie,
    pub vector_score: f32,
}

impl FullMovie {
    #[cfg(any(feature = "postgres", feature = "vector-similarity"))]
    pub fn embedding_str(&self) -> String {
        let actors: String = self
            .actors
            .iter()
            .map(
                |actor| {
                    actor
                        .name
                        .as_str()
                },
            )
            .collect::<Vec<_>>()
            .join(",");

        let genres: String = self
            .genres
            .iter()
            .map(|genre| genre.as_str())
            .collect::<Vec<_>>()
            .join(",");

        format!(
            "{} {} {} {} {}",
            self.name,
            self.description.as_ref().unwrap_or(&"".to_string()),
            genres,
            actors,
            self.director.as_ref().map(|d| d.name.as_str()).unwrap_or("")
        )
    }
}

impl std::fmt::Debug for FullMovie {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug_struct = f.debug_struct("FullMovie");
        debug_struct
            .field(
                "id", &self.id,
            )
            .field(
                "name", &self.name,
            )
            .field(
                "description",
                &self.description,
            )
            .field(
                "actors",
                &self.actors,
            )
            .field(
                "director",
                &self.director,
            )
            .field(
                "genres",
                &self.genres,
            );

        #[cfg(any(feature = "postgres", feature = "vector-similarity"))]
        {
            let embedding_len = self
                .embedding
                .as_ref()
                .map(|v| v.len());
            debug_struct.field(
                "embedding_len",
                &embedding_len,
            );
        }

        debug_struct.finish()
    }
}

#[cfg(feature = "vector-similarity")]
impl VectorSimilarity for FullMovie {
    fn cosine_similarity(&self, query_embedding: &[f32]) -> Option<f32> {
        self.embedding
            .as_deref()
            .and_then(
                |embedding| {
                    <Self as VectorSimilarity>::cosine_similarity_vectors(
                        embedding,
                        query_embedding,
                    )
                },
            )
    }
}

#[cfg(feature = "text-matching")]
impl TextMatchScoring for FullMovie {
    fn text_match_score(&self, titles: &[String], actors: &[String], genres: &[String]) -> usize {
        let mut score = 0;

        // Check title matches (highest weight)
        let movie_title = self
            .name
            .to_lowercase();
        for title in titles {
            if movie_title.contains(&title.to_lowercase()) {
                score += 3; // Higher weight for title matches
            }
        }

        // Check actor matches (medium weight)
        for actor in actors {
            if self
                .actors
                .iter()
                .any(
                    |a| {
                        a.name
                            .to_lowercase()
                            .contains(&actor.to_lowercase())
                    },
                )
            {
                score += 2;
            }
        }

        // Check genre matches (lowest weight)
        for genre in genres {
            if self
                .genres
                .iter()
                .any(|g| g.to_lowercase() == genre.to_lowercase())
            {
                score += 1;
            }
        }

        score
    }
}

impl
    From<(
        Movie,
        Option<Director>,
        Vec<Actor>,
        Vec<String>,
    )> for FullMovie
{
    fn from(
        (movie, director, actors, genres): (
            Movie,
            Option<Director>,
            Vec<Actor>,
            Vec<String>,
        ),
    ) -> Self {
        Self {
            id: movie.id,
            name: movie.name,
            description: movie.description,
            director,
            actors,
            genres,
            #[cfg(feature = "postgres")]
            embedding: movie
                .embedding
                .map(|v| v.into()),
            #[cfg(feature = "postgres")]
            added_on: Some(movie.added_on.to_string()),
            #[cfg(not(feature = "postgres"))]
            added_on: None,
            location: Some(movie.location),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct SearchRequest {
    pub query: String,
    #[serde(default)]
    pub disable_enhancement: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchResponse {
    pub results: Vec<ScoredMovie>,
    pub original_query: String,
    pub enhanced_query: String,
}

#[cfg(feature = "testing")]
impl Random for Actor {
    fn random() -> Self {
        let id = thread_rng().gen_range(0..100000000);

        Self {
            id,
            name: format!(
                "Actor Name {}",
                thread_rng().gen_range(0..1000)
            ),
        }
    }
}
#[cfg(feature = "testing")]
impl Random for Director {
    fn random() -> Self {
        Self {
            id: rand::thread_rng().gen_range(0..10000000),
            name: format!(
                "Director Name {}",
                thread_rng().gen_range(0..1000)
            ),
        }
    }
}

#[cfg(feature = "testing")]
const GENRES: [&str; 4] = ["Western", "Action", "Sci-Fi", "Fantasy"];

#[cfg(feature = "testing")]
impl Random for FullMovie {
    fn random() -> Self {
        let mut random = thread_rng();
        let num_actors = random.gen_range(1..10);
        let num_genres = random.gen_range(1..4);

        Self {
            id: random.gen_range(0..10000000),
            name: format!(
                "Movie Title {}",
                random.gen_range(0..1000),
            ),
            description: Some(
                format!(
                    "Movie Description {}",
                    random.gen_range(0..1000),
                ),
            ),
            actors: (0..num_actors)
                .map(|_| Actor::random())
                .collect(),
            director: Some(Director::random()),
            genres: (1..num_genres)
                .map(|_| GENRES[random.gen_range(1..GENRES.len())].to_string())
                .collect(),
            #[cfg(any(feature = "postgres", feature = "vector-similarity"))]
            embedding: None,
            added_on: None,
            location: None,
        }
    }
}

#[cfg(feature = "testing")]
impl FullMovie {
    /// Creates a consistent set of test movies for unit testing.
    /// Returns well-known movies with realistic data that can be used
    /// across all test suites in the workspace.
    pub fn create_test_movies() -> Vec<FullMovie> {
        vec![
            FullMovie {
                id: 1,
                name: "The Matrix".to_string(),
                description: Some(
                    "A computer hacker learns about the true nature of reality".to_string(),
                ),
                actors: vec![Actor {
                    id: 1,
                    name: "Keanu Reeves".to_string(),
                }],
                director: Some(
                    Director {
                        id: 1,
                        name: "The Wachowskis".to_string(),
                    },
                ),
                genres: vec!["Sci-Fi".to_string(), "Action".to_string()],
                #[cfg(any(feature = "postgres", feature = "vector-similarity"))]
                embedding: None,
                added_on: None,
                location: None,
            },
            FullMovie {
                id: 2,
                name: "Inception".to_string(),
                description: Some(
                    "A thief who steals corporate secrets through dream-sharing technology"
                        .to_string(),
                ),
                actors: vec![Actor {
                    id: 2,
                    name: "Leonardo DiCaprio".to_string(),
                }],
                director: Some(
                    Director {
                        id: 2,
                        name: "Christopher Nolan".to_string(),
                    },
                ),
                genres: vec!["Sci-Fi".to_string(), "Thriller".to_string()],
                #[cfg(any(feature = "postgres", feature = "vector-similarity"))]
                embedding: None,
                added_on: None,
                location: None,
            },
            FullMovie {
                id: 3,
                name: "Interstellar".to_string(),
                description: Some(
                    "A team of explorers travel through a wormhole in space in an attempt to ensure humanity's survival"
                        .to_string(),
                ),
                actors: vec![Actor {
                    id: 3,
                    name: "Matthew McConaughey".to_string(),
                }],
                director: Some(
                    Director {
                        id: 3,
                        name: "Christopher Nolan".to_string(),
                    },
                ),
                genres: vec!["Sci-Fi".to_string(), "Adventure".to_string()],
                #[cfg(any(feature = "postgres", feature = "vector-similarity"))]
                embedding: None,
                added_on: None,
                location: None,
            },
        ]
    }
}
