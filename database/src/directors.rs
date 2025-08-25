use std::collections::HashMap;

use diesel_async::{AsyncPgConnection, RunQueryDsl};
use models::{schema, NewDirector};

use crate::DatabaseError;

pub async fn insert_directors(
    directors: impl Iterator<Item = NewDirector>,
    connection: &mut AsyncPgConnection,
) -> Result<Vec<(i32, String)>, DatabaseError> {
    diesel::insert_into(schema::director::table)
        .values(&directors)
        .returning((
            schema::director::id,
            schema::director::name,
        ))
        .get_results(connection)
        .await
        .map_err(DatabaseError::from)
}
