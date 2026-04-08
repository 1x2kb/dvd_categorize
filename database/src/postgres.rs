use async_trait::async_trait;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use log::debug;
use models::{
    schema, Actor, Director, FullMovie, Movie, MovieActor, MovieGenre, NewActor, NewDirector,
    NewMovie, NewMovieActor, NewMovieGenre, StructuredQuery,
};
use pgvector::VectorExpressionMethods;
use std::collections::HashMap;

use crate::traits::*;
use crate::{structured_search, DatabaseError};

pub struct PostgresMovieRepository {
    connection: AsyncPgConnection,
}

impl PostgresMovieRepository {
    pub fn new(connection: AsyncPgConnection) -> Self {
        Self { connection }
    }

    pub fn connection_mut(&mut self) -> &mut AsyncPgConnection {
        &mut self.connection
    }
}

// Movie trait implementations

#[async_trait]
impl GetAllMovies for PostgresMovieRepository {
    async fn get_all(&mut self) -> Result<Vec<FullMovie>, DatabaseError> {
        let movies = schema::movie::table
            .left_join(schema::director::table)
            .select((
                schema::movie::all_columns,
                schema::director::all_columns.nullable(),
            ))
            .load::<(
                Movie,
                Option<Director>,
            )>(&mut self.connection)
            .await
            .map_err(DatabaseError::from)?;

        let mut full_movies = Vec::new();
        for (movie, director) in movies {
            let actors = MovieActor::belonging_to(&movie)
                .inner_join(schema::actor::table)
                .select(schema::actor::all_columns)
                .load::<Actor>(&mut self.connection)
                .await
                .unwrap_or_default();

            let genres = MovieGenre::belonging_to(&movie)
                .select(schema::movie_genre::genre)
                .load::<String>(&mut self.connection)
                .await
                .unwrap_or_default();

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
}

#[async_trait]
impl GetMovieById for PostgresMovieRepository {
    async fn get_by_id(&mut self, id: i32) -> Result<FullMovie, DatabaseError> {
        let (movie, director) = schema::movie::table
            .find(id)
            .left_join(schema::director::table)
            .select((
                schema::movie::all_columns,
                schema::director::all_columns.nullable(),
            ))
            .first::<(
                Movie,
                Option<Director>,
            )>(&mut self.connection)
            .await?;

        let actors = crate::actors_for_movie(
            &movie,
            &mut self.connection,
        )
        .await;
        let genres = crate::genres_for_movie(
            &movie,
            &mut self.connection,
        )
        .await;

        Ok(
            FullMovie::from((
                movie, director, actors, genres,
            )),
        )
    }
}

#[async_trait]
impl GetMoviesByIds for PostgresMovieRepository {
    async fn get_by_ids(&mut self, ids: Vec<i32>) -> Result<Vec<FullMovie>, DatabaseError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        // Get base movie data with directors (1:1 relationship via LEFT JOIN)
        let movies_with_directors = schema::movie::table
            .filter(schema::movie::id.eq_any(&ids))
            .left_join(schema::director::table)
            .load::<(
                Movie,
                Option<Director>,
            )>(&mut self.connection)
            .await?;

        if movies_with_directors.is_empty() {
            return Ok(Vec::new());
        }

        // Extract movie references for belonging_to queries
        let movies: Vec<&Movie> = movies_with_directors
            .iter()
            .map(|(movie, _)| movie)
            .collect();

        // Use belonging_to to get all movie_actor associations, then join with actors
        let movie_actors = MovieActor::belonging_to(&movies)
            .inner_join(schema::actor::table)
            .select((
                schema::movie_actor::all_columns,
                schema::actor::all_columns,
            ))
            .load::<(
                MovieActor,
                Actor,
            )>(&mut self.connection)
            .await?;

        // Use belonging_to to get all genre associations
        let movie_genres = MovieGenre::belonging_to(&movies)
            .load::<MovieGenre>(&mut self.connection)
            .await?;

        // Group actors by movie using Diesel's grouped_by
        let actors_per_movie = movie_actors
            .grouped_by(&movies)
            .into_iter()
            .map(
                |group| {
                    group
                        .into_iter()
                        .map(|(_, actor)| actor)
                        .collect::<Vec<_>>()
                },
            )
            .collect::<Vec<_>>();

        // Group genres by movie using Diesel's grouped_by
        let genres_per_movie = movie_genres
            .grouped_by(&movies)
            .into_iter()
            .map(
                |group| {
                    group
                        .into_iter()
                        .map(|mg| mg.genre)
                        .collect::<Vec<_>>()
                },
            )
            .collect::<Vec<_>>();

        // Build a map to preserve the requested order
        let mut movies_map: HashMap<i32, FullMovie> = movies_with_directors
            .into_iter()
            .zip(actors_per_movie)
            .zip(genres_per_movie)
            .map(
                |(((movie, director), actors), genres)| {
                    // Generate hash from display name for consistency
                    let display_name = match movie.release_year {
                        Some(year) => format!("{} ({})", movie.name, year),
                        None => movie.name.clone(),
                    };
                    let full_movie = FullMovie {
                        id: movie.id,
                        key_hash: FullMovie::generate_key_hash(&display_name),
                        name: movie.name,
                        director,
                        description: movie.description,
                        actors,
                        genres,
                        embedding: movie
                            .embedding
                            .map(|v| v.into()),
                        added_on: Some(
                            movie
                                .added_on
                                .to_string(),
                        ),
                        location: Some(movie.location),
                        release_year: movie.release_year,
                    };
                    (
                        movie.id, full_movie,
                    )
                },
            )
            .collect();

        // Return movies in the order they were requested
        let results: Vec<FullMovie> = ids
            .into_iter()
            .filter_map(|id| movies_map.remove(&id))
            .collect();

        Ok(results)
    }
}

#[async_trait]
impl InsertMovie for PostgresMovieRepository {
    async fn insert(&mut self, full_movie: FullMovie) -> Result<FullMovie, DatabaseError> {
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
                    .get_result(&mut self.connection)
                    .await?;

                Some(director_id)
            }
            None => None,
        };

        // Get embedding for insert.
        let embedding = match ai_chat::get_embedding(&embedding).await {
            Ok(em) => {
                debug!(
                    "Successfully generated embedding for movie: {}",
                    full_movie.name
                );
                Some(em)
            }
            Err(e) => {
                log::error!(
                    "Failed to generate embedding for movie '{}': {:#?}",
                    full_movie.name,
                    e
                );
                None
            }
        };

        let new_movie = NewMovie {
            name: full_movie.name,
            director_id,
            description: full_movie.description,
            embedding: embedding.map(|v| v.into()),
            added_on: None,
            location: full_movie.location,
            release_year: full_movie.release_year,
        };

        let movie_id = diesel::insert_into(schema::movie::table)
            .values(&new_movie)
            .returning(schema::movie::id)
            .get_result::<i32>(&mut self.connection)
            .await?;

        let actors: Vec<NewActor> = full_movie
            .actors
            .into_iter()
            .map(|actor| NewActor { name: actor.name })
            .collect();

        let actor_ids = diesel::insert_into(schema::actor::table)
            .values(&actors)
            .on_conflict(schema::actor::name)
            .do_update()
            .set(schema::actor::id.eq(schema::actor::id))
            .returning(schema::actor::id)
            .get_results::<i32>(&mut self.connection)
            .await?;

        let new_actor_movies: Vec<NewMovieActor> = actor_ids
            .into_iter()
            .map(|actor_id| NewMovieActor { movie_id, actor_id })
            .collect();

        diesel::insert_into(schema::movie_actor::table)
            .values(&new_actor_movies)
            .execute(&mut self.connection)
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

            diesel::insert_into(schema::movie_genre::table)
                .values(&movie_genres)
                .execute(&mut self.connection)
                .await?;
        }

        GetMovieById::get_by_id(
            self, movie_id,
        )
        .await
    }
}

#[async_trait]
impl InsertMovies for PostgresMovieRepository {
    async fn insert_batch(
        &mut self,
        new_movies: &[NewMovie],
    ) -> Result<
        Vec<(
            i32,
            String,
        )>,
        DatabaseError,
    > {
        diesel::insert_into(schema::movie::table)
            .values(new_movies)
            .on_conflict(schema::movie::name)
            .do_update()
            .set(schema::movie::id.eq(schema::movie::id))
            .returning((
                schema::movie::id,
                schema::movie::name,
            ))
            .get_results(&mut self.connection)
            .await
            .map_err(DatabaseError::from)
    }
}

#[async_trait]
impl UpdateMovieLocation for PostgresMovieRepository {
    async fn update_location(
        &mut self,
        movie_id: i32,
        new_location: String,
    ) -> Result<(), DatabaseError> {
        diesel::update(schema::movie::table.find(movie_id))
            .set(schema::movie::location.eq(new_location))
            .execute(&mut self.connection)
            .await
            .map_err(DatabaseError::from)?;

        Ok(())
    }
}

#[async_trait]
impl SearchMoviesByEmbedding for PostgresMovieRepository {
    async fn search_by_embedding(
        &mut self,
        embedding: Vec<f32>,
        limit: i64,
    ) -> Result<Vec<FullMovie>, DatabaseError> {
        let movie_ids: Vec<i32> = schema::movie::table
            .select(schema::movie::id)
            .filter(schema::movie::embedding.is_not_null())
            .order(schema::movie::embedding.cosine_distance(pgvector::Vector::from(embedding)))
            .limit(limit)
            .load::<i32>(&mut self.connection)
            .await?;

        debug!(
            "pgvector returned movie IDs in order: {:?}",
            movie_ids
                .iter()
                .take(10)
                .collect::<Vec<_>>()
        );

        GetMoviesByIds::get_by_ids(
            self, movie_ids,
        )
        .await
    }
}

#[async_trait]
impl GetRecentMovies for PostgresMovieRepository {
    async fn get_recent(&mut self, limit: i64) -> Result<Vec<FullMovie>, DatabaseError> {
        let movie_ids: Vec<i32> = schema::movie::table
            .select(schema::movie::id)
            .order(schema::movie::added_on.desc())
            .limit(limit)
            .load::<i32>(&mut self.connection)
            .await?;

        debug!(
            "Found {} recent movies",
            movie_ids.len()
        );

        GetMoviesByIds::get_by_ids(
            self, movie_ids,
        )
        .await
    }
}

#[async_trait]
impl RandomMovies for PostgresMovieRepository {
    async fn get_random(&mut self, count: i64) -> Result<Vec<FullMovie>, DatabaseError> {
        let movie_ids: Vec<i32> = schema::movie::table
            .select(schema::movie::id)
            .order(diesel::dsl::sql::<diesel::sql_types::Integer>("RANDOM()"))
            .limit(count)
            .load::<i32>(&mut self.connection)
            .await?;

        debug!(
            "Found {} random movies",
            movie_ids.len()
        );

        GetMoviesByIds::get_by_ids(
            self, movie_ids,
        )
        .await
    }
}

#[async_trait]
impl GetUnknownLocationMovies for PostgresMovieRepository {
    async fn get_unknown_location(&mut self, limit: i64) -> Result<Vec<FullMovie>, DatabaseError> {
        let movie_ids: Vec<i32> = schema::movie::table
            .select(schema::movie::id)
            .filter(schema::movie::location.eq("Unknown"))
            .order(schema::movie::added_on.desc())
            .limit(limit)
            .load::<i32>(&mut self.connection)
            .await?;

        debug!(
            "Found {} movies with unknown location",
            movie_ids.len()
        );

        GetMoviesByIds::get_by_ids(
            self, movie_ids,
        )
        .await
    }
}

#[async_trait]
impl SearchMoviesStructured for PostgresMovieRepository {
    async fn search_structured(
        &mut self,
        query: &StructuredQuery,
    ) -> Result<Vec<FullMovie>, DatabaseError> {
        structured_search::search_movies_structured(
            query,
            &mut self.connection,
        )
        .await
        .map_err(DatabaseError::from)
    }
}

pub struct PostgresActorRepository {
    connection: AsyncPgConnection,
}

impl PostgresActorRepository {
    pub fn new(connection: AsyncPgConnection) -> Self {
        Self { connection }
    }
}

// Actor trait implementations

#[async_trait]
impl InsertActors for PostgresActorRepository {
    async fn insert_batch(
        &mut self,
        actors: &[NewActor],
    ) -> Result<
        Vec<(
            i32,
            String,
        )>,
        DatabaseError,
    > {
        diesel::insert_into(schema::actor::table)
            .values(actors)
            .on_conflict(schema::actor::name)
            .do_update()
            .set(schema::actor::id.eq(schema::actor::id))
            .returning((
                schema::actor::id,
                schema::actor::name,
            ))
            .get_results::<(
                i32,
                String,
            )>(&mut self.connection)
            .await
            .map_err(DatabaseError::from)
    }
}

#[async_trait]
impl GetActorsForMovie for PostgresActorRepository {
    async fn get_for_movie(&mut self, movie: &Movie) -> Result<Vec<Actor>, DatabaseError> {
        Ok(
            crate::actors_for_movie(
                movie,
                &mut self.connection,
            )
            .await,
        )
    }
}

#[async_trait]
impl InsertMovieActorAssociations for PostgresActorRepository {
    async fn insert_movie_associations(
        &mut self,
        movie_actors: &[NewMovieActor],
    ) -> Result<usize, DatabaseError> {
        diesel::insert_into(schema::movie_actor::table)
            .values(movie_actors)
            .on_conflict((
                schema::movie_actor::movie_id,
                schema::movie_actor::actor_id,
            ))
            .do_nothing()
            .execute(&mut self.connection)
            .await
            .map_err(DatabaseError::from)
    }
}

pub struct PostgresDirectorRepository {
    connection: AsyncPgConnection,
}

impl PostgresDirectorRepository {
    pub fn new(connection: AsyncPgConnection) -> Self {
        Self { connection }
    }
}

// Director trait implementations

#[async_trait]
impl InsertDirectors for PostgresDirectorRepository {
    async fn insert_batch(
        &mut self,
        directors: &[NewDirector],
    ) -> Result<
        Vec<(
            i32,
            String,
        )>,
        DatabaseError,
    > {
        use schema::director::dsl::*;

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
                )>(&mut self.connection)
                .await?;

            result.push(inserted);
        }

        Ok(result)
    }
}

pub struct PostgresGenreRepository {
    connection: AsyncPgConnection,
}

impl PostgresGenreRepository {
    pub fn new(connection: AsyncPgConnection) -> Self {
        Self { connection }
    }
}

// Genre trait implementations

#[async_trait]
impl GetAllGenres for PostgresGenreRepository {
    async fn get_all(&mut self) -> Result<Vec<String>, DatabaseError> {
        use schema::movie_genre::dsl::*;

        movie_genre
            .select(genre)
            .distinct()
            .load::<String>(&mut self.connection)
            .await
            .map_err(DatabaseError::from)
    }
}

#[async_trait]
impl GetGenresForMovie for PostgresGenreRepository {
    async fn get_for_movie(&mut self, movie: &Movie) -> Result<Vec<String>, DatabaseError> {
        Ok(
            crate::genres_for_movie(
                movie,
                &mut self.connection,
            )
            .await,
        )
    }
}

#[async_trait]
impl InsertMovieGenreAssociations for PostgresGenreRepository {
    async fn insert_movie_associations(
        &mut self,
        movie_genres: &[NewMovieGenre],
    ) -> Result<usize, DatabaseError> {
        diesel::insert_into(schema::movie_genre::table)
            .values(movie_genres)
            .on_conflict((
                schema::movie_genre::movie_id,
                schema::movie_genre::genre,
            ))
            .do_nothing()
            .execute(&mut self.connection)
            .await
            .map_err(DatabaseError::from)
    }
}
