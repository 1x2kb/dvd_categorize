//! MongoDB Repository Implementation for DVD Categorizer
//!
//! This module provides a MongoDB-backed implementation of the MovieRepository trait.
//! It stores movies as documents with embedded actors, directors, and genres.
//! Vector search is handled by Qdrant (separate vector database).

use std::env;
use std::sync::Arc;

use async_trait::async_trait;
use mongodb::{Client, Collection, Database};
use mongodb::bson::{doc, oid::ObjectId, Bson};
use qdrant_client::Qdrant;
use qdrant_client::qdrant::{PointStruct, UpsertPointsBuilder};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::traits::*;
use crate::{DatabaseError, DbResult};
use models::{Actor, Director, FullMovie, NewMovie, ChatMessage, ChatSession, NewChatMessage};

/// MongoDB document representation of a movie
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovieDocument {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub rating: Option<f64>,
    pub release_year: i32,
    pub location: Option<String>,
    pub embedding: Option<Vec<f32>>,
    pub actors: Vec<Actor>,
    pub director: Option<Director>,
    pub genres: Vec<String>,
    pub added_on: Option<String>,
    pub key_hash: i64,
}

impl From<FullMovie> for MovieDocument {
    fn from(movie: FullMovie) -> Self {
        Self {
            id: if movie.id == 0 { None } else { Some(movie.id.to_string()) },
            name: movie.name,
            description: movie.description,
            rating: None, // Not in FullMovie currently
            release_year: movie.release_year,
            location: movie.location,
            embedding: movie.embedding,
            actors: movie.actors,
            director: movie.director,
            genres: movie.genres,
            added_on: movie.added_on,
            key_hash: movie.key_hash as i64,
        }
    }
}

impl From<MovieDocument> for FullMovie {
    fn from(doc: MovieDocument) -> Self {
        FullMovie {
            id: doc.id.as_ref().and_then(|s| s.parse().ok()).unwrap_or(0),
            name: doc.name,
            description: doc.description,
            actors: doc.actors,
            director: doc.director,
            genres: doc.genres,
            embedding: doc.embedding,
            added_on: doc.added_on,
            location: doc.location,
            release_year: doc.release_year,
            key_hash: doc.key_hash as u64,
        }
    }
}

/// MongoDB repository for movies with Qdrant vector search
#[derive(Clone)]
pub struct MongoMovieRepository {
    db: Database,
    movies: Collection<MovieDocument>,
    qdrant: Arc<Qdrant>,
    qdrant_collection: String,
}

impl MongoMovieRepository {
    /// Create a new MongoMovieRepository from environment variables
    /// 
    /// Required env vars:
    /// - MONGODB_URI: MongoDB connection string (e.g., mongodb://localhost:27017/dvd_catalog)
    /// - QDRANT_URL: Qdrant server URL (e.g., http://localhost:6334)
    /// - QDRANT_COLLECTION: Name of the Qdrant collection (default: "movies")
    pub async fn from_env() -> DbResult<Self> {
        let mongodb_uri = env::var("MONGODB_URI")
            .map_err(|_| DatabaseError::QueryError("MONGODB_URI environment variable must be set".to_string()))?;
        
        let qdrant_url = env::var("QDRANT_URL")
            .unwrap_or_else(|_| "http://localhost:6334".to_string());
        
        let qdrant_collection = env::var("QDRANT_COLLECTION")
            .unwrap_or_else(|_| "movies".to_string());

        // Parse MongoDB URI to extract database name
        let db_name = Self::extract_db_name(&mongodb_uri)
            .unwrap_or("dvd_catalog".to_string());

        let client = Client::with_uri_str(&mongodb_uri).await
            .map_err(|e| DatabaseError::QueryError(format!("MongoDB connection error: {}", e)))?;
        
        let db = client.database(&db_name);
        let movies = db.collection::<MovieDocument>("movies");

        // Initialize Qdrant client
        let qdrant = Qdrant::from_url(&qdrant_url)
            .build()
            .map_err(|e| DatabaseError::QueryError(format!("Qdrant connection error: {}", e)))?;

        let repo = Self {
            db,
            movies,
            qdrant: Arc::new(qdrant),
            qdrant_collection,
        };

        // Ensure Qdrant collection exists
        repo.ensure_qdrant_collection().await?;

        Ok(repo)
    }

    /// Extract database name from MongoDB URI
    fn extract_db_name(uri: &str) -> Option<String> {
        // Simple extraction - handles mongodb://host/dbname and mongodb+srv://host/dbname
        uri.split('/').last().map(|s| s.split('?').next().unwrap_or(s).to_string())
    }

    /// Ensure Qdrant collection exists
    async fn ensure_qdrant_collection(&self) -> DbResult<()> {
        // Check if collection exists, create if not
        let collections = self.qdrant.list_collections().await
            .map_err(|e| DatabaseError::QueryError(format!("Qdrant list collections error: {}", e)))?;
        
        let exists = collections.collections.iter()
            .any(|c| c.name == self.qdrant_collection);

        if !exists {
            // Default to 768 dimensions (common embedding size)
            // Can be made configurable
            use qdrant_client::qdrant::CreateCollectionBuilder;
            let create_request = CreateCollectionBuilder::new(&self.qdrant_collection)
                .vectors_config(qdrant_client::qdrant::VectorsConfig {
                    config: Some(qdrant_client::qdrant::vectors_config::Config::Params(
                        qdrant_client::qdrant::VectorParams {
                            size: 768,
                            distance: qdrant_client::qdrant::Distance::Cosine.into(),
                            ..Default::default()
                        }
                    )),
                });
            
            self.qdrant.create_collection(create_request).await
                .map_err(|e| DatabaseError::QueryError(format!("Qdrant create collection error: {}", e)))?;
        }

        Ok(())
    }

    /// Get a reference to the movies collection
    pub fn movies_collection(&self) -> &Collection<MovieDocument> {
        &self.movies
    }

    /// Insert a movie and its embedding into Qdrant
    async fn insert_movie_with_embedding(&self, movie: MovieDocument) -> DbResult<String> {
        let id = movie.id.clone().unwrap_or_else(|| ObjectId::new().to_string());
        
        // Insert into MongoDB
        let doc = MovieDocument { id: Some(id.clone()), ..movie };
        self.movies.insert_one(&doc).await
            .map_err(|e| DatabaseError::QueryError(format!("MongoDB insert error: {}", e)))?;

        // Insert embedding into Qdrant if available
        if let Some(embedding) = &doc.embedding {
            self.insert_qdrant_point(&id, embedding.clone()).await?;
        }

        Ok(id)
    }

    /// Insert a point into Qdrant
    async fn insert_qdrant_point(&self, id: &str, vector: Vec<f32>) -> DbResult<()> {
        use qdrant_client::qdrant::Value;
        
        let mut payload = std::collections::HashMap::new();
        payload.insert("movie_id".to_string(), Value::from(id.to_string()));
        
        let point = PointStruct::new(
            id.to_string(),
            vector,
            payload,
        );

        let operation = UpsertPointsBuilder::new(&self.qdrant_collection, vec![point]);

        self.qdrant.upsert_points(operation).await
            .map_err(|e| DatabaseError::QueryError(format!("Qdrant insert error: {}", e)))?;

        Ok(())
    }

    /// Search Qdrant for similar vectors and return movie IDs
    async fn search_qdrant(&self, embedding: Vec<f32>, limit: i64) -> DbResult<Vec<String>> {
        use qdrant_client::qdrant::SearchPointsBuilder;
        
        let search_request = SearchPointsBuilder::new(&self.qdrant_collection, embedding, limit as u64)
            .with_payload(true)
            .build();

        let response = self.qdrant.search_points(search_request).await
            .map_err(|e| DatabaseError::QueryError(format!("Qdrant search error: {}", e)))?;

        let ids: Vec<String> = response.result.iter()
            .filter_map(|point| {
                point.payload.get("movie_id")
                    .and_then(|v| v.as_str().map(|s| s.to_string()))
            })
            .collect();

        Ok(ids)
    }
}

// Implement Repository trait - MongoDB uses String IDs (ObjectId)
impl Repository for MongoMovieRepository {
    type Id = String;
}

#[async_trait]
impl GetAllMovies for MongoMovieRepository {
    async fn get_all(&self) -> DbResult<Vec<FullMovie>> {
        let mut cursor = self.movies.find(doc! {}).await
            .map_err(|e| DatabaseError::QueryError(format!("MongoDB find error: {}", e)))?;

        let mut movies = Vec::new();
        while cursor.advance().await
            .map_err(|e| DatabaseError::QueryError(format!("MongoDB cursor error: {}", e)))? 
        {
            let doc = cursor.deserialize_current()
                .map_err(|e| DatabaseError::QueryError(format!("MongoDB deserialize error: {}", e)))?;
            movies.push(doc.into());
        }

        Ok(movies)
    }
}

#[async_trait]
impl GetMovieById for MongoMovieRepository {
    async fn get_by_id(&self, id: Self::Id) -> DbResult<FullMovie> {
        let filter = doc! { "_id": &id };
        let result = self.movies.find_one(filter).await
            .map_err(|e| DatabaseError::QueryError(format!("MongoDB find_one error: {}", e)))?;
        
        result.map(|doc| doc.into()).ok_or_else(|| DatabaseError::QueryError(format!("Movie not found: {}", id)))
    }
}

#[async_trait]
impl GetMoviesByIds for MongoMovieRepository {
    async fn get_by_ids(&self, ids: Vec<Self::Id>) -> DbResult<Vec<FullMovie>> {
        let filter = doc! { "_id": { "$in": ids } };
        let mut cursor = self.movies.find(filter).await
            .map_err(|e| DatabaseError::QueryError(format!("MongoDB find error: {}", e)))?;

        let mut movies = Vec::new();
        while cursor.advance().await
            .map_err(|e| DatabaseError::QueryError(format!("MongoDB cursor error: {}", e)))? 
        {
            let doc = cursor.deserialize_current()
                .map_err(|e| DatabaseError::QueryError(format!("MongoDB deserialize error: {}", e)))?;
            movies.push(doc.into());
        }

        Ok(movies)
    }
}

#[async_trait]
impl InsertMovie for MongoMovieRepository {
    async fn insert(&self, movie: FullMovie) -> DbResult<FullMovie> {
        let doc: MovieDocument = movie.into();
        let id = self.insert_movie_with_embedding(doc).await?;
        self.get_by_id(id).await
    }
}

#[async_trait]
impl InsertMovies for MongoMovieRepository {
    async fn insert_batch(&self, movies: &[NewMovie]) -> DbResult<Vec<(Self::Id, String)>> {
        let mut results = Vec::new();
        for movie in movies {
            // Convert NewMovie to MovieDocument
            let doc = MovieDocument {
                id: None,
                name: movie.name.clone(),
                description: movie.description.clone(),
                rating: None,
                release_year: movie.release_year,
                location: movie.location.clone(),
                embedding: None, // Will be generated later if needed
                actors: Vec::new(), // Actors inserted separately
                director: None, // Director inserted separately
                genres: Vec::new(), // Genres inserted separately
                added_on: Some(chrono::Utc::now().to_rfc3339()),
                key_hash: 0,
            };
            let id = self.insert_movie_with_embedding(doc).await?;
            results.push((id, movie.name.clone()));
        }
        Ok(results)
    }
}

#[async_trait]
impl UpdateMovieLocation for MongoMovieRepository {
    async fn update_location(&self, movie_id: Self::Id, location: String) -> DbResult<()> {
        let filter = doc! { "_id": &movie_id };
        let update = doc! { "$set": { "location": location } };
        
        self.movies.update_one(filter, update).await
            .map_err(|e| DatabaseError::QueryError(format!("MongoDB update error: {}", e)))?;
        
        Ok(())
    }
}

#[async_trait]
impl GetRecentMovies for MongoMovieRepository {
    async fn get_recent(&self, limit: i64) -> DbResult<Vec<FullMovie>> {
        let mut cursor = self.movies.find(doc! {})
            .sort(doc! { "added_on": -1 })
            .limit(limit)
            .await
            .map_err(|e| DatabaseError::QueryError(format!("MongoDB find error: {}", e)))?;

        let mut movies = Vec::new();
        while cursor.advance().await
            .map_err(|e| DatabaseError::QueryError(format!("MongoDB cursor error: {}", e)))? 
        {
            let doc = cursor.deserialize_current()
                .map_err(|e| DatabaseError::QueryError(format!("MongoDB deserialize error: {}", e)))?;
            movies.push(doc.into());
        }

        Ok(movies)
    }
}

#[async_trait]
impl RandomMovies for MongoMovieRepository {
    async fn get_random(&self, count: i64) -> DbResult<Vec<FullMovie>> {
        // MongoDB $sample aggregation for random documents
        let pipeline = vec![
            doc! { "$sample": { "size": count } }
        ];

        let mut cursor = self.movies.aggregate(pipeline).await
            .map_err(|e| DatabaseError::QueryError(format!("MongoDB aggregate error: {}", e)))?;

        let mut movies = Vec::new();
        while cursor.advance().await
            .map_err(|e| DatabaseError::QueryError(format!("MongoDB cursor error: {}", e)))? 
        {
            let doc = cursor.deserialize_current()
                .map_err(|e| DatabaseError::QueryError(format!("MongoDB deserialize error: {}", e)))?;
            // Convert BSON document to MovieDocument
            let movie_doc: MovieDocument = mongodb::bson::from_bson(Bson::Document(doc))
                .map_err(|e| DatabaseError::QueryError(format!("BSON conversion error: {}", e)))?;
            movies.push(movie_doc.into());
        }

        Ok(movies)
    }
}

#[async_trait]
impl GetMoviesByReleaseYear for MongoMovieRepository {
    async fn get_by_release_year(
        &self,
        min_year: i32,
        max_year: i32,
        limit: i64,
    ) -> DbResult<Vec<FullMovie>> {
        let filter = doc! { 
            "release_year": { 
                "$gte": min_year, 
                "$lte": max_year 
            } 
        };
        
        let mut cursor = self.movies.find(filter)
            .limit(limit)
            .await
            .map_err(|e| DatabaseError::QueryError(format!("MongoDB find error: {}", e)))?;

        let mut movies = Vec::new();
        while cursor.advance().await
            .map_err(|e| DatabaseError::QueryError(format!("MongoDB cursor error: {}", e)))? 
        {
            let doc = cursor.deserialize_current()
                .map_err(|e| DatabaseError::QueryError(format!("MongoDB deserialize error: {}", e)))?;
            movies.push(doc.into());
        }

        Ok(movies)
    }
}

#[async_trait]
impl GetUnknownLocationMovies for MongoMovieRepository {
    async fn get_unknown_location(&self, limit: i64) -> DbResult<Vec<FullMovie>> {
        let filter = doc! { 
            "$or": [
                { "location": { "$exists": false } },
                { "location": Bson::Null },
                { "location": "" }
            ]
        };
        
        let mut cursor = self.movies.find(filter)
            .limit(limit)
            .await
            .map_err(|e| DatabaseError::QueryError(format!("MongoDB find error: {}", e)))?;

        let mut movies = Vec::new();
        while cursor.advance().await
            .map_err(|e| DatabaseError::QueryError(format!("MongoDB cursor error: {}", e)))? 
        {
            let doc = cursor.deserialize_current()
                .map_err(|e| DatabaseError::QueryError(format!("MongoDB deserialize error: {}", e)))?;
            movies.push(doc.into());
        }

        Ok(movies)
    }
}

#[async_trait]
impl GetUniqueLocations for MongoMovieRepository {
    async fn unique_locations(&self) -> DbResult<Vec<String>> {
        let pipeline = vec![
            doc! { "$match": { "location": { "$ne": Bson::Null, "$ne": "" } } },
            doc! { "$group": { "_id": "$location" } },
            doc! { "$sort": { "_id": 1 } }
        ];

        let mut cursor = self.movies.aggregate(pipeline).await
            .map_err(|e| DatabaseError::QueryError(format!("MongoDB aggregate error: {}", e)))?;

        let mut locations = Vec::new();
        while cursor.advance().await
            .map_err(|e| DatabaseError::QueryError(format!("MongoDB cursor error: {}", e)))? 
        {
            let doc = cursor.deserialize_current()
                .map_err(|e| DatabaseError::QueryError(format!("MongoDB deserialize error: {}", e)))?;
            if let Some(loc) = doc.get_str("_id").ok() {
                locations.push(loc.to_string());
            }
        }

        Ok(locations)
    }
}

#[async_trait]
impl MoviesByLocation for MongoMovieRepository {
    async fn movies_by_location(&self, location_name: &str) -> DbResult<Vec<FullMovie>> {
        let filter = doc! { "location": location_name };
        
        let mut cursor = self.movies.find(filter).await
            .map_err(|e| DatabaseError::QueryError(format!("MongoDB find error: {}", e)))?;

        let mut movies = Vec::new();
        while cursor.advance().await
            .map_err(|e| DatabaseError::QueryError(format!("MongoDB cursor error: {}", e)))? 
        {
            let doc = cursor.deserialize_current()
                .map_err(|e| DatabaseError::QueryError(format!("MongoDB deserialize error: {}", e)))?;
            movies.push(doc.into());
        }

        Ok(movies)
    }
}

#[async_trait]
impl SearchMoviesByEmbedding for MongoMovieRepository {
    async fn search_by_embedding(
        &self,
        embedding: Vec<f32>,
        limit: i64,
    ) -> DbResult<Vec<FullMovie>> {
        // Search Qdrant for similar vectors
        let ids = self.search_qdrant(embedding, limit).await?;
        
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        // Fetch full documents from MongoDB
        self.get_by_ids(ids).await
    }
}

// Chat sessions implementation using MongoDB
#[async_trait]
impl ChatSessions for MongoMovieRepository {
    async fn create_chat_session(&self) -> DbResult<Uuid> {
        let session = ChatSession {
            #[cfg(feature = "postgres")]
            id: 0,
            session_id: Uuid::new_v4(),
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
        };

        let collection = self.db.collection::<ChatSession>("chat_sessions");
        collection.insert_one(&session).await
            .map_err(|e| DatabaseError::QueryError(format!("MongoDB insert error: {}", e)))?;

        Ok(session.session_id)
    }

    async fn get_chat_history(&self, session_id: Uuid) -> DbResult<Vec<ChatMessage>> {
        let collection = self.db.collection::<ChatMessage>("chat_messages");
        let filter = doc! { "session_id": session_id.to_string() };
        
        let mut cursor = collection.find(filter)
            .sort(doc! { "created_at": 1 })
            .await
            .map_err(|e| DatabaseError::QueryError(format!("MongoDB find error: {}", e)))?;

        let mut messages = Vec::new();
        while cursor.advance().await
            .map_err(|e| DatabaseError::QueryError(format!("MongoDB cursor error: {}", e)))? 
        {
            let msg = cursor.deserialize_current()
                .map_err(|e| DatabaseError::QueryError(format!("MongoDB deserialize error: {}", e)))?;
            messages.push(msg);
        }

        Ok(messages)
    }

    async fn save_chat_message(&self, msg: NewChatMessage) -> DbResult<()> {
        let collection = self.db.collection::<NewChatMessage>("chat_messages");
        collection.insert_one(&msg).await
            .map_err(|e| DatabaseError::QueryError(format!("MongoDB insert error: {}", e)))?;

        // Update session timestamp
        let sessions = self.db.collection::<ChatSession>("chat_sessions");
        let filter = doc! { "session_id": msg.session_id.to_string() };
        let update = doc! { "$set": { "updated_at": Bson::String(chrono::Utc::now().to_rfc3339()) } };
        
        sessions.update_one(filter, update).await
            .map_err(|e| DatabaseError::QueryError(format!("MongoDB update error: {}", e)))?;

        Ok(())
    }

    async fn list_chat_sessions(&self) -> DbResult<Vec<ChatSession>> {
        let collection = self.db.collection::<ChatSession>("chat_sessions");
        
        let mut cursor = collection.find(doc! {})
            .sort(doc! { "updated_at": -1 })
            .await
            .map_err(|e| DatabaseError::QueryError(format!("MongoDB find error: {}", e)))?;

        let mut sessions = Vec::new();
        while cursor.advance().await
            .map_err(|e| DatabaseError::QueryError(format!("MongoDB cursor error: {}", e)))? 
        {
            let session = cursor.deserialize_current()
                .map_err(|e| DatabaseError::QueryError(format!("MongoDB deserialize error: {}", e)))?;
            sessions.push(session);
        }

        Ok(sessions)
    }
}

/// Additional methods for text search (matching Postgres API)
impl MongoMovieRepository {
    /// Search movies by text query using MongoDB regex matching
    pub async fn search_movies_by_text(
        &self,
        query: &str,
        limit: i64,
    ) -> DbResult<Vec<(FullMovie, f32)>> {
        use mongodb::bson::Regex;
        
        // Create regex pattern for case-insensitive search
        let pattern = Regex {
            pattern: query.to_string(),
            options: "i".to_string(), // case-insensitive
        };
        
        // Search across multiple fields
        let filter = doc! {
            "$or": [
                { "name": { "$regex": pattern.clone() } },
                { "description": { "$regex": pattern.clone() } },
                { "actors.name": { "$regex": pattern.clone() } },
                { "director.name": { "$regex": pattern.clone() } },
                { "genres": { "$regex": pattern.clone() } }
            ]
        };
        
        let mut cursor = self.movies.find(filter)
            .limit(limit)
            .await
            .map_err(|e| DatabaseError::QueryError(format!("MongoDB text search error: {}", e)))?;
        
        let mut results = Vec::new();
        while cursor.advance().await
            .map_err(|e| DatabaseError::QueryError(format!("MongoDB cursor error: {}", e)))? 
        {
            let doc = cursor.deserialize_current()
                .map_err(|e| DatabaseError::QueryError(format!("MongoDB deserialize error: {}", e)))?;
            
            // Simple scoring - all matches get score 1.0 for now
            results.push((doc.into(), 1.0f32));
        }
        
        Ok(results)
    }
}
