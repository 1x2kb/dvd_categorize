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

#[cfg(feature = "postgres")]
pub mod actors;
#[cfg(feature = "postgres")]
pub mod directors;
#[cfg(feature = "postgres")]
pub mod embedding;
#[cfg(feature = "postgres")]
pub mod full_movies;
#[cfg(feature = "postgres")]
pub mod genres;
#[cfg(feature = "postgres")]
pub mod mocks;
#[cfg(feature = "postgres")]
pub mod movies;
#[cfg(feature = "postgres")]
pub mod postgres;
#[cfg(feature = "postgres")]
pub mod structured_search;
pub mod traits;
#[cfg(feature = "mongo-core")]
pub mod mongo_core;
#[cfg(feature = "mongo-redis")]
pub mod mongo_redis;

#[cfg(feature = "postgres")]
use std::env;
use std::error::Error;
use std::fmt::Display;

#[cfg(feature = "postgres")]
use diesel::ConnectionError;
#[cfg(feature = "postgres")]
use diesel_async::{AsyncConnection, AsyncPgConnection};
pub use models::*;
#[cfg(feature = "postgres")]
pub use models::schema::*;

#[cfg(feature = "postgres")]
pub use actors::*;
#[cfg(feature = "postgres")]
pub use directors::*;
#[cfg(feature = "postgres")]
pub use embedding::*;
#[cfg(feature = "postgres")]
pub use full_movies::*;
#[cfg(feature = "postgres")]
pub use genres::*;
#[cfg(feature = "postgres")]
pub use mocks::*;
#[cfg(feature = "postgres")]
pub use movies::*;
#[cfg(feature = "postgres")]
pub use postgres::*;
pub use traits::*;
#[cfg(feature = "mongo-core")]
pub use mongo_core::*;
#[cfg(feature = "mongo-redis")]
pub use mongo_redis::*;

/// Type alias for database operation results
pub type DbResult<T> = Result<T, DatabaseError>;

#[cfg(feature = "testing")]
pub trait Random {
    fn random() -> Self;
}

#[derive(Debug)]
pub enum DatabaseError {
    #[cfg(feature = "postgres")]
    ConnectionError(ConnectionError),
    #[cfg(feature = "postgres")]
    DieselError(diesel::result::Error),
    #[cfg(feature = "mongo-core")]
    MongoError(mongodb::error::Error),
    #[cfg(feature = "mongo-redis")]
    RedisError(redis::RedisError),
    NotFound,
}

#[cfg(feature = "postgres")]
impl From<ConnectionError> for DatabaseError {
    fn from(value: ConnectionError) -> Self {
        DatabaseError::ConnectionError(value)
    }
}

#[cfg(feature = "postgres")]
impl From<diesel::result::Error> for DatabaseError {
    fn from(value: diesel::result::Error) -> Self {
        DatabaseError::DieselError(value)
    }
}

#[cfg(feature = "mongo-core")]
impl From<mongodb::error::Error> for DatabaseError {
    fn from(value: mongodb::error::Error) -> Self {
        DatabaseError::MongoError(value)
    }
}

#[cfg(feature = "mongo-redis")]
impl From<redis::RedisError> for DatabaseError {
    fn from(value: redis::RedisError) -> Self {
        DatabaseError::RedisError(value)
    }
}

impl Error for DatabaseError {}

impl Display for DatabaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            #[cfg(feature = "postgres")]
            DatabaseError::ConnectionError(e) => write!(f, "Database connection error: {}", e),
            #[cfg(feature = "postgres")]
            DatabaseError::DieselError(e) => write!(f, "Database query error: {}", e),
            #[cfg(feature = "mongo-core")]
            DatabaseError::MongoError(e) => write!(f, "MongoDB error: {}", e),
            #[cfg(feature = "mongo-redis")]
            DatabaseError::RedisError(e) => write!(f, "Redis error: {}", e),
            DatabaseError::NotFound => write!(f, "Document not found"),
        }
    }
}

/// Establishes a connection to the PostgreSQL database.
///
/// # Errors
/// Returns `DatabaseError` if the DATABASE_URL environment variable is not set
/// or if the connection cannot be established.
///
/// # Panics
/// Panics if the DATABASE_URL environment variable is not set.
#[cfg(feature = "postgres")]
pub async fn get_database_connection() -> Result<AsyncPgConnection, DatabaseError> {
    let database_url =
        env::var("DATABASE_URL").expect("DATABASE_URL environment variable must be set");
    AsyncPgConnection::establish(&database_url)
        .await
        .map_err(DatabaseError::from)
}

