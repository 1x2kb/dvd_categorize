use std::collections::HashMap;

use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use models::{schema::actor, Actor, Movie, MovieActor, NewActor};

use crate::DatabaseError;

pub async fn insert_actors(
    actors: impl Iterator<Item = NewActor>,
    connection: &mut AsyncPgConnection,
) -> Result<Vec<(String, i32)>, DatabaseError> {
    diesel::insert_into(actor::table)
        .values(&actors)
        .on_conflict(actor::name)
        .do_update()
        .set(actor::id.eq(actor::id))
        .returning(actor::id, actor::name)
        .get_results::<i32>(connection)
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
