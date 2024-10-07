// Generated by diesel_ext

#![allow(unused)]
#![allow(clippy::all)]

use crate::schema::*;

use diesel::prelude::*;
use movie_genre::genre;

#[derive(Queryable, Selectable, Debug, Insertable, Clone, Identifiable)]
#[diesel(table_name = actor)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Actor {
    pub id: i32,
    pub name: String,
}

#[derive(Queryable, Debug, Insertable, Clone, Identifiable)]
#[diesel(table_name = director)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Director {
    pub id: i32,
    pub name: String,
}

#[derive(Queryable, Debug, Insertable, Identifiable)]
#[diesel(table_name = movie)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Movie {
    pub id: i32,
    pub name: String,
    pub director_id: Option<i32>,
    pub description: Option<String>,
}

#[derive(Queryable, Debug, Insertable, Associations, Identifiable)]
#[diesel(table_name = movie_actor)]
#[diesel(belongs_to(Movie))]
#[diesel(belongs_to(Actor))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct MovieActor {
    pub id: i32,
    pub movie_id: i32,
    pub actor_id: i32,
}

#[derive(Queryable, Debug, Insertable, Associations, Identifiable)]
#[diesel(table_name = movie_genre)]
#[diesel(belongs_to(Movie))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct MovieGenre {
    pub id: i32,
    pub genre: String,
    pub movie_id: i32,
}

#[derive(Debug, Clone)]
pub struct FullMovie {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,

    pub actors: Vec<Actor>,
    pub director: Option<Director>,
    pub genres: Vec<String>,
}

impl From<(Movie, Option<Director>, Vec<Actor>, Vec<String>)> for FullMovie {
    fn from(
        (movie, director, actors, genres): (Movie, Option<Director>, Vec<Actor>, Vec<String>),
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
