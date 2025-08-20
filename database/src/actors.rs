use std::collections::HashMap;

use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use models::{schema::actor, Actor, Movie, MovieActor, NewActor};

use crate::DatabaseError;

pub async fn insert_actors(
    actors: impl Iterator<Item = String>,
    connection: &mut AsyncPgConnection,
) -> Result<HashMap<String, i32>, DatabaseError> {
    let new_actors: Vec<NewActor> = actors
        .into_iter()
        .map(|name| NewActor { name: name })
        .collect();

    let ids: Vec<i32> = diesel::insert_into(actor::table)
        .values(&new_actors)
        .on_conflict(actor::name)
        .do_update()
        .set(actor::id.eq(actor::id))
        .returning(actor::id)
        .get_results::<i32>(connection)
        .await
        .map_err(DatabaseError::from)?;

    Ok(
        new_actors
            .into_iter()
            .map(|actor| actor.name)
            .zip(ids)
            .collect(),
    )
}

pub async fn actors_for_movie(movie: &Movie, connection: &mut AsyncPgConnection) -> Vec<Actor> {
    MovieActor::belonging_to(&movie)
        .inner_join(actor::table)
        .select(actor::all_columns)
        .load::<Actor>(connection)
        .await
        .unwrap_or_else(|_| Vec::new())
}
