use diesel::ExpressionMethods;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use models::{schema, NewMovie};

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
        .on_conflict(schema::movie::name)
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
