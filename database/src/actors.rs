use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use models::{schema::{actor, movie_actor}, Actor, Movie, MovieActor, NewActor, NewMovieActor};

use crate::DatabaseError;

pub async fn insert_actors(
    actors: &[NewActor],
    connection: &mut AsyncPgConnection,
) -> Result<Vec<(i32, String)>, DatabaseError> {
    diesel::insert_into(actor::table)
        .values(actors)
        .on_conflict(actor::name)
        .do_update()
        .set(actor::id.eq(actor::id))
        .returning((actor::id, actor::name))
        .get_results::<(i32, String)>(connection)
        .await
        .map_err(DatabaseError::from)
}

pub async fn insert_movie_actors(
    movie_actors: &[NewMovieActor],
    connection: &mut AsyncPgConnection,
) -> Result<Vec<MovieActor>, DatabaseError> {
    diesel::insert_into(movie_actor::table)
        .values(movie_actors)
        .on_conflict((movie_actor::movie_id, movie_actor::actor_id))
        .do_nothing()
        .get_results::<MovieActor>(connection)
        .await
        .map_err(DatabaseError::from)
}

pub async fn actors_for_movie(movie: &Movie, connection: &mut AsyncPgConnection) -> Vec<Actor> {
    MovieActor::belonging_to(&movie)
        .inner_join(actor::table)
        .select(actor::all_columns)
        .load::<Actor>(connection)
        .await
        .unwrap_or_else(|_| Vec::new())
}
