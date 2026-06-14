use async_trait::async_trait;
use models::{
    Actor, FullMovie, Movie, NewActor, NewDirector, NewMovie, NewMovieActor, NewMovieGenre,
    StructuredQuery,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, RwLock};

use crate::traits::*;
use crate::DatabaseError;

pub struct MockMovieRepository {
    pub movies: Arc<RwLock<HashMap<i32, FullMovie>>>,
    pub next_id: AtomicI32,
}

impl Default for MockMovieRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl MockMovieRepository {
    pub fn new() -> Self {
        Self {
            movies: Arc::new(RwLock::new(HashMap::new())),
            next_id: AtomicI32::new(1),
        }
    }

    pub fn with_movies(movies: Vec<FullMovie>) -> Self {
        let repo = Self::new();
        for movie in movies {
            let id = movie.id;
            repo.movies
                .write()
                .unwrap()
                .insert(
                    id, movie,
                );
            let current = repo
                .next_id
                .load(Ordering::SeqCst);
            repo.next_id
                .store(
                    current.max(id + 1),
                    Ordering::SeqCst,
                );
        }
        repo
    }
}

impl Repository for MockMovieRepository {
    type Id = i32;
}

// Movie trait implementations

#[async_trait]
impl GetAllMovies for MockMovieRepository {
    async fn get_all(&self) -> Result<Vec<FullMovie>, DatabaseError> {
        Ok(
            self.movies
                .read()
                .unwrap()
                .values()
                .cloned()
                .collect(),
        )
    }
}

#[async_trait]
impl GetMovieById for MockMovieRepository {
    async fn get_by_id(&self, id: i32) -> Result<FullMovie, DatabaseError> {
        self.movies
            .read()
            .unwrap()
            .get(&id)
            .cloned()
            .ok_or_else(|| DatabaseError::DieselError(diesel::result::Error::NotFound))
    }
}

#[async_trait]
impl GetMoviesByIds for MockMovieRepository {
    async fn get_by_ids(&self, ids: Vec<i32>) -> Result<Vec<FullMovie>, DatabaseError> {
        let movies = self
            .movies
            .read()
            .unwrap();
        Ok(
            ids.into_iter()
                .filter_map(
                    |id| {
                        movies
                            .get(&id)
                            .cloned()
                    },
                )
                .collect(),
        )
    }
}

#[async_trait]
impl InsertMovie for MockMovieRepository {
    async fn insert(&self, mut full_movie: FullMovie) -> Result<FullMovie, DatabaseError> {
        let id = self
            .next_id
            .fetch_add(
                1,
                Ordering::SeqCst,
            );
        full_movie.id = id;
        self.movies
            .write()
            .unwrap()
            .insert(
                id,
                full_movie.clone(),
            );
        Ok(full_movie)
    }
}

#[async_trait]
impl InsertMovies for MockMovieRepository {
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
        let mut results = Vec::new();
        for new_movie in new_movies {
            let id = self
                .next_id
                .fetch_add(
                    1,
                    Ordering::SeqCst,
                );
            results.push((
                id,
                new_movie
                    .name
                    .clone(),
            ));
        }
        Ok(results)
    }
}

#[async_trait]
impl UpdateMovieLocation for MockMovieRepository {
    async fn update_location(
        &self,
        movie_id: i32,
        new_location: String,
    ) -> Result<(), DatabaseError> {
        if let Some(movie) = self
            .movies
            .write()
            .unwrap()
            .get_mut(&movie_id)
        {
            movie.location = Some(new_location);
            Ok(())
        } else {
            Err(DatabaseError::DieselError(diesel::result::Error::NotFound))
        }
    }
}

#[async_trait]
impl SearchMoviesByEmbedding for MockMovieRepository {
    async fn search_by_embedding(
        &self,
        _embedding: Vec<f32>,
        limit: i64,
    ) -> Result<Vec<FullMovie>, DatabaseError> {
        Ok(
            self.movies
                .read()
                .unwrap()
                .values()
                .take(limit as usize)
                .cloned()
                .collect(),
        )
    }
}

#[async_trait]
impl GetRecentMovies for MockMovieRepository {
    async fn get_recent(&self, limit: i64) -> Result<Vec<FullMovie>, DatabaseError> {
        let mut movies: Vec<_> = self
            .movies
            .read()
            .unwrap()
            .values()
            .cloned()
            .collect();
        movies.sort_by(
            |a, b| {
                b.added_on
                    .cmp(&a.added_on)
            },
        );
        Ok(
            movies
                .into_iter()
                .take(limit as usize)
                .collect(),
        )
    }
}

#[async_trait]
impl SearchMoviesStructured for MockMovieRepository {
    async fn search_structured(
        &self,
        query: &StructuredQuery,
    ) -> Result<Vec<FullMovie>, DatabaseError> {
        let mut results: Vec<FullMovie> = self
            .movies
            .read()
            .unwrap()
            .values()
            .cloned()
            .collect();

        if !query
            .title_keywords
            .is_empty()
        {
            results.retain(
                |movie| {
                    query
                        .title_keywords
                        .iter()
                        .all(
                            |kw| {
                                movie
                                    .name
                                    .to_lowercase()
                                    .contains(&kw.to_lowercase())
                            },
                        )
                },
            );
        }

        if !query
            .directors
            .is_empty()
        {
            results.retain(
                |movie| {
                    movie
                        .director
                        .as_ref()
                        .is_some_and(
                            |d| {
                                query
                                    .directors
                                    .iter()
                                    .any(
                                        |dir| {
                                            d.name
                                                .to_lowercase()
                                                .contains(&dir.to_lowercase())
                                        },
                                    )
                            },
                        )
                },
            );
        }

        if !query
            .actors
            .is_empty()
        {
            results.retain(
                |movie| {
                    query
                        .actors
                        .iter()
                        .any(
                            |actor_name| {
                                movie
                                    .actors
                                    .iter()
                                    .any(
                                        |a| {
                                            a.name
                                                .to_lowercase()
                                                .contains(&actor_name.to_lowercase())
                                        },
                                    )
                            },
                        )
                },
            );
        }

        if !query
            .genres
            .is_empty()
        {
            results.retain(
                |movie| {
                    query
                        .genres
                        .iter()
                        .any(
                            |genre_name| {
                                movie
                                    .genres
                                    .iter()
                                    .any(
                                        |g| {
                                            g.to_lowercase()
                                                .contains(&genre_name.to_lowercase())
                                        },
                                    )
                            },
                        )
                },
            );
        }

        Ok(results)
    }
}

pub struct MockActorRepository {
    pub actors: Arc<
        RwLock<
            HashMap<
                i32,
                (
                    i32,
                    String,
                ),
            >,
        >,
    >,
    pub next_id: AtomicI32,
}

impl Default for MockActorRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl MockActorRepository {
    pub fn new() -> Self {
        Self {
            actors: Arc::new(RwLock::new(HashMap::new())),
            next_id: AtomicI32::new(1),
        }
    }
}

impl Repository for MockActorRepository {
    type Id = i32;
}

// Actor trait implementations

#[async_trait]
impl InsertActors for MockActorRepository {
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
        let mut results = Vec::new();
        for actor in actors {
            let id = self
                .next_id
                .fetch_add(
                    1,
                    Ordering::SeqCst,
                );
            self.actors
                .write()
                .unwrap()
                .insert(
                    id,
                    (
                        id,
                        actor
                            .name
                            .clone(),
                    ),
                );
            results.push((
                id,
                actor
                    .name
                    .clone(),
            ));
        }
        Ok(results)
    }
}

#[async_trait]
impl GetActorsForMovie for MockActorRepository {
    async fn get_for_movie(&self, _movie: &Movie) -> Result<Vec<Actor>, DatabaseError> {
        Ok(Vec::new())
    }
}

#[async_trait]
impl InsertMovieActorAssociations for MockActorRepository {
    async fn insert_movie_associations(
        &self,
        _associations: &[NewMovieActor],
    ) -> Result<usize, DatabaseError> {
        Ok(0)
    }
}

pub struct MockDirectorRepository {
    pub directors: Arc<
        RwLock<
            HashMap<
                i32,
                (
                    i32,
                    String,
                ),
            >,
        >,
    >,
    pub next_id: AtomicI32,
}

impl Default for MockDirectorRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl MockDirectorRepository {
    pub fn new() -> Self {
        Self {
            directors: Arc::new(RwLock::new(HashMap::new())),
            next_id: AtomicI32::new(1),
        }
    }
}

impl Repository for MockDirectorRepository {
    type Id = i32;
}

// Director trait implementations

#[async_trait]
impl InsertDirectors for MockDirectorRepository {
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
        let mut results = Vec::new();
        for director in directors {
            let id = self
                .next_id
                .fetch_add(
                    1,
                    Ordering::SeqCst,
                );
            self.directors
                .write()
                .unwrap()
                .insert(
                    id,
                    (
                        id,
                        director
                            .name
                            .clone(),
                    ),
                );
            results.push((
                id,
                director
                    .name
                    .clone(),
            ));
        }
        Ok(results)
    }
}

#[derive(Default)]
pub struct MockGenreRepository {
    pub genres: Vec<String>,
}

impl MockGenreRepository {
    pub fn new() -> Self {
        Self { genres: Vec::new() }
    }

    pub fn with_genres(genres: Vec<String>) -> Self {
        Self { genres }
    }
}

// Genre trait implementations

#[async_trait]
impl GetAllGenres for MockGenreRepository {
    async fn get_all(&self) -> Result<Vec<String>, DatabaseError> {
        Ok(
            self.genres
                .clone(),
        )
    }
}

#[async_trait]
impl GetGenresForMovie for MockGenreRepository {
    async fn get_for_movie(&self, _movie: &Movie) -> Result<Vec<String>, DatabaseError> {
        Ok(Vec::new())
    }
}

#[async_trait]
impl InsertMovieGenreAssociations for MockGenreRepository {
    async fn insert_movie_associations(
        &self,
        _associations: &[NewMovieGenre],
    ) -> Result<usize, DatabaseError> {
        Ok(0)
    }
}

pub struct MockEmbeddingProvider {
    pub embeddings: HashMap<String, Vec<f32>>,
}

impl MockEmbeddingProvider {
    pub fn new() -> Self {
        Self {
            embeddings: HashMap::new(),
        }
    }

    pub fn with_embedding(mut self, text: String, embedding: Vec<f32>) -> Self {
        self.embeddings
            .insert(
                text, embedding,
            );
        self
    }
}

impl Default for MockEmbeddingProvider {
    fn default() -> Self {
        Self::new()
    }
}

// Embedding trait implementations

#[async_trait]
impl GenerateEmbedding for MockEmbeddingProvider {
    async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
        Ok(
            self.embeddings
                .get(text)
                .cloned()
                .unwrap_or_else(|| vec![0.0; 384]),
        )
    }
}

#[async_trait]
impl GenerateEmbeddings for MockEmbeddingProvider {
    async fn generate_embeddings(
        &self,
        texts: Vec<String>,
    ) -> Result<Vec<Vec<f32>>, Box<dyn std::error::Error>> {
        Ok(
            texts
                .iter()
                .map(
                    |text| {
                        self.embeddings
                            .get(text)
                            .cloned()
                            .unwrap_or_else(|| vec![0.0; 384])
                    },
                )
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_movie_repository() {
        let repo = MockMovieRepository::new();
        let movie = FullMovie {
            id: 0,
            key_hash: FullMovie::generate_key_hash("Test Movie"),
            name: "Test Movie".to_string(),
            director: None,
            description: None,
            actors: vec![],
            genres: vec![],
            embedding: None,
            added_on: None,
            location: None,
            release_year: 0,
        };

        let inserted = repo
            .insert(movie)
            .await
            .unwrap();
        assert_eq!(
            inserted.id,
            1
        );
        assert_eq!(
            inserted.name,
            "Test Movie"
        );

        let retrieved = repo
            .get_by_id(1)
            .await
            .unwrap();
        assert_eq!(
            retrieved.name,
            "Test Movie"
        );

        let all = repo
            .get_all()
            .await
            .unwrap();
        assert_eq!(
            all.len(),
            1
        );
    }

    #[tokio::test]
    async fn test_mock_embedding_provider() {
        let provider = MockEmbeddingProvider::new().with_embedding(
            "test".to_string(),
            vec![1.0, 2.0, 3.0],
        );

        let embedding = provider
            .generate_embedding("test")
            .await
            .unwrap();
        assert_eq!(
            embedding,
            vec![1.0, 2.0, 3.0]
        );

        let default_embedding = provider
            .generate_embedding("unknown")
            .await
            .unwrap();
        assert_eq!(
            default_embedding.len(),
            384
        );
    }
}
