use async_trait::async_trait;
use log::debug;
use models::{FullMovie, NewMovie, StructuredQuery};
use redis::AsyncCommands;
use std::env;

use crate::mongo_core::MongoMovieRepository;
use crate::traits::*;
use crate::DatabaseError;

// ─────────────────────────────────────────────────────────────────
// Redis helpers
// ─────────────────────────────────────────────────────────────────

/// Redis key for a movie vector: `movie:vec:<numeric_id>`
fn vector_key(id: i32) -> String {
    format!("movie:vec:{}", id)
}

/// Serialise f32 slice → little-endian bytes (FLOAT32 BLOB for Redis Stack)
fn embedding_to_bytes(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|f| f.to_le_bytes()).collect()
}

/// Parse a raw Redis KNN reply into ordered numeric IDs.
/// FT.SEARCH returns: [total, key1, [fields...], key2, [fields...], ...]
/// We only care about the keys (alternating positions starting at index 1).
fn parse_knn_reply(raw: &redis::Value) -> Vec<i32> {
    let items = match raw {
        redis::Value::Array(v) => v,
        _ => return Vec::new(),
    };
    // items[0] = total count, then pairs of (key, fields)
    items
        .iter()
        .skip(1)
        .step_by(2)
        .filter_map(|v| {
            if let redis::Value::BulkString(b) = v {
                let key = String::from_utf8_lossy(b);
                // key format: "movie:vec:<id>"
                key.rsplit(':').next().and_then(|s| s.parse::<i32>().ok())
            } else {
                None
            }
        })
        .collect()
}

// ─────────────────────────────────────────────────────────────────
// Repository struct
// ─────────────────────────────────────────────────────────────────

pub struct MongoRedisMovieRepository {
    pub mongo: MongoMovieRepository,
    pub redis: redis::aio::ConnectionManager,
}

impl MongoRedisMovieRepository {
    pub async fn new() -> Result<Self, DatabaseError> {
        let mongo = MongoMovieRepository::new().await?;
        let redis_url = env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());
        let client = redis::Client::open(redis_url.as_str())?;
        let redis = redis::aio::ConnectionManager::new(client).await?;
        let mut repo = Self { mongo, redis };
        repo.ensure_index(768).await?;
        Ok(repo)
    }

    /// Store embedding in Redis Stack as a HASH with a FLOAT32 BLOB field.
    /// Assumes the Redis Stack index `movie_vec_idx` already exists (created at startup).
    pub async fn store_embedding(&mut self, id: i32, embedding: &[f32]) -> Result<(), DatabaseError> {
        let key = vector_key(id);
        let bytes = embedding_to_bytes(embedding);
        redis::cmd("HSET")
            .arg(&key)
            .arg("numeric_id")
            .arg(id)
            .arg("embedding")
            .arg(bytes)
            .exec_async(&mut self.redis)
            .await?;
        Ok(())
    }

    /// Delete embedding from Redis.
    pub async fn delete_embedding(&mut self, id: i32) -> Result<(), DatabaseError> {
        let key = vector_key(id);
        self.redis.del::<_, ()>(key).await?;
        Ok(())
    }

    /// Create the FT index on first run if it doesn't exist.
    /// Dimension is 768 (nomic-embed-text). Adjust if using a different model.
    pub async fn ensure_index(&mut self, dim: usize) -> Result<(), DatabaseError> {
        let result: redis::RedisResult<()> = redis::cmd("FT.CREATE")
            .arg("movie_vec_idx")
            .arg("ON")
            .arg("HASH")
            .arg("PREFIX")
            .arg("1")
            .arg("movie:vec:")
            .arg("SCHEMA")
            .arg("numeric_id")
            .arg("NUMERIC")
            .arg("SORTABLE")
            .arg("embedding")
            .arg("VECTOR")
            .arg("HNSW")
            .arg("6")
            .arg("TYPE")
            .arg("FLOAT32")
            .arg("DIM")
            .arg(dim)
            .arg("DISTANCE_METRIC")
            .arg("COSINE")
            .exec_async(&mut self.redis)
            .await;

        match result {
            Ok(_) => {
                debug!("Created Redis vector index movie_vec_idx (dim={})", dim);
                Ok(())
            }
            Err(e) => {
                // ERR Index already exists — treat as success
                let msg = e.to_string();
                if msg.contains("already exists") {
                    Ok(())
                } else {
                    Err(DatabaseError::RedisError(e))
                }
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────
// Delegate all CRUD traits to inner MongoMovieRepository
// ─────────────────────────────────────────────────────────────────

#[async_trait]
impl GetAllMovies for MongoRedisMovieRepository {
    async fn get_all(&mut self) -> Result<Vec<FullMovie>, DatabaseError> {
        self.mongo.get_all().await
    }
}

#[async_trait]
impl GetMovieById for MongoRedisMovieRepository {
    async fn get_by_id(&mut self, id: i32) -> Result<FullMovie, DatabaseError> {
        self.mongo.get_by_id(id).await
    }
}

#[async_trait]
impl GetMoviesByIds for MongoRedisMovieRepository {
    async fn get_by_ids(&mut self, ids: Vec<i32>) -> Result<Vec<FullMovie>, DatabaseError> {
        self.mongo.get_by_ids(ids).await
    }
}

#[async_trait]
impl InsertMovie for MongoRedisMovieRepository {
    async fn insert(&mut self, full_movie: FullMovie) -> Result<FullMovie, DatabaseError> {
        let inserted = self.mongo.insert(full_movie).await?;
        if let Some(ref emb) = inserted.embedding {
            if let Err(e) = self.store_embedding(inserted.id, emb).await {
                log::error!("Failed to store embedding in Redis for movie {}: {}", inserted.id, e);
            }
        }
        Ok(inserted)
    }
}

#[async_trait]
impl InsertMovies for MongoRedisMovieRepository {
    async fn insert_batch(
        &mut self,
        new_movies: &[NewMovie],
    ) -> Result<Vec<(i32, String)>, DatabaseError> {
        self.mongo.insert_batch(new_movies).await
    }
}

#[async_trait]
impl UpdateMovieLocation for MongoRedisMovieRepository {
    async fn update_location(
        &mut self,
        movie_id: i32,
        new_location: String,
    ) -> Result<(), DatabaseError> {
        self.mongo.update_location(movie_id, new_location).await
    }
}

#[async_trait]
impl GetRecentMovies for MongoRedisMovieRepository {
    async fn get_recent(&mut self, limit: i64) -> Result<Vec<FullMovie>, DatabaseError> {
        self.mongo.get_recent(limit).await
    }
}

#[async_trait]
impl GetMoviesByReleaseYear for MongoRedisMovieRepository {
    async fn get_by_release_year(
        &mut self,
        min_year: i32,
        max_year: i32,
        limit: i64,
    ) -> Result<Vec<FullMovie>, DatabaseError> {
        self.mongo.get_by_release_year(min_year, max_year, limit).await
    }
}

#[async_trait]
impl RandomMovies for MongoRedisMovieRepository {
    async fn get_random(&mut self, count: i64) -> Result<Vec<FullMovie>, DatabaseError> {
        self.mongo.get_random(count).await
    }
}

#[async_trait]
impl GetUnknownLocationMovies for MongoRedisMovieRepository {
    async fn get_unknown_location(&mut self, limit: i64) -> Result<Vec<FullMovie>, DatabaseError> {
        self.mongo.get_unknown_location(limit).await
    }
}

#[async_trait]
impl GetUniqueLocations for MongoRedisMovieRepository {
    async fn unique_locations(&mut self) -> Result<Vec<String>, DatabaseError> {
        self.mongo.unique_locations().await
    }
}

#[async_trait]
impl MoviesByLocation for MongoRedisMovieRepository {
    async fn movies_by_location(&mut self, location: &str) -> Result<Vec<FullMovie>, DatabaseError> {
        self.mongo.movies_by_location(location).await
    }
}

#[async_trait]
impl SearchMoviesStructured for MongoRedisMovieRepository {
    async fn search_structured(
        &mut self,
        query: &StructuredQuery,
    ) -> Result<Vec<FullMovie>, DatabaseError> {
        self.mongo.search_structured(query).await
    }
}

// ─────────────────────────────────────────────────────────────────
// Vector search via Redis Stack FT.SEARCH KNN
// ─────────────────────────────────────────────────────────────────

#[async_trait]
impl SearchMoviesByEmbedding for MongoRedisMovieRepository {
    async fn search_by_embedding(
        &mut self,
        embedding: Vec<f32>,
        limit: i64,
    ) -> Result<Vec<FullMovie>, DatabaseError> {
        let bytes = embedding_to_bytes(&embedding);

        // FT.SEARCH movie_vec_idx "*=>[KNN $K @embedding $BLOB]"
        // PARAMS 4 K <limit> BLOB <bytes>
        // RETURN 1 numeric_id
        // SORTBY __embedding_score
        // DIALECT 2
        let raw: redis::Value = redis::cmd("FT.SEARCH")
            .arg("movie_vec_idx")
            .arg("*=>[KNN $K @embedding $BLOB]")
            .arg("PARAMS")
            .arg(4_i64)
            .arg("K")
            .arg(limit)
            .arg("BLOB")
            .arg(bytes)
            .arg("RETURN")
            .arg(1_i64)
            .arg("numeric_id")
            .arg("SORTBY")
            .arg("__embedding_score")
            .arg("DIALECT")
            .arg(2_i64)
            .query_async(&mut self.redis)
            .await?;

        let ids = parse_knn_reply(&raw);
        debug!(
            "MongoRedisMovieRepository::search_by_embedding KNN returned {} ids",
            ids.len()
        );

        if ids.is_empty() {
            return Ok(Vec::new());
        }

        use bson::Bson;
        use futures::TryStreamExt;

        let bson_ids: Vec<Bson> = ids.iter().map(|&i| Bson::Int32(i)).collect();
        let cursor = self
            .mongo
            .collection()
            .find(bson::doc! { "numeric_id": { "$in": bson_ids } })
            .await?;
        let docs: Vec<crate::mongo_core::MongoMovie> = cursor.try_collect().await?;

        // Restore Redis KNN score order
        let mut map: std::collections::HashMap<i32, FullMovie> =
            docs.into_iter().map(|d| (d.numeric_id, FullMovie::from(d))).collect();
        Ok(ids.into_iter().filter_map(|id| map.remove(&id)).collect())
    }
}
