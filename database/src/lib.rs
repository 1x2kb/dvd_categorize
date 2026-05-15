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

pub mod actors;
pub mod directors;
pub mod embedding;
pub mod full_movies;
pub mod genres;
pub mod mocks;
pub mod movies;
pub mod postgres;
pub mod structured_search;
pub mod traits;

use std::env;
use std::error::Error;
use std::fmt::Display;

use diesel::ConnectionError;
use diesel_async::{AsyncConnection, AsyncPgConnection};
use diesel_async::pooled_connection::deadpool::Pool;
pub use models::{schema::*, *};

pub use actors::*;
pub use directors::*;
pub use embedding::*;
pub use full_movies::*;
pub use genres::*;
pub use mocks::*;
pub use movies::*;
pub use postgres::*;
pub use traits::*;

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
            DatabaseError::QueryError(msg) => write!(f, "Query error: {}", msg),
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

/// Creates a connection pool for the PostgreSQL database.
///
/// # Errors
/// Returns `DatabaseError` if the DATABASE_URL environment variable is not set
/// or if the pool cannot be created.
///
/// # Panics
/// Panics if the DATABASE_URL environment variable is not set.
pub async fn get_connection_pool() -> Result<Pool<AsyncPgConnection>, DatabaseError> {
    let database_url =
        env::var("DATABASE_URL").expect("DATABASE_URL environment variable must be set");
    
    let manager = diesel_async::pooled_connection::AsyncDieselConnectionManager::<AsyncPgConnection>::new(database_url);
    let pool = Pool::builder(manager)
        .build()
        .map_err(|e| DatabaseError::ConnectionError(
            diesel::ConnectionError::BadConnection(e.to_string())
        ))?;
    
    Ok(pool)
}

/// Establishes a connection to the PostgreSQL database.
///
/// # Errors
/// Returns `DatabaseError` if the DATABASE_URL environment variable is not set
/// or if the connection cannot be established.
///
/// # Panics
/// Panics if the DATABASE_URL environment variable is not set.
#[deprecated(note = "Use get_connection_pool() instead")]
pub async fn get_database_connection() -> Result<AsyncPgConnection, DatabaseError> {
    let database_url =
        env::var("DATABASE_URL").expect("DATABASE_URL environment variable must be set");
    AsyncPgConnection::establish(&database_url)
        .await
        .map_err(DatabaseError::from)
}

