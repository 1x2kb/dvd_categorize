//! Database Layer for DVD Categorizer
//!
//! This crate provides the database abstraction layer using Diesel ORM with async PostgreSQL support.
//! It handles all database operations including CRUD operations, vector similarity search using pgvector,
//! and structured query building.
//!
//! # Features
//!
//! - **Async PostgreSQL**: Full async support via diesel-async
//! - **Vector Search**: Semantic similarity search using pgvector extension
//! - **Structured Search**: Type-safe query building from structured criteria
//! - **Batch Operations**: Optimized bulk inserts and queries
//! - **Testing Support**: Mock implementations and test utilities
//!
//! # Architecture
//!
//! The crate is organized into several modules:
//! - `actors`, `directors`, `genres`, `movies`: Entity-specific operations
//! - `full_movies`: Operations on complete movie objects with all relationships
//! - `embedding`: Vector embedding storage and retrieval
//! - `structured_search`: Dynamic query building from structured criteria
//! - `postgres`: Repository pattern implementations
//! - `traits`: Trait definitions for database operations
//!
//! # Example
//!
//! ```no_run
//! use database::{
//!     traits::{GetAllMovies, SearchMoviesByEmbedding},
//!     PostgresMovieRepository,
//! };
//!
//! #[tokio::main]
//! async fn main() {
//!     let mut repo = PostgresMovieRepository::from_env().await.unwrap();
//!
//!     // Get all movies
//!     let movies = repo.get_all().await.unwrap();
//!
//!     // Vector similarity search
//!     let embedding = vec![0.1; 768]; // Example embedding
//!     let similar = repo.search_by_embedding(embedding, 10).await.unwrap();
//! }
//! ```

pub mod embedding;
#[cfg(any(test, feature = "testing"))]
pub mod mocks;
pub mod postgres;
pub mod traits;

// Postgres-dependent modules - re-exported from postgres internal modules when feature is enabled
#[cfg(feature = "postgres")]
pub mod actors {
    pub use crate::postgres::actors::*;
}
#[cfg(feature = "postgres")]
pub mod directors {
    pub use crate::postgres::directors::*;
}
#[cfg(feature = "postgres")]
pub mod genres {
    pub use crate::postgres::genres::*;
}
#[cfg(feature = "postgres")]
pub mod movies {
    pub use crate::postgres::movies::*;
}
#[cfg(feature = "postgres")]
pub mod full_movies {
    pub use crate::postgres::full_movies::*;
}
#[cfg(feature = "postgres")]
pub mod structured_search {
    pub use crate::postgres::structured_search::*;
}

use std::env;
use std::error::Error;
use std::fmt::Display;

use diesel::prelude::*;
use diesel::ConnectionError;
use diesel_async::pooled_connection::deadpool::Pool;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
// Schema module is only available when postgres feature is enabled
#[cfg(feature = "postgres")]
pub use models::schema::*;
pub use models::*;

pub use embedding::*;
#[cfg(any(test, feature = "testing"))]
pub use mocks::*;
// Postgres-specific re-exports (only available when postgres feature is enabled)
#[cfg(feature = "postgres")]
pub use postgres::*;
#[cfg(feature = "postgres")]
pub use postgres::full_movies::insert_full_movies;
#[cfg(feature = "postgres")]
pub use postgres::movies::update_movie_location;
pub use traits::*;

/// Type alias for the active movie repository backend.
/// 
/// This resolves to a concrete type at compile time based on which database feature is enabled.
/// - `postgres` feature: Uses `PostgresMovieRepository`
/// - Future: `mongodb` feature will use `MongoMovieRepository`
#[cfg(feature = "postgres")]
pub type MovieRepo = postgres::PostgresMovieRepository;

/// Type alias for database operation results
pub type DbResult<T> = Result<T, DatabaseError>;

#[cfg(feature = "testing")]
pub trait Random {
    fn random() -> Self;
}

#[derive(Debug)]
pub enum DatabaseError {
    ConnectionError(ConnectionError),
    DieselError(diesel::result::Error),
    QueryError(String),
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
        match self {
            DatabaseError::QueryError(msg) => write!(
                f,
                "Query error: {}",
                msg
            ),
            DatabaseError::ConnectionError(e) => write!(
                f,
                "Database connection error: {}",
                e
            ),
            DatabaseError::DieselError(e) => write!(
                f,
                "Database query error: {}",
                e
            ),
        }
    }
}

pub fn parse_added_on_date(date_str: &str, movie_name: &str) -> Option<chrono::NaiveDateTime> {
    match chrono::NaiveDateTime::parse_from_str(
        date_str,
        "%Y-%m-%d %H:%M:%S%.f",
    ) {
        Ok(dt) => Some(dt),
        Err(_) => {
            match chrono::NaiveDate::parse_from_str(
                date_str, "%Y-%m-%d",
            ) {
                Ok(date) => match date.and_hms_opt(
                    0, 0, 0,
                ) {
                    Some(dt) => Some(dt),
                    None => {
                        log::error!(
                            "Invalid time components for date '{}' in movie '{}'",
                            date_str,
                            movie_name
                        );
                        None
                    }
                },
                Err(e) => {
                    log::error!(
                        "Failed to parse date '{}' for movie '{}': {}",
                        date_str,
                        movie_name,
                        e
                    );
                    None
                }
            }
        }
    }
}

#[cfg(feature = "postgres")]
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

#[cfg(feature = "postgres")]
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

#[cfg(feature = "postgres")]
pub(crate) async fn load_actors_and_genres(
    movies: &[&Movie],
    conn_actors: &mut AsyncPgConnection,
    conn_genres: &mut AsyncPgConnection,
) -> Result<
    (
        Vec<Vec<Actor>>,
        Vec<Vec<String>>,
    ),
    DatabaseError,
> {
    let movie_ids: Vec<i32> = movies
        .iter()
        .map(|m| m.id)
        .collect();

    let (raw_actors, raw_genres) = tokio::try_join!(
        load_actors_for_movies(
            &movie_ids,
            conn_actors
        ),
        load_genres_for_movies(
            &movie_ids,
            conn_genres
        ),
    )?;

    let actors_per_movie = raw_actors
        .grouped_by(movies)
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

    let genres_per_movie = raw_genres
        .grouped_by(movies)
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

    Ok((
        actors_per_movie,
        genres_per_movie,
    ))
}

/// Creates a connection pool for the PostgreSQL database.
///
/// # Errors
/// Returns `DatabaseError` if the DATABASE_URL environment variable is not set
/// or if the pool cannot be created.
#[cfg(feature = "postgres")]
pub async fn get_connection_pool() -> Result<Pool<AsyncPgConnection>, DatabaseError> {
    let database_url = env::var("DATABASE_URL").map_err(
        |_| {
            DatabaseError::ConnectionError(
                diesel::ConnectionError::BadConnection(
                    "DATABASE_URL environment variable must be set".to_string(),
                ),
            )
        },
    )?;

    let manager =
        diesel_async::pooled_connection::AsyncDieselConnectionManager::<AsyncPgConnection>::new(
            database_url,
        );
    let pool = Pool::builder(manager)
        .build()
        .map_err(
            |e| {
                DatabaseError::ConnectionError(
                    diesel::ConnectionError::BadConnection(e.to_string()),
                )
            },
        )?;

    Ok(pool)
}
