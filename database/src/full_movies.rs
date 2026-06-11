use std::{collections::HashSet, error::Error};

use diesel::ConnectionError;
use diesel_async::pooled_connection::deadpool::Pool;
use diesel_async::AsyncPgConnection;

use chrono::NaiveDate;
use log::error;
use models::{FullMovie, NewActor, NewDirector, NewMovie, NewMovieActor, NewMovieGenre};

fn find_movie_id(
    movie_name: &str,
    movie_inserts: &[(
        i32,
        String,
    )],
) -> Option<i32> {
    movie_inserts
        .binary_search_by(
            |(_, name)| {
                name.as_str()
                    .cmp(movie_name)
            },
        )
        .ok()
        .and_then(
            |idx| {
                movie_inserts
                    .get(idx)
                    .map(|(id, _)| *id)
            },
        )
}

///
/// Adds a Vec of FullMovies in bulk to the database.
///
/// TODO: Needs refactor
pub async fn insert_full_movies(
    mut full_movies: Vec<FullMovie>,
    pool: &Pool<AsyncPgConnection>,
) -> Result<(), Box<dyn Error>> {
    // Validate all dates before proceeding with insert
    for (index, movie) in full_movies
        .iter()
        .enumerate()
    {
        if let Some(date_str) = &movie.added_on {
            let date_result = parse_date(
                date_str,
                &movie.name,
            );

            if date_result.is_err() {
                error!(
                    "Movie '{}' at index '{}' had a date parsing issue",
                    movie.name, index
                );
            }

            date_result?
        }
    }

    let (actors, _genres, directors) = (
        get_unique_actors(&full_movies),
        get_unique_genres(&full_movies),
        get_unique_directors(&full_movies),
    );

    let mut connection = pool
        .get()
        .await
        .map_err(
            |e| {
                crate::DatabaseError::ConnectionError(ConnectionError::BadConnection(e.to_string()))
            },
        )?;
    let mut actors = crate::insert_actors(
        &actors,
        &mut connection,
    )
    .await?;
    actors.sort_by(
        |a, b| {
            a.1.cmp(&b.1)
        },
    );

    let mut directors = crate::insert_directors(
        &directors,
        &mut connection,
    )
    .await?;
    directors.sort_by(
        |a, b| {
            a.1.cmp(&b.1)
        },
    );

    let movies: Vec<NewMovie> = full_movies
        .iter_mut()
        .map(
            |movie| NewMovie {
                name: movie
                    .name
                    .to_string(),
                director_id: movie
                    .director
                    .iter()
                    .flat_map(
                        |director| {
                            directors
                                .binary_search_by(
                                    |(_, director_name)| director_name.cmp(&director.name),
                                )
                                .ok()
                                .and_then(
                                    |index| {
                                        directors
                                            .get(index)
                                            .map(|(id, _)| *id)
                                    },
                                )
                        },
                    )
                    .next(),
                description: movie
                    .description
                    .clone(),
                embedding: movie
                    .embedding
                    .take()
                    .map(|v| v.into()),
                added_on: movie
                    .added_on
                    .as_deref()
                    .and_then(
                        |date_str| {
                            crate::parse_added_on_date(
                                date_str,
                                &movie.name,
                            )
                        },
                    ),
                location: movie
                    .location
                    .clone(),
                release_year: movie.release_year,
            },
        )
        .collect();

    let mut movie_inserts = crate::insert_movies(
        &movies,
        &mut connection,
    )
    .await?;

    movie_inserts.sort_by(
        |a, b| {
            a.1.cmp(&b.1)
        },
    );

    let movie_actors: Vec<NewMovieActor> = full_movies
        .iter()
        .flat_map(
            |movie| {
                let Some(movie_id) = find_movie_id(
                    &movie.name,
                    &movie_inserts,
                ) else {
                    return Vec::new();
                };

                movie
                    .actors
                    .iter()
                    .enumerate()
                    .filter_map(
                        |(index, actor)| {
                            actors
                                .binary_search_by(|(_, name)| name.cmp(&actor.name))
                                .ok()
                                .and_then(|actor_index| actors.get(actor_index))
                                .map(
                                    |(actor_id, _)| NewMovieActor {
                                        movie_id,
                                        actor_id: *actor_id,
                                        actor_order: (index + 1) as i32,
                                    },
                                )
                        },
                    )
                    .collect()
            },
        )
        .collect();

    let _movie_actors = crate::insert_movie_actors(
        &movie_actors,
        &mut connection,
    )
    .await?;

    let movie_genres: Vec<NewMovieGenre> = full_movies
        .iter()
        .flat_map(
            |movie| {
                let Some(movie_id) = find_movie_id(
                    &movie.name,
                    &movie_inserts,
                ) else {
                    return Vec::new();
                };

                movie
                    .genres
                    .iter()
                    .map(
                        |genre| NewMovieGenre {
                            movie_id,
                            genre: genre.clone(),
                        },
                    )
                    .collect()
            },
        )
        .collect();

    let _movie_genres: Vec<models::MovieGenre> = crate::insert_movie_genres(
        &movie_genres,
        &mut connection,
    )
    .await?;

    Ok(())
}

fn parse_date(date_str: &str, movie_name: &str) -> Result<(), Box<dyn Error>> {
    // Try parsing as full timestamp first, then fall back to date-only
    let valid = chrono::NaiveDateTime::parse_from_str(
        date_str,
        "%Y-%m-%d %H:%M:%S%.f",
    )
    .is_ok()
        || NaiveDate::parse_from_str(
            date_str, "%Y-%m-%d",
        )
        .map(
            |d| {
                d.and_hms_opt(
                    0, 0, 0,
                )
                .is_some()
            },
        )
        .unwrap_or(false);

    if !valid {
        return Err(
            format!(
                "Invalid date '{}' for movie '{}'",
                date_str, movie_name
            )
            .into(),
        );
    }

    Ok(())
}

fn get_unique_actors(movies: &[FullMovie]) -> Vec<NewActor> {
    movies
        .iter()
        .flat_map(
            |movie| {
                movie
                    .actors
                    .iter()
                    .map(
                        |actor| {
                            actor
                                .name
                                .as_str()
                        },
                    )
            },
        )
        .collect::<HashSet<&str>>()
        .into_iter()
        .map(
            |actor_name| NewActor {
                name: actor_name.to_string(),
            },
        )
        .collect()
}

fn get_unique_genres(movies: &[FullMovie]) -> HashSet<&str> {
    movies
        .iter()
        .flat_map(
            |movie| {
                movie
                    .genres
                    .iter()
                    .map(|genre| genre.as_str())
            },
        )
        .collect()
}

fn get_unique_directors(movies: &[FullMovie]) -> Vec<NewDirector> {
    movies
        .iter()
        .filter_map(
            |movie| {
                movie
                    .director
                    .as_ref()
                    .map(
                        |d| {
                            d.name
                                .as_str()
                        },
                    )
            },
        )
        .collect::<HashSet<&str>>()
        .into_iter()
        .map(
            |director_name| NewDirector {
                name: director_name.to_string(),
            },
        )
        .collect()
}
