//! Postgres-specific database implementations
//!
//! This module contains all Diesel/Postgres-specific code and is feature-gated
//! behind the "postgres" feature flag.

// Re-export shared types that are always available
pub use models::{Actor, Director, FullMovie, Movie};

// All postgres-specific code is wrapped in a single feature-gated module
#[cfg(feature = "postgres")]
mod inner {
    pub use super::{Actor, Director, FullMovie, Movie};

    // Postgres-specific imports
    use async_trait::async_trait;
    use diesel::prelude::*;
    use diesel_async::pooled_connection::deadpool::{Object, Pool};
    use diesel_async::{AsyncPgConnection, RunQueryDsl};
    use log::debug;
    use models::{
        schema, MovieActor, MovieGenre, NewActor, NewDirector, NewMovie, NewMovieActor,
        NewMovieGenre, StructuredQuery,
    };
    #[cfg(feature = "ai")]
    use pgvector::VectorExpressionMethods;
    use std::collections::HashMap;

    use crate::traits::*;
    use crate::DatabaseError;

    fn build_full_movie_from_row(
        movie: Movie,
        director: Option<Director>,
        actors: Vec<Actor>,
        genres: Vec<String>,
    ) -> FullMovie {
        let display_name = if movie.release_year == 0 {
            movie
                .name
                .clone()
        } else {
            format!(
                "{} ({})",
                movie.name, movie.release_year
            )
        };
        FullMovie {
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
        }
    }

    // All postgres repository implementations are feature-gated
    #[derive(Clone)]
    pub struct PostgresMovieRepository {
        pool: Pool<AsyncPgConnection>,
    }

    impl PostgresMovieRepository {
        pub fn new(pool: Pool<AsyncPgConnection>) -> Self {
            Self { pool }
        }

        pub fn pool(&self) -> &Pool<AsyncPgConnection> {
            &self.pool
        }

        async fn get_conn(&self) -> Result<Object<AsyncPgConnection>, DatabaseError> {
            self.pool
                .get()
                .await
                .map_err(
                    |e| {
                        DatabaseError::ConnectionError(
                            diesel::ConnectionError::BadConnection(e.to_string()),
                        )
                    },
                )
        }
    }

    impl std::fmt::Debug for PostgresMovieRepository {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("PostgresMovieRepository")
                .finish_non_exhaustive()
        }
    }

    impl PostgresMovieRepository {
        /// Builds a repository using the `DATABASE_URL` environment variable.
        pub async fn from_env() -> Result<Self, DatabaseError> {
            let pool = crate::get_connection_pool().await?;
            Ok(Self::new(pool))
        }

        /// Search movies by a single text query across title, actors, director, and genres.
        /// Uses database-level ILIKE matching with scoring.
        /// Returns (movie, score) tuples sorted by score descending.
        pub async fn search_movies_by_text(
            &self,
            query: &str,
            limit: i64,
        ) -> Result<
            Vec<(
                FullMovie,
                f32,
            )>,
            DatabaseError,
        > {
            use schema::{actor, director, movie, movie_actor, movie_genre};

            let mut conn = self
                .get_conn()
                .await?;

            if query.is_empty() {
                return Ok(Vec::new());
            }

            let pattern = format!(
                "%{}%",
                query.to_lowercase()
            );

            // Build query to find movies where query matches any field
            let matching_movies = movie::table
                .left_join(director::table.on(movie::director_id.eq(director::id.nullable())))
                .select((
                    movie::all_columns,
                    director::all_columns.nullable(),
                ))
                .filter(
                    // Match movie name
                    movie::name
                        .ilike(pattern.clone())
                        // Match actor name (via subquery)
                        .or(
                            movie::id.eq_any(
                                movie_actor::table
                                    .inner_join(
                                        actor::table.on(movie_actor::actor_id.eq(actor::id)),
                                    )
                                    .select(movie_actor::movie_id)
                                    .filter(actor::name.ilike(pattern.clone())),
                            ),
                        )
                        // Match director name
                        .or(
                            director::name
                                .is_not_null()
                                .and(director::name.ilike(pattern.clone())),
                        )
                        // Match genre (via subquery)
                        .or(
                            movie::id.eq_any(
                                movie_genre::table
                                    .select(movie_genre::movie_id)
                                    .filter(movie_genre::genre.ilike(pattern.clone())),
                            ),
                        ),
                )
                .limit(limit * 2)
                .load::<(
                    Movie,
                    Option<Director>,
                )>(&mut conn)
                .await?;

            // Bulk-load actors and genres for all matched movies in 2 queries
            let movie_refs: Vec<&Movie> = matching_movies
                .iter()
                .map(|(m, _)| m)
                .collect();

            let (mut conn_actors, mut conn_genres) = tokio::try_join!(
                self.get_conn(),
                self.get_conn()
            )?;
            let (actors_per_movie, genres_per_movie) = crate::load_actors_and_genres(
                &movie_refs,
                &mut conn_actors,
                &mut conn_genres,
            )
            .await?;

            let query_lower = query.to_lowercase();

            // Score results
            let mut scored_results: Vec<(
                FullMovie,
                f32,
            )> = matching_movies
                .into_iter()
                .zip(actors_per_movie)
                .zip(genres_per_movie)
                .filter_map(
                    |(((movie_row, director), movie_actors), movie_genres)| {
                        let mut score: f32 = 0.0;

                        // Title scoring
                        if movie_row
                            .name
                            .to_lowercase()
                            .contains(&query_lower)
                        {
                            score += 100.0;
                        }

                        // Actor scoring
                        if movie_actors
                            .iter()
                            .any(
                                |a| {
                                    a.name
                                        .to_lowercase()
                                        .contains(&query_lower)
                                },
                            )
                        {
                            score += 20.0;
                        }

                        // Director scoring
                        if let Some(ref d) = director {
                            if d.name
                                .to_lowercase()
                                .contains(&query_lower)
                            {
                                score += 25.0;
                            }
                        }

                        // Genre scoring
                        if movie_genres
                            .iter()
                            .any(
                                |g| {
                                    g.to_lowercase()
                                        .contains(&query_lower)
                                },
                            )
                        {
                            score += 15.0;
                        }

                        if score > 0.0 {
                            Some((
                                build_full_movie_from_row(
                                    movie_row,
                                    director,
                                    movie_actors,
                                    movie_genres,
                                ),
                                score,
                            ))
                        } else {
                            None
                        }
                    },
                )
                .collect();

            // Sort by score descending
            scored_results.sort_by(
                |a, b| {
                    b.1.partial_cmp(&a.1)
                        .unwrap_or(std::cmp::Ordering::Equal)
                },
            );
            scored_results.truncate(limit as usize);

            Ok(scored_results)
        }

        // Chat history methods

        pub async fn create_chat_session(&self) -> Result<uuid::Uuid, DatabaseError> {
            use models::{schema::chat_sessions, NewChatSession};

            let session_id = uuid::Uuid::new_v4();
            let new_session = NewChatSession { session_id };

            let mut conn = self
                .get_conn()
                .await?;

            diesel::insert_into(chat_sessions::table)
                .values(&new_session)
                .execute(&mut conn)
                .await?;

            Ok(session_id)
        }

        pub async fn get_chat_history(
            &self,
            session_id: uuid::Uuid,
        ) -> Result<Vec<models::ChatMessage>, DatabaseError> {
            use models::schema::chat_messages;

            let mut conn = self
                .get_conn()
                .await?;

            chat_messages::table
                .filter(chat_messages::session_id.eq(session_id))
                .order(chat_messages::created_at.asc())
                .load(&mut conn)
                .await
                .map_err(Into::into)
        }

        pub async fn save_chat_message(
            &self,
            message: models::NewChatMessage,
        ) -> Result<(), DatabaseError> {
            use models::schema::{chat_messages, chat_sessions};

            let mut conn = self
                .get_conn()
                .await?;

            diesel::insert_into(chat_messages::table)
                .values(&message)
                .execute(&mut conn)
                .await?;

            // Update session timestamp
            diesel::update(chat_sessions::table)
                .filter(chat_sessions::session_id.eq(message.session_id))
                .set(chat_sessions::updated_at.eq(diesel::dsl::now))
                .execute(&mut conn)
                .await?;

            Ok(())
        }

        pub async fn list_chat_sessions(&self) -> Result<Vec<models::ChatSession>, DatabaseError> {
            use models::schema::chat_sessions;

            let mut conn = self
                .get_conn()
                .await?;

            chat_sessions::table
                .order(chat_sessions::updated_at.desc())
                .load(&mut conn)
                .await
                .map_err(Into::into)
        }

        pub async fn search_structured(
            &self,
            query: &models::StructuredQuery,
        ) -> Result<Vec<FullMovie>, DatabaseError> {
            let mut conn = self
                .get_conn()
                .await?;

            crate::structured_search::search_movies_structured(
                query, &mut conn,
            )
            .await
            .map_err(Into::into)
        }
    }

    // Movie trait implementations

    #[async_trait]
    impl GetAllMovies for PostgresMovieRepository {
        async fn get_all(&self) -> Result<Vec<FullMovie>, DatabaseError> {
            let mut conn = self
                .get_conn()
                .await?;

            let movies = schema::movie::table
                .left_join(schema::director::table)
                .select((
                    schema::movie::all_columns,
                    schema::director::all_columns.nullable(),
                ))
                .load::<(
                    Movie,
                    Option<Director>,
                )>(&mut conn)
                .await
                .map_err(DatabaseError::from)?;

            let movie_refs: Vec<&Movie> = movies
                .iter()
                .map(|(movie, _)| movie)
                .collect();

            let (mut conn_actors, mut conn_genres) = tokio::try_join!(
                self.get_conn(),
                self.get_conn()
            )?;
            let (actors_per_movie, genres_per_movie) = crate::load_actors_and_genres(
                &movie_refs,
                &mut conn_actors,
                &mut conn_genres,
            )
            .await?;

            let full_movies: Vec<FullMovie> = movies
                .into_iter()
                .zip(actors_per_movie)
                .zip(genres_per_movie)
                .map(
                    |(((movie, director), actors), genres)| {
                        build_full_movie_from_row(
                            movie, director, actors, genres,
                        )
                    },
                )
                .collect();

            debug!(
                "Found {} results",
                full_movies.len()
            );
            Ok(full_movies)
        }
    }

    #[async_trait]
    impl GetMovieById for PostgresMovieRepository {
        async fn get_by_id(&self, id: i32) -> Result<FullMovie, DatabaseError> {
            let mut conn = self
                .get_conn()
                .await?;

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
                )>(&mut conn)
                .await?;

            let actors = actors::actors_for_movie(
                &movie, &mut conn,
            )
            .await;
            let genres = genres::genres_for_movie(
                &movie, &mut conn,
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
        async fn get_by_ids(&self, ids: Vec<i32>) -> Result<Vec<FullMovie>, DatabaseError> {
            if ids.is_empty() {
                return Ok(Vec::new());
            }

            let mut conn = self
                .get_conn()
                .await?;

            // Get base movie data with directors (1:1 relationship via LEFT JOIN)
            let movies_with_directors = schema::movie::table
                .filter(schema::movie::id.eq_any(&ids))
                .left_join(schema::director::table)
                .load::<(
                    Movie,
                    Option<Director>,
                )>(&mut conn)
                .await?;

            if movies_with_directors.is_empty() {
                return Ok(Vec::new());
            }

            // Extract movie references for belonging_to queries
            let movies: Vec<&Movie> = movies_with_directors
                .iter()
                .map(|(movie, _)| movie)
                .collect();

            let (mut conn_actors, mut conn_genres) = tokio::try_join!(
                self.get_conn(),
                self.get_conn()
            )?;
            let (actors_per_movie, genres_per_movie) = crate::load_actors_and_genres(
                &movies,
                &mut conn_actors,
                &mut conn_genres,
            )
            .await?;

            // Build a map to preserve the requested order
            let mut movies_map: HashMap<i32, FullMovie> = movies_with_directors
                .into_iter()
                .zip(actors_per_movie)
                .zip(genres_per_movie)
                .map(
                    |(((movie, director), actors), genres)| {
                        let id = movie.id;
                        (
                            id,
                            build_full_movie_from_row(
                                movie, director, actors, genres,
                            ),
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
        async fn insert(&self, full_movie: FullMovie) -> Result<FullMovie, DatabaseError> {
            let mut conn = self
                .get_conn()
                .await?;

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

            let embedding = full_movie.embedding;

            let added_on = full_movie
                .added_on
                .as_deref()
                .and_then(
                    |date_str| {
                        crate::parse_added_on_date(
                            date_str,
                            &full_movie.name,
                        )
                    },
                );

            let new_movie = NewMovie {
                name: full_movie.name,
                director_id,
                description: full_movie.description,
                embedding: embedding.map(|v| v.into()),
                added_on,
                location: full_movie.location,
                release_year: full_movie.release_year,
            };

            let movie_id = diesel::insert_into(schema::movie::table)
                .values(&new_movie)
                .on_conflict((
                    schema::movie::name,
                    schema::movie::release_year,
                ))
                .do_update()
                .set(schema::movie::id.eq(schema::movie::id))
                .returning(schema::movie::id)
                .get_result::<i32>(&mut conn)
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
                .get_results::<i32>(&mut conn)
                .await?;

            let new_actor_movies: Vec<NewMovieActor> = actor_ids
                .into_iter()
                .enumerate()
                .map(
                    |(index, actor_id)| NewMovieActor {
                        movie_id,
                        actor_id,
                        actor_order: (index + 1) as i32,
                    },
                )
                .collect();

            diesel::insert_into(schema::movie_actor::table)
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

                diesel::insert_into(schema::movie_genre::table)
                    .values(&movie_genres)
                    .execute(&mut conn)
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
            &self,
            new_movies: &[NewMovie],
        ) -> Result<
            Vec<(
                i32,
                String,
            )>,
            DatabaseError,
        > {
            let mut conn = self
                .get_conn()
                .await?;

            diesel::insert_into(schema::movie::table)
                .values(new_movies)
                .on_conflict((
                    schema::movie::name,
                    schema::movie::release_year,
                ))
                .do_update()
                .set(schema::movie::id.eq(schema::movie::id)) // No op
                .returning((
                    schema::movie::id,
                    schema::movie::name,
                ))
                .get_results(&mut conn)
                .await
                .map_err(DatabaseError::from)
        }
    }

    #[async_trait]
    impl UpdateMovieLocation for PostgresMovieRepository {
        async fn update_location(
            &self,
            movie_id: i32,
            new_location: String,
        ) -> Result<(), DatabaseError> {
            let mut conn = self
                .get_conn()
                .await?;

            diesel::update(schema::movie::table.find(movie_id))
                .set(schema::movie::location.eq(new_location))
                .execute(&mut conn)
                .await
                .map_err(DatabaseError::from)?;

            Ok(())
        }
    }

    #[cfg(feature = "ai")]
    #[async_trait]
    impl SearchMoviesByEmbedding for PostgresMovieRepository {
        async fn search_by_embedding(
            &self,
            embedding: Vec<f32>,
            limit: i64,
        ) -> Result<Vec<FullMovie>, DatabaseError> {
            let mut conn = self
                .get_conn()
                .await?;

            let movie_ids: Vec<i32> = schema::movie::table
                .select(schema::movie::id)
                .filter(schema::movie::embedding.is_not_null())
                .order(schema::movie::embedding.cosine_distance(pgvector::Vector::from(embedding)))
                .limit(limit)
                .load::<i32>(&mut conn)
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
        async fn get_recent(&self, limit: i64) -> Result<Vec<FullMovie>, DatabaseError> {
            let mut conn = self
                .get_conn()
                .await?;

            let movie_ids: Vec<i32> = schema::movie::table
                .select(schema::movie::id)
                .order(schema::movie::added_on.desc())
                .limit(limit)
                .load::<i32>(&mut conn)
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
    impl GetMoviesByReleaseYear for PostgresMovieRepository {
        async fn get_by_release_year(
            &self,
            min_year: i32,
            max_year: i32,
            limit: i64,
        ) -> Result<Vec<FullMovie>, DatabaseError> {
            let mut conn = self
                .get_conn()
                .await?;

            let movie_ids: Vec<i32> = schema::movie::table
                .select(schema::movie::id)
                .filter(schema::movie::release_year.ge(min_year))
                .filter(schema::movie::release_year.le(max_year))
                .order(schema::movie::release_year.desc())
                .limit(limit)
                .load::<i32>(&mut conn)
                .await?;

            debug!(
                "Found {} movies released between {} and {}",
                movie_ids.len(),
                min_year,
                max_year
            );

            GetMoviesByIds::get_by_ids(
                self, movie_ids,
            )
            .await
        }
    }

    #[async_trait]
    impl RandomMovies for PostgresMovieRepository {
        async fn get_random(&self, count: i64) -> Result<Vec<FullMovie>, DatabaseError> {
            let mut conn = self
                .get_conn()
                .await?;

            let movie_ids: Vec<i32> = schema::movie::table
                .select(schema::movie::id)
                .order(diesel::dsl::sql::<diesel::sql_types::Integer>("RANDOM()"))
                .limit(count)
                .load::<i32>(&mut conn)
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
        async fn get_unknown_location(&self, limit: i64) -> Result<Vec<FullMovie>, DatabaseError> {
            let mut conn = self
                .get_conn()
                .await?;

            let movie_ids: Vec<i32> = schema::movie::table
                .select(schema::movie::id)
                .filter(schema::movie::location.eq("Unknown"))
                .order(schema::movie::added_on.desc())
                .limit(limit)
                .load::<i32>(&mut conn)
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
            &self,
            query: &StructuredQuery,
        ) -> Result<Vec<FullMovie>, DatabaseError> {
            let mut conn = self
                .get_conn()
                .await?;

            structured_search::search_movies_structured(
                query, &mut conn,
            )
            .await
            .map_err(DatabaseError::from)
        }
    }

    #[async_trait]
    impl GetUniqueLocations for PostgresMovieRepository {
        async fn unique_locations(&self) -> Result<Vec<String>, DatabaseError> {
            let mut conn = self
                .get_conn()
                .await?;

            schema::movie::table
                .select(schema::movie::location)
                .distinct()
                .get_results(&mut conn)
                .await
                .map_err(DatabaseError::from)
        }
    }

    #[async_trait]
    impl MoviesByLocation for PostgresMovieRepository {
        async fn movies_by_location(
            &self,
            location: &str,
        ) -> Result<Vec<FullMovie>, DatabaseError> {
            let mut conn = self
                .get_conn()
                .await?;

            let movie_ids: Vec<i32> = schema::movie::table
                .select(schema::movie::id)
                .filter(schema::movie::location.eq(location))
                .get_results(&mut conn)
                .await
                .map_err(DatabaseError::from)?;

            GetMoviesByIds::get_by_ids(
                self, movie_ids,
            )
            .await
        }
    }

    #[derive(Clone)]
    pub struct PostgresActorRepository {
        pool: Pool<AsyncPgConnection>,
    }

    impl PostgresActorRepository {
        pub fn new(pool: Pool<AsyncPgConnection>) -> Self {
            Self { pool }
        }

        pub async fn from_env() -> Result<Self, DatabaseError> {
            let pool = crate::get_connection_pool().await?;
            Ok(Self::new(pool))
        }

        async fn get_conn(&self) -> Result<Object<AsyncPgConnection>, DatabaseError> {
            self.pool
                .get()
                .await
                .map_err(
                    |e| {
                        DatabaseError::ConnectionError(
                            diesel::ConnectionError::BadConnection(e.to_string()),
                        )
                    },
                )
        }
    }

    // Actor trait implementations

    #[async_trait]
    impl InsertActors for PostgresActorRepository {
        async fn insert_batch(
            &self,
            actors: &[NewActor],
        ) -> Result<
            Vec<(
                i32,
                String,
            )>,
            DatabaseError,
        > {
            let mut conn = self
                .get_conn()
                .await?;

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
                )>(&mut conn)
                .await
                .map_err(DatabaseError::from)
        }
    }

    #[async_trait]
    impl GetActorsForMovie for PostgresActorRepository {
        async fn get_for_movie(&self, movie: &Movie) -> Result<Vec<Actor>, DatabaseError> {
            let mut conn = self
                .get_conn()
                .await?;

            Ok(
                actors::actors_for_movie(
                    movie, &mut conn,
                )
                .await,
            )
        }
    }

    #[async_trait]
    impl InsertMovieActorAssociations for PostgresActorRepository {
        async fn insert_movie_associations(
            &self,
            movie_actors: &[NewMovieActor],
        ) -> Result<usize, DatabaseError> {
            let mut conn = self
                .get_conn()
                .await?;

            diesel::insert_into(schema::movie_actor::table)
                .values(movie_actors)
                .on_conflict((
                    schema::movie_actor::movie_id,
                    schema::movie_actor::actor_id,
                ))
                .do_nothing()
                .execute(&mut conn)
                .await
                .map_err(DatabaseError::from)
        }
    }

    #[derive(Clone)]
    pub struct PostgresDirectorRepository {
        pool: Pool<AsyncPgConnection>,
    }

    impl PostgresDirectorRepository {
        pub fn new(pool: Pool<AsyncPgConnection>) -> Self {
            Self { pool }
        }

        pub async fn from_env() -> Result<Self, DatabaseError> {
            let pool = crate::get_connection_pool().await?;
            Ok(Self::new(pool))
        }

        async fn get_conn(&self) -> Result<Object<AsyncPgConnection>, DatabaseError> {
            self.pool
                .get()
                .await
                .map_err(
                    |e| {
                        DatabaseError::ConnectionError(
                            diesel::ConnectionError::BadConnection(e.to_string()),
                        )
                    },
                )
        }
    }

    // Director trait implementations

    #[async_trait]
    impl InsertDirectors for PostgresDirectorRepository {
        async fn insert_batch(
            &self,
            directors: &[NewDirector],
        ) -> Result<
            Vec<(
                i32,
                String,
            )>,
            DatabaseError,
        > {
            use schema::director::dsl::*;

            let mut conn = self
                .get_conn()
                .await?;

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
                    )>(&mut conn)
                    .await?;

                result.push(inserted);
            }

            Ok(result)
        }
    }

    #[derive(Clone)]
    pub struct PostgresGenreRepository {
        pool: Pool<AsyncPgConnection>,
    }

    impl PostgresGenreRepository {
        pub fn new(pool: Pool<AsyncPgConnection>) -> Self {
            Self { pool }
        }

        pub async fn from_env() -> Result<Self, DatabaseError> {
            let pool = crate::get_connection_pool().await?;
            Ok(Self::new(pool))
        }

        async fn get_conn(&self) -> Result<Object<AsyncPgConnection>, DatabaseError> {
            self.pool
                .get()
                .await
                .map_err(
                    |e| {
                        DatabaseError::ConnectionError(
                            diesel::ConnectionError::BadConnection(e.to_string()),
                        )
                    },
                )
        }
    }

    // Genre trait implementations

    #[async_trait]
    impl GetAllGenres for PostgresGenreRepository {
        async fn get_all(&self) -> Result<Vec<String>, DatabaseError> {
            use schema::movie_genre::dsl::*;

            let mut conn = self
                .get_conn()
                .await?;

            movie_genre
                .select(genre)
                .distinct()
                .load::<String>(&mut conn)
                .await
                .map_err(DatabaseError::from)
        }
    }

    #[async_trait]
    impl GetGenresForMovie for PostgresGenreRepository {
        async fn get_for_movie(&self, movie: &Movie) -> Result<Vec<String>, DatabaseError> {
            let mut conn = self
                .get_conn()
                .await?;

            Ok(
                genres::genres_for_movie(
                    movie, &mut conn,
                )
                .await,
            )
        }
    }

    #[async_trait]
    impl InsertMovieGenreAssociations for PostgresGenreRepository {
        async fn insert_movie_associations(
            &self,
            movie_genres: &[NewMovieGenre],
        ) -> Result<usize, DatabaseError> {
            let mut conn = self
                .get_conn()
                .await?;

            diesel::insert_into(schema::movie_genre::table)
                .values(movie_genres)
                .on_conflict((
                    schema::movie_genre::movie_id,
                    schema::movie_genre::genre,
                ))
                .do_nothing()
                .execute(&mut conn)
                .await
                .map_err(DatabaseError::from)
        }
    }

    /// Internal module for postgres-specific actor operations
    pub mod actors {
        use super::*;

        pub(crate) async fn load_actors_for_movies(
            movie_ids: &[i32],
            conn: &mut AsyncPgConnection,
        ) -> Result<
            Vec<(
                MovieActor,
                Actor,
            )>,
            DatabaseError,
        > {
            use schema::{actor, movie_actor};
            movie_actor::table
                .inner_join(actor::table)
                .select((
                    movie_actor::all_columns,
                    actor::all_columns,
                ))
                .filter(movie_actor::movie_id.eq_any(movie_ids))
                .order(movie_actor::actor_order.asc())
                .load::<(
                    MovieActor,
                    Actor,
                )>(conn)
                .await
                .map_err(DatabaseError::from)
        }

        pub async fn insert_actors(
            actors: &[NewActor],
            connection: &mut AsyncPgConnection,
        ) -> Result<
            Vec<(
                i32,
                String,
            )>,
            DatabaseError,
        > {
            use schema::actor;
            diesel::insert_into(actor::table)
                .values(actors)
                .on_conflict(actor::name)
                .do_update()
                .set(actor::id.eq(actor::id))
                .returning((
                    actor::id,
                    actor::name,
                ))
                .get_results::<(
                    i32,
                    String,
                )>(connection)
                .await
                .map_err(DatabaseError::from)
        }

        pub async fn insert_movie_actors(
            movie_actors: &[NewMovieActor],
            connection: &mut AsyncPgConnection,
        ) -> Result<Vec<MovieActor>, DatabaseError> {
            use schema::movie_actor;
            diesel::insert_into(movie_actor::table)
                .values(movie_actors)
                .on_conflict((
                    movie_actor::movie_id,
                    movie_actor::actor_id,
                ))
                .do_nothing()
                .get_results::<MovieActor>(connection)
                .await
                .map_err(DatabaseError::from)
        }

        pub async fn actors_for_movie(
            movie: &Movie,
            connection: &mut AsyncPgConnection,
        ) -> Vec<Actor> {
            use schema::{actor, movie_actor};
            MovieActor::belonging_to(&movie)
                .inner_join(actor::table)
                .select(actor::all_columns)
                .order(movie_actor::actor_order.asc())
                .load::<Actor>(connection)
                .await
                .unwrap_or_else(|_| Vec::new())
        }
    }

    /// Internal module for postgres-specific director operations
    pub mod directors {
        use super::*;

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
    }

    /// Internal module for postgres-specific genre operations
    pub mod genres {
        use super::*;
        use diesel::ConnectionError;

        pub(crate) async fn load_genres_for_movies(
            movie_ids: &[i32],
            conn: &mut AsyncPgConnection,
        ) -> Result<Vec<MovieGenre>, DatabaseError> {
            use schema::movie_genre;
            movie_genre::table
                .filter(movie_genre::movie_id.eq_any(movie_ids))
                .load::<MovieGenre>(conn)
                .await
                .map_err(DatabaseError::from)
        }

        /// Get all unique genres from the database
        pub async fn get_all_genres() -> Result<Vec<String>, DatabaseError> {
            use crate::schema::movie_genre::dsl::*;
            let pool = crate::get_connection_pool().await?;
            let mut conn = pool
                .get()
                .await
                .map_err(
                    |e| {
                        DatabaseError::ConnectionError(
                            ConnectionError::BadConnection(e.to_string()),
                        )
                    },
                )?;

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
            use schema::movie_genre;
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

        pub async fn genres_for_movie(
            movie: &Movie,
            connection: &mut AsyncPgConnection,
        ) -> Vec<String> {
            use schema::movie_genre;
            MovieGenre::belonging_to(&movie)
                .select(movie_genre::genre)
                .load::<String>(connection)
                .await
                .unwrap_or_else(|_| Vec::new())
        }
    }

    /// Internal module for postgres-specific movie operations
    pub mod movies {
        use super::*;
        use diesel::ConnectionError;

        pub async fn insert_movies(
            new_movies: &[NewMovie],
            connection: &mut AsyncPgConnection,
        ) -> Result<
            Vec<(
                i32,
                String,
            )>,
            DatabaseError,
        > {
            diesel::insert_into(schema::movie::table)
                .values(new_movies)
                .on_conflict((
                    schema::movie::name,
                    schema::movie::release_year,
                ))
                .do_update()
                .set(schema::movie::id.eq(schema::movie::id))
                .returning((
                    schema::movie::id,
                    schema::movie::name,
                ))
                .get_results(connection)
                .await
                .map_err(DatabaseError::from)
        }

        pub async fn update_movie_location(
            movie_id: i32,
            new_location: String,
        ) -> Result<(), DatabaseError> {
            let pool = crate::get_connection_pool().await?;
            let mut connection = pool
                .get()
                .await
                .map_err(
                    |e| {
                        DatabaseError::ConnectionError(
                            ConnectionError::BadConnection(e.to_string()),
                        )
                    },
                )?;

            diesel::update(schema::movie::table.find(movie_id))
                .set(schema::movie::location.eq(new_location))
                .execute(&mut connection)
                .await
                .map_err(DatabaseError::from)?;

            Ok(())
        }
    }

    /// Internal module for postgres-specific full movie operations
    pub mod full_movies {
        use super::*;
        use std::collections::HashSet;

        /// Adds a Vec of FullMovies in bulk to the database.
        pub async fn insert_full_movies(
            mut full_movies: Vec<FullMovie>,
            pool: &Pool<AsyncPgConnection>,
        ) -> Result<(), Box<dyn std::error::Error>> {
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
                        log::error!(
                            "Movie '{}' at index '{}' had a date parsing issue",
                            movie.name,
                            index
                        );
                    }

                    date_result?;
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
                        crate::DatabaseError::ConnectionError(
                            diesel::ConnectionError::BadConnection(e.to_string()),
                        )
                    },
                )?;
            let mut actors = crate::postgres::actors::insert_actors(
                &actors,
                &mut connection,
            )
            .await?;
            actors.sort_by(
                |a, b| {
                    a.1.cmp(&b.1)
                },
            );

            let mut directors = crate::postgres::directors::insert_directors(
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

            let mut movie_inserts = crate::postgres::movies::insert_movies(
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

            let _movie_actors = crate::postgres::actors::insert_movie_actors(
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

            let _movie_genres: Vec<models::MovieGenre> =
                crate::postgres::genres::insert_movie_genres(
                    &movie_genres,
                    &mut connection,
                )
                .await?;

            Ok(())
        }

        fn parse_date(date_str: &str, movie_name: &str) -> Result<(), Box<dyn std::error::Error>> {
            use chrono::NaiveDate;
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
    }

    /// Internal module for postgres-specific structured search
    pub mod structured_search {
        use super::*;
        use diesel::expression::BoxableExpression;
        use diesel::BoolExpressionMethods;

        fn to_ilike_patterns(names: &[String]) -> Vec<String> {
            names
                .iter()
                .map(
                    |n| {
                        format!(
                            "%{}%",
                            n.to_lowercase()
                        )
                    },
                )
                .collect()
        }

        /// Searches for movies using structured query criteria with dynamic Diesel query building
        pub async fn search_movies_structured(
            structured_query: &StructuredQuery,
            connection: &mut AsyncPgConnection,
        ) -> Result<Vec<FullMovie>, diesel::result::Error> {
            use models::Movie;
            use schema::{actor, director, movie, movie_actor, movie_genre};

            log::info!(
                "Executing structured search with criteria: {:?}",
                structured_query
            );

            // Step 1: Build the main query with director and title filters (AND logic)
            let mut base_query = movie::table
                .left_join(director::table.on(movie::director_id.eq(director::id.nullable())))
                .distinct()
                .into_boxed();

            // Apply director filters (AND logic - all must match)
            if !structured_query
                .directors
                .is_empty()
            {
                log::info!(
                    "Filtering by directors: {:?}",
                    structured_query.directors
                );
                for director_name in &structured_query.directors {
                    let pattern = format!(
                        "%{}%",
                        director_name.to_lowercase()
                    );
                    base_query = base_query.filter(director::name.ilike(pattern));
                }
            }

            // Apply title keyword filters (AND logic - all must match)
            if !structured_query
                .title_keywords
                .is_empty()
            {
                log::info!(
                    "Filtering by title keywords: {:?}",
                    structured_query.title_keywords
                );
                for keyword in &structured_query.title_keywords {
                    let pattern = format!(
                        "%{}%",
                        keyword.to_lowercase()
                    );
                    base_query = base_query.filter(movie::name.ilike(pattern));
                }
            }

            // Apply description keyword filters (OR logic - any must match)
            if !structured_query
                .description_keywords
                .is_empty()
            {
                log::info!(
                    "Filtering by description keywords (OR): {:?}",
                    structured_query.description_keywords
                );

                use diesel::sql_types::Nullable;

                let patterns = to_ilike_patterns(&structured_query.description_keywords);
                let or_condition: Option<
                    Box<
                        dyn BoxableExpression<
                            _,
                            diesel::pg::Pg,
                            SqlType = Nullable<diesel::sql_types::Bool>,
                        >,
                    >,
                > = patterns
                    .into_iter()
                    .fold(
                        None,
                        |acc, pattern| {
                            let expr = movie::description.ilike(pattern);
                            Some(
                                match acc {
                                    None => Box::new(expr),
                                    Some(prev) => Box::new(prev.or(expr)),
                                },
                            )
                        },
                    );

                if let Some(condition) = or_condition {
                    base_query = base_query.filter(condition);
                }
            }

            // Apply actor filters (OR logic - match ANY actor) using Diesel's exists()
            if !structured_query
                .actors
                .is_empty()
            {
                log::info!(
                    "Filtering by actors (OR): {:?}",
                    structured_query.actors
                );

                use diesel::dsl::exists;

                let patterns = to_ilike_patterns(&structured_query.actors);
                let actor_or_condition: Option<
                    Box<
                        dyn BoxableExpression<_, diesel::pg::Pg, SqlType = diesel::sql_types::Bool>,
                    >,
                > = patterns
                    .into_iter()
                    .fold(
                        None,
                        |acc, pattern| {
                            let expr = actor::name.ilike(pattern);
                            Some(
                                match acc {
                                    None => Box::new(expr),
                                    Some(prev) => Box::new(prev.or(expr)),
                                },
                            )
                        },
                    );

                if let Some(condition) = actor_or_condition {
                    let actor_subquery = movie_actor::table
                        .inner_join(actor::table)
                        .filter(movie_actor::movie_id.eq(movie::id))
                        .filter(condition)
                        .select(movie_actor::movie_id);

                    base_query = base_query.filter(exists(actor_subquery));
                }
            }

            // Apply genre filters (OR logic - match ANY genre) using Diesel's exists()
            if !structured_query
                .genres
                .is_empty()
            {
                log::info!(
                    "Filtering by genres (OR): {:?}",
                    structured_query.genres
                );

                use diesel::dsl::exists;

                let patterns = to_ilike_patterns(&structured_query.genres);
                let genre_or_condition: Option<
                    Box<
                        dyn BoxableExpression<_, diesel::pg::Pg, SqlType = diesel::sql_types::Bool>,
                    >,
                > = patterns
                    .into_iter()
                    .fold(
                        None,
                        |acc, pattern| {
                            let expr = movie_genre::genre.ilike(pattern);
                            Some(
                                match acc {
                                    None => Box::new(expr),
                                    Some(prev) => Box::new(prev.or(expr)),
                                },
                            )
                        },
                    );

                if let Some(condition) = genre_or_condition {
                    let genre_subquery = movie_genre::table
                        .filter(movie_genre::movie_id.eq(movie::id))
                        .filter(condition)
                        .select(movie_genre::movie_id);

                    base_query = base_query.filter(exists(genre_subquery));
                }
            }

            // Step 5: Execute the final query
            let movies: Vec<(
                models::Movie,
                Option<Director>,
            )> = base_query
                .load::<(
                    models::Movie,
                    Option<Director>,
                )>(connection)
                .await?;

            log::info!(
                "Found {} movies matching all criteria",
                movies.len()
            );

            // Step 6: Hydrate with actors and genres - 2 bulk queries instead of 2N
            let movie_refs: Vec<&Movie> = movies
                .iter()
                .map(|(m, _)| m)
                .collect();
            let movie_ids: Vec<i32> = movie_refs
                .iter()
                .map(|m| m.id)
                .collect();

            let raw_actors = super::actors::load_actors_for_movies(
                &movie_ids, connection,
            )
            .await
            .map_err(
                |e| match e {
                    crate::DatabaseError::DieselError(de) => de,
                    _ => diesel::result::Error::NotFound,
                },
            )?;
            let raw_genres = super::genres::load_genres_for_movies(
                &movie_ids, connection,
            )
            .await
            .map_err(
                |e| match e {
                    crate::DatabaseError::DieselError(de) => de,
                    _ => diesel::result::Error::NotFound,
                },
            )?;

            let actors_per_movie: Vec<Vec<Actor>> = raw_actors
                .grouped_by(&movie_refs)
                .into_iter()
                .map(
                    |group| {
                        group
                            .into_iter()
                            .map(|(_, actor)| actor)
                            .collect()
                    },
                )
                .collect();

            let genres_per_movie: Vec<Vec<String>> = raw_genres
                .grouped_by(&movie_refs)
                .into_iter()
                .map(
                    |group| {
                        group
                            .into_iter()
                            .map(|mg| mg.genre)
                            .collect()
                    },
                )
                .collect();

            let full_movies: Vec<FullMovie> = movies
                .into_iter()
                .zip(actors_per_movie)
                .zip(genres_per_movie)
                .map(
                    |(((movie, director), actors), genres)| {
                        FullMovie::from((
                            movie, director, actors, genres,
                        ))
                    },
                )
                .collect();

            log::info!(
                "Returning {} full movies",
                full_movies.len()
            );
            Ok(full_movies)
        }
    }
} // End of inner module

// Re-export all postgres-specific types for backwards compatibility
#[cfg(feature = "postgres")]
pub use inner::*;
