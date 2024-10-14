pub mod models;
mod schema;

use std::env;
use std::error::Error;

pub use diesel::prelude::*;
pub use diesel::BelongingToDsl;
pub use models::*;
use schema::director;
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

pub fn get_movie(id: i32) -> Result<FullMovie, Box<dyn Error>> {
    let mut connection = get_database_connection()?;

    let (movie, director) = schema::movie::table
        .find(id)
        .left_join(schema::director::table)
        .get_result::<(Movie, Option<Director>)>(&mut connection)?;

    let actors = MovieActor::belonging_to(&movie)
        .inner_join(schema::actor::table)
        .select(schema::actor::all_columns)
        .load::<Actor>(&mut connection)
        .unwrap_or_else(|_| Vec::new());

    let genres = MovieGenre::belonging_to(&movie)
        .select(movie_genre::genre)
        .load::<String>(&mut connection)
        .unwrap_or_else(|_| Vec::new());

    Ok(FullMovie::from((movie, director, actors, genres)))
}

pub fn insert_full_movie(full_movie: FullMovie) -> Result<FullMovie, Box<dyn Error>> {
    let mut conn = get_database_connection()?;

    // Director is optional, insert or select id for director name.
    let director_id: Option<i32> = match full_movie.director {
        Some(director) => {
            let new_director = NewDirector {
                name: director.name,
            };

            let director_id: i32 = diesel::insert_into(schema::director::table)
                .values(&new_director)
                .on_conflict(schema::director::name)
                .do_nothing()
                .returning(schema::director::id)
                .get_result(&mut conn)?;

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
        .get_result::<i32>(&mut conn)?;

    let actors: Vec<NewActor> = full_movie
        .actors
        .into_iter()
        .map(|actor| NewActor { name: actor.name })
        .collect();

    let actors = diesel::insert_into(schema::actor::table)
        .values(&actors)
        .on_conflict(schema::actor::name)
        .do_nothing()
        .returning(schema::actor::id)
        .get_results::<i32>(&mut conn)?;

    let new_actor_movies: Vec<NewMovieActor> = actors
        .into_iter()
        .map(|actor_id| NewMovieActor { movie_id, actor_id })
        .collect();

    let _ = diesel::insert_into(schema::movie_actor::table)
        .values(&new_actor_movies)
        .execute(&mut conn)?;

    if !full_movie.genres.is_empty() {
        let movie_genres: Vec<NewMovieGenre> = full_movie
            .genres
            .into_iter()
            .map(|genre| NewMovieGenre { movie_id, genre })
            .collect();

        let _ = diesel::insert_into(schema::movie_genre::table)
            .values(&movie_genres)
            .execute(&mut conn);
    }

    // TODO: Return results of insert so lookup is unnecessary.
    get_movie(movie_id)
}

// pub fn insert_movie()
