pub mod models;
mod schema;

use std::env;

pub use diesel::prelude::*;
pub use diesel::BelongingToDsl;
pub use models::*;
use schema::movie_genre;

fn get_database_connection() -> Result<PgConnection, ConnectionError> {
    let database_url =
        env::var("DATABASE_URL").expect("No database information found, cannot connect");
    PgConnection::establish(&database_url)
}

pub fn get_movies() -> Result<Vec<FullMovie>, diesel::result::Error> {
    let mut connection =
        get_database_connection().unwrap_or_else(|_| panic!("Failed to establish a connection"));

    let movies = schema::movie::table
        .left_join(schema::director::table)
        .get_results::<(Movie, Option<Director>)>(&mut connection)?;

    let full_movies = movies
        .into_iter()
        .map(|(movie, director)| {
            let actors = MovieActor::belonging_to(&movie)
                .inner_join(schema::actor::table)
                .select(schema::actor::all_columns)
                .load::<Actor>(&mut connection)
                .unwrap_or_else(|_| Vec::new());

            let genres = MovieGenre::belonging_to(&movie)
                .select(movie_genre::genre)
                .load::<String>(&mut connection)
                .unwrap_or_else(|_| Vec::new());

            FullMovie::from((movie, director, actors, genres))
        })
        .collect();

    Ok(full_movies)
}
