//! MongoDB Repository Implementation for DVD Categorizer
//!
//! This module provides a MongoDB-backed implementation of the MovieRepository trait.
//! It stores movies as documents with embedded actors, directors, and genres.
//! Vector search is handled by Qdrant (separate vector database).

#[cfg(feature = "mongodb")]
pub mod inner {
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

/// Serde helper that stores the document `_id` as a BSON `ObjectId` while
/// keeping it as an `Option<String>` (hex) in Rust. `None` is skipped on
/// serialize so MongoDB assigns the id; on read the stored `ObjectId` is
/// converted to its hex string representation.
mod opt_hex_as_object_id {
    use mongodb::bson::oid::ObjectId;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(value: &Option<String>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(hex) => {
                let oid = ObjectId::parse_str(hex).map_err(serde::ser::Error::custom)?;
                oid.serialize(serializer)
            }
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt = Option::<ObjectId>::deserialize(deserializer)?;
        Ok(opt.map(|oid| oid.to_hex()))
    }
}

/// MongoDB document representation of a movie
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovieDocument {
    #[serde(
        rename = "_id",
        default,
        skip_serializing_if = "Option::is_none",
        with = "opt_hex_as_object_id"
    )]
    pub id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub rating: Option<f64>,
    pub release_year: i32,
    pub location: Option<String>,
    pub actors: Vec<Actor>,
    pub director: Option<Director>,
    pub genres: Vec<String>,
    pub added_on: Option<String>,
}

impl From<FullMovie> for MovieDocument {
    fn from(movie: FullMovie) -> Self {
        Self {
            // MongoDB assigns the _id on insert; never set it ourselves.
            // (FullMovie.id is a hashed i32, not a real ObjectId.)
            id: None,
            name: movie.name,
            description: movie.description,
            rating: None, // Not in FullMovie currently
            release_year: movie.release_year,
            location: movie.location,
            actors: movie.actors,
            director: movie.director,
            genres: movie.genres,
            added_on: movie.added_on,
        }
    }
}

impl From<MovieDocument> for FullMovie {
    fn from(doc: MovieDocument) -> Self {
        // Under the mongodb backend `FullMovie::id` is the ObjectId hex string;
        // an unsaved document (no `_id`) maps to an empty id.
        let id = doc.id.unwrap_or_default();

        // Compute key_hash before moving doc.name
        let key_hash = FullMovie::generate_key_hash(&doc.name);

        FullMovie {
            id,
            name: doc.name,
            description: doc.description,
            actors: doc.actors,
            director: doc.director,
            genres: doc.genres,
            embedding: None,
            added_on: doc.added_on,
            location: doc.location,
            release_year: doc.release_year,
            key_hash,
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
    async fn insert_movie_with_embedding(&self, movie: MovieDocument, embedding: Option<Vec<f32>>) -> DbResult<String> {
        // Never set the _id ourselves; it stays None so it is skipped during
        // serialization and MongoDB assigns the ObjectId.
        let insert_result = self.movies.insert_one(&movie).await
            .map_err(|e| DatabaseError::QueryError(format!("MongoDB insert error: {}", e)))?;

        // Take the ObjectId MongoDB assigned from the insert response.
        let oid = insert_result.inserted_id.as_object_id()
            .ok_or_else(|| DatabaseError::QueryError(
                "MongoDB did not return an ObjectId for the inserted movie".to_string()))?;
        let id = oid.to_hex();

        // Insert embedding into Qdrant if available (moved, not cloned)
        if let Some(embedding) = embedding {
            let qdrant_id = self.insert_qdrant_point(&id, embedding).await
                .map_err(|e| DatabaseError::QueryError(format!("Qdrant insert error: {}", e)))?;

            // Update MongoDB document with Qdrant point ID for reference
            self.movies.update_one(
                doc! { "_id": oid },
                doc! { "$set": { "qdrant_id": qdrant_id.to_string() } },
            ).await
            .map_err(|e| DatabaseError::QueryError(format!("MongoDB update error: {}", e)))?;
        }

        Ok(id)
    }

    /// Insert a point into Qdrant, returns the Qdrant point UUID
    async fn insert_qdrant_point(&self, mongo_id: &str, vector: Vec<f32>) -> DbResult<Uuid> {
        use qdrant_client::qdrant::Value;

        // Generate a UUID for the Qdrant point ID
        let qdrant_id = Uuid::new_v4();

        let mut payload = std::collections::HashMap::new();
        payload.insert("movie_id".to_string(), Value::from(mongo_id.to_string()));

        let point = PointStruct::new(
            qdrant_id.to_string(),
            vector,
            payload,
        );

        let operation = UpsertPointsBuilder::new(&self.qdrant_collection, vec![point]);

        self.qdrant.upsert_points(operation).await
            .map_err(|e| DatabaseError::QueryError(format!("Qdrant insert error: {}", e)))?;

        Ok(qdrant_id)
    }

    /// Bulk insert points into Qdrant
    /// Takes the inserted_ids from MongoDB and embeddings to insert
    async fn insert_qdrant_points_bulk(
        &self,
        inserted_ids: &[String],
        new_docs: &[MovieDocument],
        embeddings_to_insert: Vec<(usize, Vec<f32>, String)>, // (doc_index, embedding, movie_name)
    ) -> DbResult<()> {
        use qdrant_client::qdrant::Value;
        use log::{debug, info, warn};

        // Defensive check: ensure inserted_ids and new_docs have same length
        if inserted_ids.len() != new_docs.len() {
            return Err(DatabaseError::QueryError(format!(
                "Qdrant bulk insert: length mismatch - inserted_ids({}) != new_docs({})",
                inserted_ids.len(),
                new_docs.len()
            )));
        }

        let mut points = Vec::new();

        for (doc_index, embedding, movie_name) in embeddings_to_insert {
            // Get the corresponding MongoDB ID
            if doc_index >= inserted_ids.len() {
                warn!("Invalid doc_index {} for movie {}, skipping Qdrant insert", doc_index, movie_name);
                continue;
            }
            let mongo_id = &inserted_ids[doc_index];

            // Generate a UUID for the Qdrant point ID
            let qdrant_id = Uuid::new_v4();

            let mut payload = std::collections::HashMap::new();
            payload.insert("movie_id".to_string(), Value::from(mongo_id.clone()));

            let point = PointStruct::new(
                qdrant_id.to_string(),
                embedding,
                payload,
            );
            points.push(point);
            debug!("Prepared Qdrant point for movie: {}", movie_name);
        }

        if !points.is_empty() {
            info!("Bulk inserting {} points to Qdrant", points.len());
            let operation = UpsertPointsBuilder::new(&self.qdrant_collection, points);

            self.qdrant.upsert_points(operation).await
                .map_err(|e| DatabaseError::QueryError(format!("Qdrant bulk insert error: {}", e)))?;

            info!("Successfully inserted embeddings to Qdrant");
        }

        Ok(())
    }

    /// Search Qdrant for similar vectors and return movie IDs
    async fn search_qdrant(&self, embedding: Vec<f32>, limit: i64) -> DbResult<Vec<String>> {
        use qdrant_client::qdrant::SearchPointsBuilder;
        use log::{debug, warn};

        debug!("Searching Qdrant collection '{}' with {}-dimensional vector", self.qdrant_collection, embedding.len());

        let search_request = SearchPointsBuilder::new(&self.qdrant_collection, embedding, limit as u64)
            .with_payload(true)
            .build();

        let response = self.qdrant.search_points(search_request).await
            .map_err(|e| DatabaseError::QueryError(format!("Qdrant search error: {}", e)))?;

        debug!("Qdrant returned {} raw results", response.result.len());

        let mut ids = Vec::new();
        for point in &response.result {
            debug!("Processing point id={:?}, payload keys={:?}", point.id, point.payload.keys().collect::<Vec<_>>());

            match point.payload.get("movie_id") {
                Some(value) => {
                    // The Value type is prost-generated, check the kind field
                    use qdrant_client::qdrant::value::Kind;
                    let movie_id = match &value.kind {
                        Some(Kind::StringValue(s)) => Some(s.clone()),
                        Some(Kind::IntegerValue(i)) => Some(i.to_string()),
                        Some(Kind::NullValue(_)) => {
                            warn!("movie_id value is null");
                            None
                        }
                        _ => {
                            warn!("movie_id has unexpected kind: {:?}", value.kind);
                            None
                        }
                    };

                    if let Some(id) = movie_id {
                        debug!("Extracted movie_id: {}", id);
                        ids.push(id);
                    }
                }
                None => {
                    warn!("Point {:?} has no movie_id in payload", point.id);
                }
            }
        }

        debug!("Qdrant search returning {} movie IDs", ids.len());
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
        let oid = ObjectId::parse_str(&id)
            .map_err(|e| DatabaseError::QueryError(format!("Invalid movie id '{}': {}", id, e)))?;
        let filter = doc! { "_id": oid };
        let result = self.movies.find_one(filter).await
            .map_err(|e| DatabaseError::QueryError(format!("MongoDB find_one error: {}", e)))?;
        
        result.map(|doc| doc.into()).ok_or_else(|| DatabaseError::QueryError(format!("Movie not found: {}", id)))
    }
}

#[async_trait]
impl GetMoviesByIds for MongoMovieRepository {
    async fn get_by_ids(&self, ids: Vec<Self::Id>) -> DbResult<Vec<FullMovie>> {
        let oids: Vec<ObjectId> = ids.iter()
            .map(|id| ObjectId::parse_str(id)
                .map_err(|e| DatabaseError::QueryError(format!("Invalid movie id '{}': {}", id, e))))
            .collect::<DbResult<Vec<_>>>()?;
        let filter = doc! { "_id": { "$in": oids } };
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
    async fn insert(&self, mut movie: FullMovie) -> DbResult<FullMovie> {
        let embedding = movie.embedding.take();
        let doc: MovieDocument = movie.into();
        let id = self.insert_movie_with_embedding(doc, embedding).await?;
        self.get_by_id(id).await
    }
}

#[async_trait]
impl InsertMovies for MongoMovieRepository {
    async fn insert_batch(&self, movies: &[NewMovie]) -> DbResult<Vec<(Self::Id, String)>> {
        // Convert to tuples with no embeddings for bulk insert
        let movies_with_embeddings: Vec<(NewMovie, Option<Vec<f32>>)> = movies
            .iter()
            .cloned()
            .map(|m| (m, None))
            .collect();
        self.insert_batch_with_embeddings(movies_with_embeddings).await
    }
}

#[async_trait]
impl InsertMoviesWithEmbeddings for MongoMovieRepository {
    async fn insert_batch_with_embeddings(
        &self,
        movies: Vec<(NewMovie, Option<Vec<f32>>)>,
    ) -> DbResult<Vec<(Self::Id, String)>> {
        use log::{debug, info, warn};

        if movies.is_empty() {
            return Ok(Vec::new());
        }

        info!("Starting bulk insert of {} movies with embeddings", movies.len());

        // Step 1: Check for existing movies by name (duplicate detection)
        let movie_names: Vec<String> = movies.iter().map(|(m, _)| m.name.clone()).collect();
        let existing_filter = doc! { "name": { "$in": &movie_names } };

        let mut existing_cursor = self.movies.find(existing_filter).await
            .map_err(|e| DatabaseError::QueryError(format!("MongoDB find error checking duplicates: {}", e)))?;

        let mut existing_names = std::collections::HashSet::new();
        while existing_cursor.advance().await
            .map_err(|e| DatabaseError::QueryError(format!("MongoDB cursor error: {}", e)))?
        {
            let doc: MovieDocument = existing_cursor.deserialize_current()
                .map_err(|e| DatabaseError::QueryError(format!("MongoDB deserialize error: {}", e)))?;
            existing_names.insert(doc.name);
        }

        if !existing_names.is_empty() {
            info!("Found {} existing movies, skipping duplicates", existing_names.len());
        }

        // Step 2: Filter out duplicates and prepare documents
        let mut new_docs = Vec::new();
        let mut embeddings_to_insert = Vec::new(); // (index, embedding) pairs

        for (idx, (movie, embedding)) in movies.into_iter().enumerate() {
            if existing_names.contains(&movie.name) {
                debug!("Skipping duplicate movie: {}", movie.name);
                continue;
            }

            let doc = MovieDocument {
                id: None,
                name: movie.name.clone(),
                description: movie.description,
                rating: None,
                release_year: movie.release_year,
                location: movie.location,
                actors: Vec::new(),
                director: None,
                genres: Vec::new(),
                added_on: Some(chrono::Utc::now().to_rfc3339()),
            };

            new_docs.push(doc);
            if let Some(emb) = embedding {
                embeddings_to_insert.push((new_docs.len() - 1, emb, movie.name));
            }
        }

        if new_docs.is_empty() {
            info!("All movies already exist, nothing to insert");
            return Ok(Vec::new());
        }

        info!("Inserting {} new movies to MongoDB", new_docs.len());

        // Step 3: Bulk insert to MongoDB (documents have id = None, so MongoDB
        // assigns each ObjectId)
        let insert_result = self.movies.insert_many(&new_docs).await
            .map_err(|e| DatabaseError::QueryError(format!("MongoDB bulk insert error: {}", e)))?;

        // Defensive check: ensure every document got an id back
        if insert_result.inserted_ids.len() != new_docs.len() {
            return Err(DatabaseError::QueryError(format!(
                "Length mismatch: inserted {} movies but have {} documents",
                insert_result.inserted_ids.len(),
                new_docs.len()
            )));
        }

        // Assign the MongoDB-generated ObjectId back to each document
        for (idx, bson_id) in &insert_result.inserted_ids {
            let hex = bson_id.as_object_id()
                .map(|oid| oid.to_hex())
                .ok_or_else(|| DatabaseError::QueryError(
                    "MongoDB did not return an ObjectId for an inserted movie".to_string()))?;
            if let Some(doc) = new_docs.get_mut(*idx) {
                doc.id = Some(hex);
            }
        }

        // Collect ids in document order for Qdrant references and return values
        let inserted_ids: Vec<String> = new_docs.iter()
            .map(|d| d.id.clone().unwrap_or_default())
            .collect();

        info!("Successfully inserted {} movies to MongoDB", inserted_ids.len());

        // Step 4: Bulk insert embeddings to Qdrant
        if !embeddings_to_insert.is_empty() {
            debug!("Inserting {} embeddings to Qdrant", embeddings_to_insert.len());
            self.insert_qdrant_points_bulk(&inserted_ids, &new_docs, embeddings_to_insert).await?;
        }

        // Return results (id, name pairs)
        let results: Vec<(String, String)> = inserted_ids.iter()
            .zip(new_docs.iter().map(|d| d.name.clone()))
            .map(|(id, name)| (id.clone(), name))
            .collect();

        info!("Bulk insert completed: {} movies inserted", results.len());
        Ok(results)
    }
}

#[async_trait]
impl crate::traits::InsertFullMovies for MongoMovieRepository {
    async fn insert_full_movies_bulk(&self, movies: Vec<FullMovie>) -> DbResult<usize> {
        use log::{debug, info};

        if movies.is_empty() {
            return Ok(0);
        }

        info!("Starting bulk insert of {} FullMovies", movies.len());

        // Duplicate detection
        let movie_names: Vec<String> = movies.iter().map(|m| m.name.clone()).collect();
        let existing_filter = doc! { "name": { "$in": &movie_names } };
        let mut existing_cursor = self.movies.find(existing_filter).await
            .map_err(|e| DatabaseError::QueryError(format!("MongoDB find error: {}", e)))?;

        let mut existing_names = std::collections::HashSet::new();
        while existing_cursor.advance().await
            .map_err(|e| DatabaseError::QueryError(format!("MongoDB cursor error: {}", e)))?
        {
            let doc: MovieDocument = existing_cursor.deserialize_current()
                .map_err(|e| DatabaseError::QueryError(format!("MongoDB deserialize error: {}", e)))?;
            existing_names.insert(doc.name);
        }

        // Split out embeddings; build documents preserving all fields
        let mut new_docs: Vec<MovieDocument> = Vec::new();
        let mut embeddings_to_insert: Vec<(usize, Vec<f32>, String)> = Vec::new();

        for mut movie in movies {
            if existing_names.contains(&movie.name) {
                debug!("Skipping duplicate movie: {}", movie.name);
                continue;
            }
            let embedding = movie.embedding.take();
            let name = movie.name.clone();
            let doc = MovieDocument::from(movie);
            let idx = new_docs.len();
            new_docs.push(doc);
            if let Some(emb) = embedding {
                embeddings_to_insert.push((idx, emb, name));
            }
        }

        if new_docs.is_empty() {
            info!("All movies already exist, nothing to insert");
            return Ok(0);
        }

        info!("Inserting {} new FullMovies to MongoDB", new_docs.len());
        let insert_result = self.movies.insert_many(&new_docs).await
            .map_err(|e| DatabaseError::QueryError(format!("MongoDB bulk insert error: {}", e)))?;

        if insert_result.inserted_ids.len() != new_docs.len() {
            return Err(DatabaseError::QueryError(format!(
                "Length mismatch: inserted {} but have {} documents",
                insert_result.inserted_ids.len(),
                new_docs.len()
            )));
        }

        // Write ObjectIds back into docs so Qdrant can reference them
        for (idx, bson_id) in &insert_result.inserted_ids {
            let hex = bson_id.as_object_id()
                .map(|oid| oid.to_hex())
                .ok_or_else(|| DatabaseError::QueryError(
                    "MongoDB did not return an ObjectId".to_string()))?;
            if let Some(doc) = new_docs.get_mut(*idx) {
                doc.id = Some(hex);
            }
        }

        let inserted_ids: Vec<String> = new_docs.iter()
            .map(|d| d.id.clone().unwrap_or_default())
            .collect();

        let count = inserted_ids.len();
        info!("Successfully inserted {} FullMovies to MongoDB", count);

        if !embeddings_to_insert.is_empty() {
            self.insert_qdrant_points_bulk(&inserted_ids, &new_docs, embeddings_to_insert).await?;
        }

        Ok(count)
    }
}

#[async_trait]
impl UpdateMovieLocation for MongoMovieRepository {
    async fn update_location(&self, movie_id: Self::Id, location: String) -> DbResult<()> {
        let oid = ObjectId::parse_str(&movie_id)
            .map_err(|e| DatabaseError::QueryError(format!("Invalid movie id '{}': {}", movie_id, e)))?;
        let filter = doc! { "_id": oid };
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

        let sort = doc! { "release_year": -1 }; // -1 = descending

        let mut cursor = self.movies.find(filter)
            .sort(sort)
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
    /// Uses Postgres-compatible scoring: title=100, actor=20, director=25, genre=15
    pub async fn search_movies_by_text(
        &self,
        query: &str,
        limit: i64,
    ) -> DbResult<Vec<(FullMovie, f32)>> {
        use mongodb::bson::Regex;

        if query.is_empty() {
            return Ok(Vec::new());
        }

        // Create regex pattern for case-insensitive search
        let pattern = Regex {
            pattern: query.to_string(),
            options: "i".to_string(), // case-insensitive
        };

        // Search across multiple fields (same as before)
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
            .limit(limit * 2) // Fetch more to allow for scoring/filtering
            .await
            .map_err(|e| DatabaseError::QueryError(format!("MongoDB text search error: {}", e)))?;

        let mut scored_results = Vec::new();
        while cursor.advance().await
            .map_err(|e| DatabaseError::QueryError(format!("MongoDB cursor error: {}", e)))?
        {
            let doc: MovieDocument = cursor.deserialize_current()
                .map_err(|e| DatabaseError::QueryError(format!("MongoDB deserialize error: {}", e)))?;

            // Convert to FullMovie and use centralized scoring
            let movie: FullMovie = doc.into();
            let score = movie.text_search_score(query);

            // Only include if there's an actual match with score > 0
            if score > 0.0 {
                scored_results.push((movie, score));
            }
        }

        // Sort by score descending (same as Postgres)
        scored_results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored_results.truncate(limit as usize);

        Ok(scored_results)
    }
}

// ==================== Stats Trait Implementations ====================

#[async_trait]
impl GetStatsOverview for MongoMovieRepository {
    async fn get_stats_overview(&self) -> DbResult<(i64, i64, i64)> {
        use mongodb::bson::doc;

        // Count total movies
        let total_movies = self.movies.count_documents(doc! {}).await
            .map_err(|e| DatabaseError::QueryError(format!("MongoDB count movies error: {}", e)))?;

        // Count unique directors using distinct
        let filter = doc! { "director": { "$exists": true, "$ne": null } };
        let directors = self.movies.distinct("director.name", filter).await
            .map_err(|e| DatabaseError::QueryError(format!("MongoDB distinct directors error: {}", e)))?;
        let total_directors = directors.len() as i64;

        // Count unique actors using aggregation (actors is an array)
        let pipeline = vec![
            doc! { "$unwind": "$actors" },
            doc! { "$group": { "_id": "$actors.name" } },
            doc! { "$count": "total" }
        ];
        let mut cursor = self.movies.aggregate(pipeline).await
            .map_err(|e| DatabaseError::QueryError(format!("MongoDB actor count error: {}", e)))?;

        let total_actors = if cursor.advance().await
            .map_err(|e| DatabaseError::QueryError(format!("MongoDB cursor error: {}", e)))?
        {
            let doc: mongodb::bson::Document = cursor.deserialize_current()
                .map_err(|e| DatabaseError::QueryError(format!("MongoDB deserialize error: {}", e)))?;
            doc.get_i32("total").unwrap_or(0) as i64
        } else {
            0
        };

        Ok((total_movies as i64, total_directors, total_actors))
    }
}

#[async_trait]
impl GetMoviesByYearStats for MongoMovieRepository {
    async fn get_movies_by_year(&self, limit: i64) -> DbResult<Vec<(i32, i64)>> {
        use mongodb::bson::doc;

        let pipeline = vec![
            doc! { "$group": {
                "_id": "$release_year",
                "count": { "$sum": 1 }
            }},
            doc! { "$sort": { "count": -1 } },
            doc! { "$limit": limit }
        ];

        let mut cursor = self.movies.aggregate(pipeline).await
            .map_err(|e| DatabaseError::QueryError(format!("MongoDB aggregation error: {}", e)))?;

        let mut results = Vec::new();
        while cursor.advance().await
            .map_err(|e| DatabaseError::QueryError(format!("MongoDB cursor error: {}", e)))?
        {
            let doc: mongodb::bson::Document = cursor.deserialize_current()
                .map_err(|e| DatabaseError::QueryError(format!("MongoDB deserialize error: {}", e)))?;
            let year = doc.get_i32("_id").unwrap_or(0);
            let count = doc.get_i64("count").unwrap_or(0);
            results.push((year, count));
        }

        Ok(results)
    }
}

#[async_trait]
impl GetGenreStats for MongoMovieRepository {
    async fn get_genre_counts(&self, limit: i64) -> DbResult<Vec<(String, i64)>> {
        use mongodb::bson::doc;

        let pipeline = vec![
            doc! { "$unwind": "$genres" },
            doc! { "$group": {
                "_id": "$genres",
                "count": { "$sum": 1 }
            }},
            doc! { "$sort": { "count": -1 } },
            doc! { "$limit": limit }
        ];

        let mut cursor = self.movies.aggregate(pipeline).await
            .map_err(|e| DatabaseError::QueryError(format!("MongoDB aggregation error: {}", e)))?;

        let mut results = Vec::new();
        while cursor.advance().await
            .map_err(|e| DatabaseError::QueryError(format!("MongoDB cursor error: {}", e)))?
        {
            let doc: mongodb::bson::Document = cursor.deserialize_current()
                .map_err(|e| DatabaseError::QueryError(format!("MongoDB deserialize error: {}", e)))?;
            if let Some(genre) = doc.get_str("_id").ok() {
                let count = doc.get_i64("count").unwrap_or(0);
                results.push((genre.to_string(), count));
            }
        }

        Ok(results)
    }
}

#[async_trait]
impl GetTopActorsStats for MongoMovieRepository {
    async fn get_top_actors(&self, limit: i64) -> DbResult<Vec<(String, i64)>> {
        use mongodb::bson::doc;

        let pipeline = vec![
            doc! { "$unwind": "$actors" },
            doc! { "$group": {
                "_id": "$actors.name",
                "count": { "$sum": 1 }
            }},
            doc! { "$sort": { "count": -1 } },
            doc! { "$limit": limit }
        ];

        let mut cursor = self.movies.aggregate(pipeline).await
            .map_err(|e| DatabaseError::QueryError(format!("MongoDB aggregation error: {}", e)))?;

        let mut results = Vec::new();
        while cursor.advance().await
            .map_err(|e| DatabaseError::QueryError(format!("MongoDB cursor error: {}", e)))?
        {
            let doc: mongodb::bson::Document = cursor.deserialize_current()
                .map_err(|e| DatabaseError::QueryError(format!("MongoDB deserialize error: {}", e)))?;
            if let Some(actor) = doc.get_str("_id").ok() {
                let count = doc.get_i64("count").unwrap_or(0);
                results.push((actor.to_string(), count));
            }
        }

        Ok(results)
    }
}
}

#[cfg(feature = "mongodb")]
pub use inner::*;
