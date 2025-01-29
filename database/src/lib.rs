use std::env;
use std::error::Error;
use std::fmt::Display;

pub use diesel::prelude::*;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
use log::debug;
pub use models::{schema::*, *};

#[cfg(feature = "testing")]
pub trait Random {
    fn random() -> Self;
}

#[derive(Debug)]
pub enum DatabaseError {
    ConnectionError(ConnectionError),
    DieselError(diesel::result::Error),
}

impl From<ConnectionError> for DatabaseError {
    fn from(value: ConnectionError) -> Self {
        DatabaseError::ConnectionError(value)
    }
}

impl From<diesel::result::Error> for DatabaseError {
    fn from(value: diesel::result::Error) -> Self {
        DatabaseError::DieselError(value)
    }
}

impl Error for DatabaseError {}

impl Display for DatabaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:#?}",
            self
        )
    }
}

async fn get_database_connection() -> Result<AsyncPgConnection, DatabaseError> {
    let database_url =
        env::var("DATABASE_URL").expect("No database information found, cannot connect");
    AsyncPgConnection::establish(&database_url)
        .await
        .map_err(DatabaseError::from)
}

pub async fn get_movies() -> Result<Vec<FullMovie>, DatabaseError> {
    let mut connection = get_database_connection().await?;

    let movies = schema::movie::table
        .left_join(schema::director::table)
        .get_results::<(
            Movie,
            Option<Director>,
        )>(&mut connection)
        .await
        .map_err(DatabaseError::from)?;

    let mut full_movies = Vec::new();

    for (movie, director) in movies {
        let actors = MovieActor::belonging_to(&movie)
            .inner_join(schema::actor::table)
            .select(schema::actor::all_columns)
            .load::<Actor>(&mut connection)
            .await
            .unwrap_or_else(|_| Vec::new());

        let genres = MovieGenre::belonging_to(&movie)
            .select(movie_genre::genre)
            .load::<String>(&mut connection)
            .await
            .unwrap_or_else(|_| Vec::new());

        full_movies.push(
            FullMovie::from((
                movie, director, actors, genres,
            )),
        );
    }

    debug!(
        "Found {} results",
        full_movies.len()
    );
    Ok(full_movies)
}

pub async fn get_movie(id: i32) -> Result<FullMovie, DatabaseError> {
    let mut connection = get_database_connection()
        .await
        .map_err(DatabaseError::from)?;

    let (movie, director) = schema::movie::table
        .find(id)
        .left_join(schema::director::table)
        .get_result::<(
            Movie,
            Option<Director>,
        )>(&mut connection)
        .await?;

    let actors = actors_for_movie(
        &movie,
        &mut connection,
    )
    .await;

    let genres = genres_for_movie(
        &movie,
        &mut connection,
    )
    .await;

    Ok(
        FullMovie::from((
            movie, director, actors, genres,
        )),
    )
}

pub async fn insert_full_movie(full_movie: FullMovie) -> Result<FullMovie, DatabaseError> {
    let mut conn = get_database_connection()
        .await
        .map_err(DatabaseError::from)?;

    // Director is optional, insert or select id for director name.
    let director_id: Option<i32> = match full_movie.director {
        Some(director) => {
            let new_director = NewDirector {
                name: director.name,
            };

            let director_id: i32 = diesel::insert_into(schema::director::table)
                .values(&new_director)
                .on_conflict(schema::director::name)
                .do_update()
                .set(schema::director::id.eq(schema::director::id))
                .returning(schema::director::id)
                .get_result(&mut conn)
                .await?;

            Some(director_id)
        }
        None => None,
    };

    let new_movie = NewMovie {
        name: full_movie.name,
        director_id,
        description: full_movie.description,
    };

    let movie_id = diesel::insert_into(schema::movie::table)
        .values(&new_movie)
        .returning(schema::movie::id)
        .get_result::<i32>(&mut conn)
        .await?;

    let actors: Vec<NewActor> = full_movie
        .actors
        .into_iter()
        .map(|actor| NewActor { name: actor.name })
        .collect();

    let actors = diesel::insert_into(schema::actor::table)
        .values(&actors)
        .on_conflict(schema::actor::name)
        .do_update()
        .set(schema::actor::id.eq(schema::actor::id))
        .returning(schema::actor::id)
        .get_results::<i32>(&mut conn)
        .await?;

    let new_actor_movies: Vec<NewMovieActor> = actors
        .into_iter()
        .map(|actor_id| NewMovieActor { movie_id, actor_id })
        .collect();

    let _ = diesel::insert_into(schema::movie_actor::table)
        .values(&new_actor_movies)
        .execute(&mut conn)
        .await?;

    if !full_movie
        .genres
        .is_empty()
    {
        let movie_genres: Vec<NewMovieGenre> = full_movie
            .genres
            .into_iter()
            .map(|genre| NewMovieGenre { movie_id, genre })
            .collect();

        let _ = diesel::insert_into(schema::movie_genre::table)
            .values(&movie_genres)
            .execute(&mut conn)
            .await;
    }

    // TODO: Return results of insert so lookup is unnecessary.
    get_movie(movie_id).await
}

pub async fn actors_for_movie(movie: &Movie, connection: &mut AsyncPgConnection) -> Vec<Actor> {
    MovieActor::belonging_to(&movie)
        .inner_join(schema::actor::table)
        .select(schema::actor::all_columns)
        .load::<Actor>(connection)
        .await
        .unwrap_or_else(|_| Vec::new())
}

pub async fn genres_for_movie(movie: &Movie, connection: &mut AsyncPgConnection) -> Vec<String> {
    MovieGenre::belonging_to(&movie)
        .select(movie_genre::genre)
        .load::<String>(connection)
        .await
        .unwrap_or_else(|_| Vec::new())
}
