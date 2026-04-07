use async_trait::async_trait;
use models::{
    Actor, FullMovie, Movie, NewActor, NewDirector, NewMovie, NewMovieActor, NewMovieGenre,
    StructuredQuery,
};

use crate::DatabaseError;

// Single-purpose traits for movies

#[async_trait]
pub trait GetMovieById: Send + Sync {
    async fn get_by_id(&mut self, id: i32) -> Result<FullMovie, DatabaseError>;
}

#[async_trait]
pub trait GetMoviesByIds: Send + Sync {
    async fn get_by_ids(&mut self, ids: Vec<i32>) -> Result<Vec<FullMovie>, DatabaseError>;
}

#[async_trait]
pub trait GetAllMovies: Send + Sync {
    async fn get_all(&mut self) -> Result<Vec<FullMovie>, DatabaseError>;
}

#[async_trait]
pub trait InsertMovie: Send + Sync {
    async fn insert(&mut self, full_movie: FullMovie) -> Result<FullMovie, DatabaseError>;
}

#[async_trait]
pub trait InsertMovies: Send + Sync {
    async fn insert_batch(
        &mut self,
        movies: &[NewMovie],
    ) -> Result<
        Vec<(
            i32,
            String,
        )>,
        DatabaseError,
    >;
}

#[async_trait]
pub trait UpdateMovieLocation: Send + Sync {
    async fn update_location(
        &mut self,
        movie_id: i32,
        location: String,
    ) -> Result<(), DatabaseError>;
}

#[async_trait]
pub trait SearchMoviesByEmbedding: Send + Sync {
    async fn search_by_embedding(
        &mut self,
        embedding: Vec<f32>,
        limit: i64,
    ) -> Result<Vec<FullMovie>, DatabaseError>;
}

#[async_trait]
pub trait GetRecentMovies: Send + Sync {
    async fn get_recent(&mut self, limit: i64) -> Result<Vec<FullMovie>, DatabaseError>;
}

#[async_trait]
pub trait RandomMovies: Send + Sync {
    async fn get_random(&mut self, count: i64) -> Result<Vec<FullMovie>, DatabaseError>;
}

#[async_trait]
pub trait GetUnknownLocationMovies: Send + Sync {
    async fn get_unknown_location(&mut self, limit: i64) -> Result<Vec<FullMovie>, DatabaseError>;
}

#[async_trait]
pub trait SearchMoviesStructured: Send + Sync {
    async fn search_structured(
        &mut self,
        query: &StructuredQuery,
    ) -> Result<Vec<FullMovie>, DatabaseError>;
}

// Single-purpose traits for actors

#[async_trait]
pub trait InsertActors: Send + Sync {
    async fn insert_batch(
        &mut self,
        actors: &[NewActor],
    ) -> Result<
        Vec<(
            i32,
            String,
        )>,
        DatabaseError,
    >;
}

#[async_trait]
pub trait GetActorsForMovie: Send + Sync {
    async fn get_for_movie(&mut self, movie: &Movie) -> Result<Vec<Actor>, DatabaseError>;
}

#[async_trait]
pub trait InsertMovieActorAssociations: Send + Sync {
    async fn insert_movie_associations(
        &mut self,
        associations: &[NewMovieActor],
    ) -> Result<usize, DatabaseError>;
}

// Single-purpose traits for directors

#[async_trait]
pub trait InsertDirectors: Send + Sync {
    async fn insert_batch(
        &mut self,
        directors: &[NewDirector],
    ) -> Result<
        Vec<(
            i32,
            String,
        )>,
        DatabaseError,
    >;
}

// Single-purpose traits for genres

#[async_trait]
pub trait GetAllGenres: Send + Sync {
    async fn get_all(&mut self) -> Result<Vec<String>, DatabaseError>;
}

#[async_trait]
pub trait GetGenresForMovie: Send + Sync {
    async fn get_for_movie(&mut self, movie: &Movie) -> Result<Vec<String>, DatabaseError>;
}

#[async_trait]
pub trait InsertMovieGenreAssociations: Send + Sync {
    async fn insert_movie_associations(
        &mut self,
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
