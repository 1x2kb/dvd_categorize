use diesel::{ConnectionError, ExpressionMethods, QueryDsl};
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use models::*;

use crate::DatabaseError;

pub async fn insert_movies(
    new_movies: &[NewMovie], // Accept borrowed slice
    connection: &mut AsyncPgConnection,
) -> Result<
    Vec<(
        i32,
        String,
    )>,
    DatabaseError,
> {
    diesel::insert_into(schema::movie::table)
        .values(new_movies)
        .on_conflict((
            schema::movie::name,
            schema::movie::release_year,
        ))
        .do_update()
        .set(schema::movie::id.eq(schema::movie::id))
        .returning((
            schema::movie::id,
            schema::movie::name,
        ))
        .get_results(connection)
        .await
        .map_err(DatabaseError::from)
}

pub async fn update_movie_location(
    movie_id: i32,
    new_location: String,
) -> Result<(), DatabaseError> {
    let pool = crate::get_connection_pool().await?;
    let mut connection = pool.get().await.map_err(|e| {
        DatabaseError::ConnectionError(ConnectionError::BadConnection(e.to_string()))
    })?;

    diesel::update(schema::movie::table.find(movie_id))
        .set(schema::movie::location.eq(new_location))
        .execute(&mut connection)
        .await
        .map_err(DatabaseError::from)?;

    Ok(())
}
