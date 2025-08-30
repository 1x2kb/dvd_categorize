use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use models::{schema, NewDirector};

use crate::DatabaseError;

pub async fn insert_directors(
    directors: &[NewDirector],
    connection: &mut AsyncPgConnection,
) -> Result<
    Vec<(
        i32,
        String,
    )>,
    DatabaseError,
> {
    use schema::director::dsl::*;

    // For each director, try to insert or get the existing one
    let mut result = Vec::with_capacity(directors.len());

    for new_director in directors {
        let inserted = diesel::insert_into(schema::director::table)
            .values(new_director)
            .on_conflict(name)
            .do_update()
            .set(id.eq(id))
            .returning((
                id, name,
            ))
            .get_result::<(
                i32,
                String,
            )>(connection)
            .await?;

        result.push(inserted);
    }

    Ok(result)
}
