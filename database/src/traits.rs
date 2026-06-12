use async_trait::async_trait;
use models::{Actor, FullMovie, Movie, NewActor, NewDirector, NewMovie};

#[cfg(feature = "postgres")]
use models::{NewMovieActor, NewMovieGenre, StructuredQuery};

use crate::DatabaseError;

/// Base trait for all repositories defining the ID type used by the backend.
/// Postgres uses i32, MongoDB uses String (ObjectId), etc.
pub trait Repository: Send + Sync {
    type Id: Send + Sync;
}

// Single-purpose traits for movies

#[async_trait]
pub trait GetMovieById: Repository {
    async fn get_by_id(&self, id: Self::Id) -> Result<FullMovie, DatabaseError>;
}

#[async_trait]
pub trait GetMoviesByIds: Repository {
    async fn get_by_ids(&self, ids: Vec<Self::Id>) -> Result<Vec<FullMovie>, DatabaseError>;
}

#[async_trait]
pub trait GetAllMovies: Send + Sync {
    async fn get_all(&self) -> Result<Vec<FullMovie>, DatabaseError>;
}

#[async_trait]
pub trait InsertMovie: Send + Sync {
    async fn insert(&self, full_movie: FullMovie) -> Result<FullMovie, DatabaseError>;
}

#[async_trait]
pub trait InsertMovies: Repository {
    async fn insert_batch(
        &self,
        movies: &[NewMovie],
    ) -> Result<
        Vec<(
            Self::Id,
            String,
        )>,
        DatabaseError,
    >;
}

#[async_trait]
pub trait UpdateMovieLocation: Repository {
    async fn update_location(&self, movie_id: Self::Id, location: String) -> Result<(), DatabaseError>;
}

#[async_trait]
pub trait SearchMoviesByEmbedding: Send + Sync {
    async fn search_by_embedding(
        &self,
        embedding: Vec<f32>,
        limit: i64,
    ) -> Result<Vec<FullMovie>, DatabaseError>;
}

#[async_trait]
pub trait GetRecentMovies: Send + Sync {
    async fn get_recent(&self, limit: i64) -> Result<Vec<FullMovie>, DatabaseError>;
}

#[async_trait]
pub trait RandomMovies: Send + Sync {
    async fn get_random(&self, count: i64) -> Result<Vec<FullMovie>, DatabaseError>;
}

#[async_trait]
pub trait GetMoviesByReleaseYear: Send + Sync {
    async fn get_by_release_year(
        &self,
        min_year: i32,
        max_year: i32,
        limit: i64,
    ) -> Result<Vec<FullMovie>, DatabaseError>;
}

#[async_trait]
pub trait GetUnknownLocationMovies: Send + Sync {
    async fn get_unknown_location(&self, limit: i64) -> Result<Vec<FullMovie>, DatabaseError>;
}

#[async_trait]
pub trait GetUniqueLocations: Send + Sync {
    async fn unique_locations(&self) -> Result<Vec<String>, DatabaseError>;
}

#[async_trait]
pub trait MoviesByLocation: Send + Sync {
    async fn movies_by_location(
        &self,
        location_name: &str,
    ) -> Result<Vec<FullMovie>, DatabaseError>;
}

#[cfg(feature = "postgres")]
#[async_trait]
pub trait SearchMoviesStructured: Send + Sync {
    async fn search_structured(
        &self,
        query: &StructuredQuery,
    ) -> Result<Vec<FullMovie>, DatabaseError>;
}

// Single-purpose traits for actors

#[async_trait]
pub trait InsertActors: Repository {
    async fn insert_batch(
        &self,
        actors: &[NewActor],
    ) -> Result<
        Vec<(
            Self::Id,
            String,
        )>,
        DatabaseError,
    >;
}

#[async_trait]
pub trait GetActorsForMovie: Send + Sync {
    async fn get_for_movie(&self, movie: &Movie) -> Result<Vec<Actor>, DatabaseError>;
}

#[cfg(feature = "postgres")]
#[async_trait]
pub trait InsertMovieActorAssociations: Send + Sync {
    async fn insert_movie_associations(
        &self,
        associations: &[NewMovieActor],
    ) -> Result<usize, DatabaseError>;
}

// Single-purpose traits for directors

#[async_trait]
pub trait InsertDirectors: Repository {
    async fn insert_batch(
        &self,
        directors: &[NewDirector],
    ) -> Result<
        Vec<(
            Self::Id,
            String,
        )>,
        DatabaseError,
    >;
}

// Single-purpose traits for genres

#[async_trait]
pub trait GetAllGenres: Send + Sync {
    async fn get_all(&self) -> Result<Vec<String>, DatabaseError>;
}

#[async_trait]
pub trait GetGenresForMovie: Send + Sync {
    async fn get_for_movie(&self, movie: &Movie) -> Result<Vec<String>, DatabaseError>;
}

#[cfg(feature = "postgres")]
#[async_trait]
pub trait InsertMovieGenreAssociations: Send + Sync {
    async fn insert_movie_associations(
        &self,
        associations: &[NewMovieGenre],
    ) -> Result<usize, DatabaseError>;
}

// Single-purpose traits for embeddings

#[async_trait]
pub trait GenerateEmbedding: Send + Sync {
    async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>, Box<dyn std::error::Error>>;
}

#[async_trait]
pub trait GenerateEmbeddings: Send + Sync {
    async fn generate_embeddings(
        &self,
        texts: Vec<String>,
    ) -> Result<Vec<Vec<f32>>, Box<dyn std::error::Error>>;
}
