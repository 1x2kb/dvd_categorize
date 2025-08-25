use std::collections::HashMap;

use diesel_async::{AsyncPgConnection, RunQueryDsl};
use models::{schema, NewMovie};

use crate::DatabaseError;

pub async fn insert_movies(
    new_movies: &[NewMovie],  // Accept borrowed slice
    connection: &mut AsyncPgConnection,
) -> Result<HashMap<String, i32>, DatabaseError> {
    let insert_result: Vec<(i32, String)> = diesel::insert_into(schema::movie::table)
        .values(new_movies)  // No & needed, already borrowed
        .returning((schema::movie::id, schema::movie::name))
        .get_results(connection)
        .await
        .map_err(DatabaseError::from)?;

    let mut result = HashMap::new();
    for (id, name) in insert_result {
        result.insert(name, id);
    }
    Ok(result)
}
