use async_trait::async_trait;
use models::{
    Actor, FullMovie, Movie, NewActor, NewDirector, NewMovie, NewMovieActor, NewMovieGenre,
    StructuredQuery,
};
use std::collections::HashMap;

use crate::traits::*;
use crate::DatabaseError;

#[derive(Default)]
pub struct MockMovieRepository {
    pub movies: HashMap<i32, FullMovie>,
    pub next_id: i32,
}

impl MockMovieRepository {
    pub fn new() -> Self {
        Self {
            movies: HashMap::new(),
            next_id: 1,
        }
    }

    pub fn with_movies(movies: Vec<FullMovie>) -> Self {
        let mut repo = Self::new();
        for movie in movies {
            let id = movie.id;
            repo.movies
                .insert(
                    id, movie,
                );
            repo.next_id = repo
                .next_id
                .max(id + 1);
        }
        repo
    }
}

// Movie trait implementations

#[async_trait]
impl GetAllMovies for MockMovieRepository {
    async fn get_all(&mut self) -> Result<Vec<FullMovie>, DatabaseError> {
        Ok(
            self.movies
                .values()
                .cloned()
                .collect(),
        )
    }
}

#[async_trait]
impl GetMovieById for MockMovieRepository {
    async fn get_by_id(&mut self, id: i32) -> Result<FullMovie, DatabaseError> {
        self.movies
            .get(&id)
            .cloned()
            .ok_or_else(|| DatabaseError::DieselError(diesel::result::Error::NotFound))
    }
}

#[async_trait]
impl GetMoviesByIds for MockMovieRepository {
    async fn get_by_ids(&mut self, ids: Vec<i32>) -> Result<Vec<FullMovie>, DatabaseError> {
        Ok(
            ids.into_iter()
                .filter_map(
                    |id| {
                        self.movies
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
    async fn insert(&mut self, mut full_movie: FullMovie) -> Result<FullMovie, DatabaseError> {
        let id = self.next_id;
        self.next_id += 1;
        full_movie.id = id;
        self.movies
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
        &mut self,
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
            let id = self.next_id;
            self.next_id += 1;
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
        &mut self,
        movie_id: i32,
        new_location: String,
    ) -> Result<(), DatabaseError> {
        if let Some(movie) = self
            .movies
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
        &mut self,
        _embedding: Vec<f32>,
        limit: i64,
    ) -> Result<Vec<FullMovie>, DatabaseError> {
        Ok(
            self.movies
                .values()
                .take(limit as usize)
                .cloned()
                .collect(),
        )
    }
}

#[async_trait]
impl GetRecentMovies for MockMovieRepository {
    async fn get_recent(&mut self, limit: i64) -> Result<Vec<FullMovie>, DatabaseError> {
        let mut movies: Vec<_> = self
            .movies
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
        &mut self,
        query: &StructuredQuery,
    ) -> Result<Vec<FullMovie>, DatabaseError> {
        let mut results: Vec<FullMovie> = self
            .movies
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

#[derive(Default)]
pub struct MockActorRepository {
    pub actors: HashMap<
        i32,
        (
            i32,
            String,
        ),
    >,
    pub next_id: i32,
}

impl MockActorRepository {
    pub fn new() -> Self {
        Self {
            actors: HashMap::new(),
            next_id: 1,
        }
    }
}

// Actor trait implementations

#[async_trait]
impl InsertActors for MockActorRepository {
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
        let mut results = Vec::new();
        for actor in actors {
            let id = self.next_id;
            self.next_id += 1;
            self.actors
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
    async fn get_for_movie(&mut self, _movie: &Movie) -> Result<Vec<Actor>, DatabaseError> {
        Ok(Vec::new())
    }
}

#[async_trait]
impl InsertMovieActorAssociations for MockActorRepository {
    async fn insert_movie_associations(
        &mut self,
        _associations: &[NewMovieActor],
    ) -> Result<usize, DatabaseError> {
        Ok(0)
    }
}

#[derive(Default)]
pub struct MockDirectorRepository {
    pub directors: HashMap<
        i32,
        (
            i32,
            String,
        ),
    >,
    pub next_id: i32,
}

impl MockDirectorRepository {
    pub fn new() -> Self {
        Self {
            directors: HashMap::new(),
            next_id: 1,
        }
    }
}

// Director trait implementations

#[async_trait]
impl InsertDirectors for MockDirectorRepository {
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
        let mut results = Vec::new();
        for director in directors {
            let id = self.next_id;
            self.next_id += 1;
            self.directors
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
    async fn get_all(&mut self) -> Result<Vec<String>, DatabaseError> {
        Ok(
            self.genres
                .clone(),
        )
    }
}

#[async_trait]
impl GetGenresForMovie for MockGenreRepository {
    async fn get_for_movie(&mut self, _movie: &Movie) -> Result<Vec<String>, DatabaseError> {
        Ok(Vec::new())
    }
}

#[async_trait]
impl InsertMovieGenreAssociations for MockGenreRepository {
    async fn insert_movie_associations(
        &mut self,
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
        let mut repo = MockMovieRepository::new();
        let movie = FullMovie {
            id: 0,
            name: "Test Movie".to_string(),
            director: None,
            description: None,
            actors: vec![],
            genres: vec![],
            embedding: None,
            added_on: None,
            location: None,
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
