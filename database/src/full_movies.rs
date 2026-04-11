use std::{collections::HashSet, error::Error};

use models::{FullMovie, NewActor, NewDirector, NewMovie, NewMovieActor, NewMovieGenre};

///
/// Adds a Vec of FullMovies in bulk to the database.
///
/// TODO: Needs refactor
pub async fn insert_full_movies(mut full_movies: Vec<FullMovie>) -> Result<(), Box<dyn Error>> {
    let (mut actors, _genres, mut directors) = (
        get_unique_actors(&full_movies),
        get_unique_genres(&full_movies),
        get_unique_directors(&full_movies),
    );

    // Sort results for binary search
    actors.sort_by(
        |a, b| {
            a.name
                .cmp(&b.name)
        },
    );
    directors.sort_by(
        |a, b| {
            a.name
                .cmp(&b.name)
        },
    );

    let mut connection = crate::get_database_connection().await?;
    let actors = crate::insert_actors(
        &actors,
        &mut connection,
    )
    .await?;
    let directors = crate::insert_directors(
        &directors,
        &mut connection,
    )
    .await?;

    let embeddings: Vec<String> = full_movies
        .iter()
        .map(|movie| movie.embedding_str())
        .collect();
    let embeddings = ai_chat::get_embeddings(
        embeddings,
        ai_chat::EMBEDDING_MODEL,
    )
    .await?;

    let movies: Vec<NewMovie> = full_movies
        .iter_mut()
        .zip(embeddings.into_iter())
        .map(
            |(movie, embedding)| NewMovie {
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
                embedding: Some(embedding.into()),
                added_on: None,
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
                let movie_id = movie_inserts
                    .binary_search_by(|(_, movie_name)| movie_name.cmp(&movie.name))
                    .ok()
                    .and_then(
                        |found_index| {
                            movie_inserts
                                .get(found_index)
                                .map(|(id, _)| *id)
                        },
                    );

                if let Some(movie_id) = movie_id {
                    return movie
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
                        .collect();
                }

                Vec::new()
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
                let movie_id = movie_inserts
                    .binary_search_by(|(_, movie_name)| movie_name.cmp(&movie.name))
                    .ok()
                    .and_then(
                        |found_index| {
                            movie_inserts
                                .get(found_index)
                                .map(|(id, _)| *id)
                        },
                    );

                if let Some(movie_id) = movie_id {
                    return movie
                        .genres
                        .iter()
                        .map(
                            |genre| NewMovieGenre {
                                movie_id,
                                genre: genre.clone(),
                            },
                        )
                        .collect();
                }

                Vec::new()
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
