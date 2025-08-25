use std::collections::HashMap;

use diesel_async::AsyncPgConnection;
use models::{schema, NewMovie};

use crate::DatabaseError;

pub async fn insert_movies(
    new_movies: impl Iterator<Item = NewMovie>,
    connection: &mut AsyncPgConnection,
) -> Result<HashMap<String, i32>, DatabaseError> {
    let insert_result: Vec<(i32, String)> = diesel::insert_into(schema::movie::table)
        .values(&new_movies)
        .returning((schema::movie::id, schema::movie::name))
        .execute(connection)
        .await
        .map_err(DatabaseError::from)?;

    Ok(HashMap::new())
}
