use crate::{get_database_connection, DatabaseError};
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use models::{
    schema::movie_genre::{self},
    Movie, MovieGenre, NewMovieGenre,
};

/// Get all unique genres from the database
pub async fn get_all_genres() -> Result<Vec<String>, DatabaseError> {
    use crate::schema::movie_genre::dsl::*;
    let mut conn = get_database_connection().await?;

    movie_genre
        .select(genre)
        .distinct()
        .load::<String>(&mut conn)
        .await
        .map_err(DatabaseError::from)
}

pub async fn insert_movie_genres(
    movie_genres: &[NewMovieGenre],
    connection: &mut AsyncPgConnection,
) -> Result<Vec<MovieGenre>, DatabaseError> {
    diesel::insert_into(movie_genre::table)
        .values(movie_genres)
        .on_conflict((
            movie_genre::movie_id,
            movie_genre::genre,
        ))
        .do_nothing()
        .get_results::<MovieGenre>(connection)
        .await
        .map_err(DatabaseError::from)
}

pub async fn genres_for_movie(movie: &Movie, connection: &mut AsyncPgConnection) -> Vec<String> {
    MovieGenre::belonging_to(&movie)
        .select(movie_genre::genre)
        .load::<String>(connection)
        .await
        .unwrap_or_else(|_| Vec::new())
}
