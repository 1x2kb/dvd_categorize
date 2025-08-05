#[cfg(feature = "ai")]
pub mod ai_state;
mod chat;

#[cfg(feature = "ai")]
pub use ai_state::*;
#[cfg(feature = "postgres")]
use pgvector::Vector;

#[cfg(feature = "postgres")]
pub mod schema;

#[cfg(feature = "testing")]
use crate::Random;
#[cfg(feature = "testing")]
use rand::{thread_rng, Rng};

#[cfg(feature = "postgres")]
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

pub mod roled_message;
pub use roled_message::*;

#[cfg_attr(feature="postgres", derive(Queryable, Selectable,Identifiable), diesel(table_name = schema::actor, check_for_backend(diesel::pg::Pg)))]
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
    #[cfg(all(feature = "postgres", feature = "ai"))]
    pub embedding: Option<Vector>,
}

#[cfg_attr(feature="postgres", derive(Insertable), diesel(table_name = schema::movie, check_for_backend(diesel::pg::Pg)))]
#[derive(Debug, Clone)]
pub struct NewMovie {
    pub name: String,
    pub director_id: Option<i32>,
    pub description: Option<String>,
    #[cfg(all(feature = "postgres", feature = "ai"))]
    pub embedding: Option<Vector>,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FullMovie {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,

    pub actors: Vec<Actor>,
    pub director: Option<Director>,
    pub genres: Vec<String>,
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
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct SearchRequest {
    pub query: String,
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
const GENRES: [&'static str; 4] = ["Western", "Action", "Sci-Fi", "Fantasy"];

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
        }
    }
}
