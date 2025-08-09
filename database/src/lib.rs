use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fmt::Display;

use diesel::dsl::{any, exists}; // Function-style exists for subqueries
use diesel::prelude::*;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
use log::{debug, error};
use models::dvd_filters::DvdFilters;
pub use models::{schema::*, *};
use pgvector::{Vector, VectorExpressionMethods};

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

// TODO: Setup pool in main.rs?
async fn get_database_connection() -> Result<AsyncPgConnection, DatabaseError> {
    let database_url =
        env::var("DATABASE_URL").expect("No database information found, cannot connect");
    AsyncPgConnection::establish(&database_url)
        .await
        .map_err(DatabaseError::from)
}

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

/// Get all unique actor names from the database
pub async fn get_all_actors() -> Result<Vec<String>, DatabaseError> {
    use crate::schema::actor::dsl::*;
    let mut conn = get_database_connection().await?;
    
    actor
        .select(name)
        .distinct()
        .load::<String>(&mut conn)
        .await
        .map_err(DatabaseError::from)
}

/// Get all unique director names from the database
pub async fn get_all_directors() -> Result<Vec<String>, DatabaseError> {
    use crate::schema::director::dsl::*;
    let mut conn = get_database_connection().await?;
    
    director
        .select(name)
        .distinct()
        .load::<String>(&mut conn)
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

pub async fn get_movies_by_ids(ids: Vec<i32>) -> Result<Vec<FullMovie>, DatabaseError> {
    let mut connection = get_database_connection()
        .await
        .map_err(DatabaseError::from)?;

    // Early return for empty input
    if ids.is_empty() {
        return Ok(Vec::new());
    }

    // Batch get movies and directors
    let movies_with_directors = schema::movie::table
        .filter(schema::movie::id.eq_any(&ids))
        .left_join(schema::director::table)
        .load::<(
            Movie,
            Option<Director>,
        )>(&mut connection)
        .await?;

    // Early return if no movies found
    if movies_with_directors.is_empty() {
        return Ok(Vec::new());
    }

    // Use original ids for batch queries (only query for movies that actually exist)
    let existing_movie_ids: Vec<i32> = movies_with_directors
        .iter()
        .map(|(movie, _)| movie.id)
        .collect();

    // Batch get all actors for all movies
    let all_actors = schema::movie_actor::table
        .filter(schema::movie_actor::movie_id.eq_any(&existing_movie_ids))
        .inner_join(schema::actor::table)
        .load::<(MovieActor, Actor)>(&mut connection)
        .await?;

    // Batch get all genres for all movies
    let all_genres = schema::movie_genre::table
        .filter(schema::movie_genre::movie_id.eq_any(&existing_movie_ids))
        .load::<MovieGenre>(&mut connection)
        .await?;

    // Pre-allocate HashMaps with capacity for better performance
    let mut actors_map: HashMap<i32, Vec<Actor>> = HashMap::with_capacity(existing_movie_ids.len());
    let mut genres_map: HashMap<i32, Vec<String>> = HashMap::with_capacity(existing_movie_ids.len());

    // Build lookup maps
    for (ma, actor) in all_actors {
        actors_map
            .entry(ma.movie_id)
            .or_default()
            .push(actor);
    }

    for mg in all_genres {
        genres_map
            .entry(mg.movie_id)
            .or_default()
            .push(mg.genre);
    }

    // Assemble final results - avoid cloning by using remove() instead of get().cloned()
    let results = movies_with_directors
        .into_iter()
        .map(
            |(movie, director)| FullMovie {
                id: movie.id,
                name: movie.name,
                director,
                description: movie.description,
                actors: actors_map
                    .remove(&movie.id)
                    .unwrap_or_default(),
                genres: genres_map
                    .remove(&movie.id)
                    .unwrap_or_default(),
            },
        )
        .collect();

    Ok(results)
}

pub async fn insert_full_movie(full_movie: FullMovie) -> Result<FullMovie, DatabaseError> {
    let mut conn = get_database_connection()
        .await
        .map_err(DatabaseError::from)?;

    let actors: String = full_movie
        .actors
        .iter()
        .map(
            |actor| {
                actor
                    .name
                    .as_str()
            },
        )
        .collect::<Vec<_>>()
        .join(",");

    let genres: String = full_movie
        .genres
        .iter()
        .map(|genre| genre.as_str())
        .collect::<Vec<_>>()
        .join(",");

    // Create string data to be vectorized later.
    let embedding = format!(
        "{}-{} and has genres {} with actors {} and directed by {}",
        &full_movie.name,
        &full_movie
            .description
            .as_ref()
            .unwrap_or(&"".to_string()),
        genres,
        actors,
        full_movie
            .director
            .as_ref()
            .map(
                |director| director
                    .name
                    .as_str()
            )
            .unwrap_or("")
    );

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

    // Vectorize string for storage in the database.
    let embedding = match ai_chat::get_embedding(&embedding).await {
        Ok(em) => {
            debug!("Successfully generated embedding for movie: {}", full_movie.name);
            Some(Vector::from(em))
        },
        Err(e) => {
            error!("Failed to generate embedding for movie '{}': {:#?}", full_movie.name, e);
            None
        }
    };

    let new_movie = NewMovie {
        name: full_movie.name,
        director_id,
        description: full_movie.description,
        embedding,
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

pub async fn run_dvd_filters(filters: DvdFilters) -> Result<Vec<i32>, DatabaseError> {
    // Base query with joins needed for nullable director
    let mut query = movie::table
        .left_join(director::table)
        .into_boxed();

    // Apply filters conditionally
    if let Some(names) = filters.movie_names {
        query = query.filter(movie::name.eq_any(names));
    }

    if let Some(directors) = filters.directors {
        query = query.filter(director::name.eq_any(directors));
    }

    // Actor filter using EXISTS subquery
    if let Some(actors) = filters.actors {
        query = query.filter(
            exists(
                movie_actor::table
                    .inner_join(actor::table)
                    .filter(movie_actor::movie_id.eq(movie::id))
                    .filter(actor::name.eq_any(actors)),
            ),
        );
    }

    // Genre filter using EXISTS subquery
    if let Some(genres) = filters.genres {
        query = query.filter(
            exists(
                movie_genre::table
                    .filter(movie_genre::movie_id.eq(movie::id))
                    .filter(movie_genre::genre.eq_any(genres)),
            ),
        );
    }

    if let Some(terms) = filters.plot_terms {
        let search_patterns = terms
            .iter()
            .map(
                |t| {
                    format!(
                        "%{}%",
                        t
                    )
                },
            )
            .collect::<Vec<_>>();

        query = query.filter(movie::description.ilike(any(search_patterns)));
    }

    let mut conn = get_database_connection().await?;

    query
        .select(movie::id)
        .distinct()
        .get_results(&mut conn)
        .await
        .map_err(DatabaseError::from)
}

pub async fn search_movies(embedding: Vec<f32>, limit: i64) -> Result<Vec<FullMovie>, DatabaseError> {
    let mut conn = get_database_connection().await?;

    // Get the IDs of the most similar movies
    let movie_ids: Vec<i32> = movie::table
        .select(movie::id)
        .filter(movie::embedding.is_not_null())
        .order(movie::embedding.cosine_distance(Vector::from(embedding)))
        .limit(limit)
        .load::<i32>(&mut conn)
        .await?;

    // Use the optimized helper function to get full movie data
    get_movies_by_ids(movie_ids).await
}
