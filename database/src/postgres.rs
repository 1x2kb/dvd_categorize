use async_trait::async_trait;
use chrono::NaiveDate;
use diesel::prelude::*;
use diesel_async::pooled_connection::deadpool::Pool;
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
}

impl std::fmt::Debug for PostgresMovieRepository {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PostgresMovieRepository").finish_non_exhaustive()
    }
}

impl PostgresMovieRepository {

    /// Builds a repository using the `DATABASE_URL` environment variable.
    pub async fn from_env() -> Result<Self, DatabaseError> {
        let pool = crate::get_connection_pool().await?;
        Ok(Self::new(pool))
    }

    // Chat history methods

    pub async fn create_chat_session(&self) -> Result<uuid::Uuid, DatabaseError> {
        use models::{schema::chat_sessions, NewChatSession};

        let session_id = uuid::Uuid::new_v4();
        let new_session = NewChatSession { session_id };

        let mut conn = self
            .pool
            .get()
            .await
            .map_err(
                |e| {
                    DatabaseError::ConnectionError(
                        diesel::ConnectionError::BadConnection(e.to_string()),
                    )
                },
            )?;

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
            .pool
            .get()
            .await
            .map_err(
                |e| {
                    DatabaseError::ConnectionError(
                        diesel::ConnectionError::BadConnection(e.to_string()),
                    )
                },
            )?;

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
            .pool
            .get()
            .await
            .map_err(
                |e| {
                    DatabaseError::ConnectionError(
                        diesel::ConnectionError::BadConnection(e.to_string()),
                    )
                },
            )?;

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
            .pool
            .get()
            .await
            .map_err(
                |e| {
                    DatabaseError::ConnectionError(
                        diesel::ConnectionError::BadConnection(e.to_string()),
                    )
                },
            )?;

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
            .pool
            .get()
            .await
            .map_err(
                |e| {
                    DatabaseError::ConnectionError(
                        diesel::ConnectionError::BadConnection(e.to_string()),
                    )
                },
            )?;

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
            .pool
            .get()
            .await
            .map_err(
                |e| {
                    DatabaseError::ConnectionError(
                        diesel::ConnectionError::BadConnection(e.to_string()),
                    )
                },
            )?;

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

        let mut full_movies = Vec::new();
        for (movie, director) in movies {
            let actors = MovieActor::belonging_to(&movie)
                .inner_join(schema::actor::table)
                .select(schema::actor::all_columns)
                .load::<Actor>(&mut conn)
                .await
                .unwrap_or_default();

            let genres = MovieGenre::belonging_to(&movie)
                .select(schema::movie_genre::genre)
                .load::<String>(&mut conn)
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
    async fn get_by_id(&self, id: i32) -> Result<FullMovie, DatabaseError> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(
                |e| {
                    DatabaseError::ConnectionError(
                        diesel::ConnectionError::BadConnection(e.to_string()),
                    )
                },
            )?;

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

        let actors = crate::actors_for_movie(
            &movie, &mut conn,
        )
        .await;
        let genres = crate::genres_for_movie(
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
            .pool
            .get()
            .await
            .map_err(
                |e| {
                    DatabaseError::ConnectionError(
                        diesel::ConnectionError::BadConnection(e.to_string()),
                    )
                },
            )?;

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
            )>(&mut conn)
            .await?;

        // Use belonging_to to get all genre associations
        let movie_genres = MovieGenre::belonging_to(&movies)
            .load::<MovieGenre>(&mut conn)
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
    async fn insert(&self, full_movie: FullMovie) -> Result<FullMovie, DatabaseError> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(
                |e| {
                    DatabaseError::ConnectionError(
                        diesel::ConnectionError::BadConnection(e.to_string()),
                    )
                },
            )?;

        let _actors_unused: String = full_movie
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

        let _genres_unused: String = full_movie
            .genres
            .iter()
            .map(|genre| genre.as_str())
            .collect::<Vec<_>>()
            .join(",");

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

        // TODO: Fix circular dependency - move embedding generation to caller
        // Get embedding for insert.
        let embedding: Option<Vec<f32>> = None; // Stub - was: ai_chat::get_embedding(&embedding).await

        let added_on = full_movie.added_on.and_then(|date_str| {
            // Try parsing as full timestamp first, then fall back to date-only
            match chrono::NaiveDateTime::parse_from_str(&date_str, "%Y-%m-%d %H:%M:%S%.f") {
                Ok(dt) => Some(dt),
                Err(_) => {
                    match NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
                        Ok(date) => {
                            match date.and_hms_opt(0, 0, 0) {
                                Some(dt) => Some(dt),
                                None => {
                                    log::error!("Invalid time components for date '{}' in movie '{}'", date_str, full_movie.name);
                                    None
                                }
                            }
                        }
                        Err(e) => {
                            log::error!("Failed to parse date '{}' for movie '{}': {}", date_str, full_movie.name, e);
                            None
                        }
                    }
                }
            }
        });

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
            .pool
            .get()
            .await
            .map_err(
                |e| {
                    DatabaseError::ConnectionError(
                        diesel::ConnectionError::BadConnection(e.to_string()),
                    )
                },
            )?;

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
            .pool
            .get()
            .await
            .map_err(
                |e| {
                    DatabaseError::ConnectionError(
                        diesel::ConnectionError::BadConnection(e.to_string()),
                    )
                },
            )?;

        diesel::update(schema::movie::table.find(movie_id))
            .set(schema::movie::location.eq(new_location))
            .execute(&mut conn)
            .await
            .map_err(DatabaseError::from)?;

        Ok(())
    }
}

#[async_trait]
impl SearchMoviesByEmbedding for PostgresMovieRepository {
    async fn search_by_embedding(
        &self,
        embedding: Vec<f32>,
        limit: i64,
    ) -> Result<Vec<FullMovie>, DatabaseError> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(
                |e| {
                    DatabaseError::ConnectionError(
                        diesel::ConnectionError::BadConnection(e.to_string()),
                    )
                },
            )?;

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
impl SearchMoviesByText for PostgresMovieRepository {
    async fn search_by_text(
        &self,
        query: &str,
        limit: i64,
    ) -> Result<Vec<(FullMovie, f32)>, DatabaseError> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(
                |e| {
                    DatabaseError::ConnectionError(
                        diesel::ConnectionError::BadConnection(e.to_string()),
                    )
                },
            )?;

        let pattern = format!("%{}%", query.to_lowercase());

        // Diesel raw query to search across all columns with scoring
        let results = sql_query(
            r#"
            SELECT 
                m.id,
                m.name,
                m.description,
                m.release_year,
                m.box_office_revenue,
                m.location,
                m.dvd_summary,
                m.added_on,
                m.imdb_id,
                m.studio,
                m.runtime,
                d.id as director_id,
                d.name as director_name,
                d.birth_date as director_birth_date,
                d.birth_location as director_birth_location,
                ARRAY_AGG(DISTINCT a.name) FILTER (WHERE a.name IS NOT NULL) as actors,
                ARRAY_AGG(DISTINCT mg.genre) FILTER (WHERE mg.genre IS NOT NULL) as genres,
                (
                    CASE WHEN LOWER(m.name) LIKE $1 THEN 100 ELSE 0 END +
                    CASE WHEN EXISTS (
                        SELECT 1 FROM movie_actor ma2
                        JOIN actor a2 ON ma2.actor_id = a2.id
                        WHERE ma2.movie_id = m.id AND LOWER(a2.name) LIKE $1
                    ) THEN 20 ELSE 0 END +
                    CASE WHEN d.name IS NOT NULL AND LOWER(d.name) LIKE $1 THEN 25 ELSE 0 END +
                    CASE WHEN EXISTS (
                        SELECT 1 FROM movie_genre mg2
                        WHERE mg2.movie_id = m.id AND LOWER(mg2.genre) LIKE $1
                    ) THEN 15 ELSE 0 END
                )::float4 as score
            FROM movie m
            LEFT JOIN director d ON m.director_id = d.id
            LEFT JOIN movie_actor ma ON m.id = ma.movie_id
            LEFT JOIN actor a ON ma.actor_id = a.id
            LEFT JOIN movie_genre mg ON m.id = mg.movie_id
            WHERE 
                LOWER(m.name) LIKE $1
                OR EXISTS (
                    SELECT 1 FROM movie_actor ma2
                    JOIN actor a2 ON ma2.actor_id = a2.id
                    WHERE ma2.movie_id = m.id AND LOWER(a2.name) LIKE $1
                )
                OR (d.name IS NOT NULL AND LOWER(d.name) LIKE $1)
                OR EXISTS (
                    SELECT 1 FROM movie_genre mg2
                    WHERE mg2.movie_id = m.id AND LOWER(mg2.genre) LIKE $1
                )
            GROUP BY m.id, m.name, m.description, m.release_year, m.box_office_revenue,
                     m.location, m.dvd_summary, m.added_on, m.imdb_id, m.studio, m.runtime,
                     d.id, d.name, d.birth_date, d.birth_location
            HAVING (
                CASE WHEN LOWER(m.name) LIKE $1 THEN 100 ELSE 0 END +
                CASE WHEN EXISTS (
                    SELECT 1 FROM movie_actor ma2
                    JOIN actor a2 ON ma2.actor_id = a2.id
                    WHERE ma2.movie_id = m.id AND LOWER(a2.name) LIKE $1
                ) THEN 20 ELSE 0 END +
                CASE WHEN d.name IS NOT NULL AND LOWER(d.name) LIKE $1 THEN 25 ELSE 0 END +
                CASE WHEN EXISTS (
                    SELECT 1 FROM movie_genre mg2
                    WHERE mg2.movie_id = m.id AND LOWER(mg2.genre) LIKE $1
                ) THEN 15 ELSE 0 END
            ) > 0
            ORDER BY score DESC
            LIMIT $2
            "#
        )
        .bind::<diesel::sql_types::Text, _>(&pattern)
        .bind::<diesel::sql_types::BigInt, _>(limit)
        .load::<SearchResultRow>(&mut conn)
        .await
        .map_err(DatabaseError::from)?;

        Ok(results.into_iter().map(|r| (r.to_full_movie(), r.score)).collect())
    }
}

// Row type for search_by_text query
#[derive(QueryableByName)]
#[diesel(table_name = schema::movie)]
struct SearchResultRow {
    #[diesel(sql_type = diesel::sql_types::Integer)]
    id: i32,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    name: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    description: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Integer>)]
    release_year: Option<i32>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    box_office_revenue: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    location: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    dvd_summary: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Timestamp)]
    added_on: chrono::NaiveDateTime,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    imdb_id: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    studio: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Integer>)]
    runtime: Option<i32>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Integer>)]
    director_id: Option<i32>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    director_name: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Date>)]
    director_birth_date: Option<chrono::NaiveDate>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    director_birth_location: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Array<diesel::sql_types::Nullable<diesel::sql_types::Text>>)]
    actors: Vec<Option<String>>,
    #[diesel(sql_type = diesel::sql_types::Array<diesel::sql_types::Nullable<diesel::sql_types::Text>>)]
    genres: Vec<Option<String>>,
    #[diesel(sql_type = diesel::sql_types::Float)]
    score: f32,
}

impl SearchResultRow {
    fn to_full_movie(self) -> FullMovie {
        use models::Director;

        let director = self.director_id.map(|id| Director {
            id,
            name: self.director_name.unwrap_or_default(),
            birth_date: self.director_birth_date,
            birth_location: self.director_birth_location,
        });

        let actors: Vec<models::Actor> = self.actors.into_iter()
            .flatten()
            .enumerate()
            .map(|(idx, name)| models::Actor {
                id: idx as i32, // placeholder, not used for display
                name,
            })
            .collect();

        let genres: Vec<String> = self.genres.into_iter().flatten().collect();

        FullMovie::from((
            models::Movie {
                id: self.id,
                name: self.name.unwrap_or_default(),
                description: self.description,
                release_year: self.release_year,
                box_office_revenue: self.box_office_revenue,
                location: self.location,
                dvd_summary: self.dvd_summary,
                added_on: self.added_on,
                imdb_id: self.imdb_id,
                studio: self.studio,
                runtime: self.runtime,
                director_id: self.director_id,
            },
            director,
            actors,
            genres,
        ))
    }
}

#[async_trait]
impl GetRecentMovies for PostgresMovieRepository {
    async fn get_recent(&self, limit: i64) -> Result<Vec<FullMovie>, DatabaseError> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(
                |e| {
                    DatabaseError::ConnectionError(
                        diesel::ConnectionError::BadConnection(e.to_string()),
                    )
                },
            )?;

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
            .pool
            .get()
            .await
            .map_err(
                |e| {
                    DatabaseError::ConnectionError(
                        diesel::ConnectionError::BadConnection(e.to_string()),
                    )
                },
            )?;

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
            .pool
            .get()
            .await
            .map_err(
                |e| {
                    DatabaseError::ConnectionError(
                        diesel::ConnectionError::BadConnection(e.to_string()),
                    )
                },
            )?;

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
            .pool
            .get()
            .await
            .map_err(
                |e| {
                    DatabaseError::ConnectionError(
                        diesel::ConnectionError::BadConnection(e.to_string()),
                    )
                },
            )?;

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
            .pool
            .get()
            .await
            .map_err(
                |e| {
                    DatabaseError::ConnectionError(
                        diesel::ConnectionError::BadConnection(e.to_string()),
                    )
                },
            )?;

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
            .pool
            .get()
            .await
            .map_err(
                |e| {
                    DatabaseError::ConnectionError(
                        diesel::ConnectionError::BadConnection(e.to_string()),
                    )
                },
            )?;

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
    async fn movies_by_location(&self, location: &str) -> Result<Vec<FullMovie>, DatabaseError> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(
                |e| {
                    DatabaseError::ConnectionError(
                        diesel::ConnectionError::BadConnection(e.to_string()),
                    )
                },
            )?;

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
            .pool
            .get()
            .await
            .map_err(
                |e| {
                    DatabaseError::ConnectionError(
                        diesel::ConnectionError::BadConnection(e.to_string()),
                    )
                },
            )?;

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
            .pool
            .get()
            .await
            .map_err(
                |e| {
                    DatabaseError::ConnectionError(
                        diesel::ConnectionError::BadConnection(e.to_string()),
                    )
                },
            )?;

        Ok(
            crate::actors_for_movie(
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
            .pool
            .get()
            .await
            .map_err(
                |e| {
                    DatabaseError::ConnectionError(
                        diesel::ConnectionError::BadConnection(e.to_string()),
                    )
                },
            )?;

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
            .pool
            .get()
            .await
            .map_err(
                |e| {
                    DatabaseError::ConnectionError(
                        diesel::ConnectionError::BadConnection(e.to_string()),
                    )
                },
            )?;

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
}

// Genre trait implementations

#[async_trait]
impl GetAllGenres for PostgresGenreRepository {
    async fn get_all(&self) -> Result<Vec<String>, DatabaseError> {
        use schema::movie_genre::dsl::*;

        let mut conn = self
            .pool
            .get()
            .await
            .map_err(
                |e| {
                    DatabaseError::ConnectionError(
                        diesel::ConnectionError::BadConnection(e.to_string()),
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
}

#[async_trait]
impl GetGenresForMovie for PostgresGenreRepository {
    async fn get_for_movie(&self, movie: &Movie) -> Result<Vec<String>, DatabaseError> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(
                |e| {
                    DatabaseError::ConnectionError(
                        diesel::ConnectionError::BadConnection(e.to_string()),
                    )
                },
            )?;

        Ok(
            crate::genres_for_movie(
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
            .pool
            .get()
            .await
            .map_err(
                |e| {
                    DatabaseError::ConnectionError(
                        diesel::ConnectionError::BadConnection(e.to_string()),
                    )
                },
            )?;

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
