use async_trait::async_trait;
use bson::{doc, oid::ObjectId, Bson, Document};
use log::debug;
use models::{
    Actor, Director, FullMovie, NewMovie, StructuredQuery,
};
use mongodb::{options::ClientOptions, Client, Collection};
use serde::{Deserialize, Serialize};
use std::env;

use crate::traits::*;
use crate::DatabaseError;

// ─────────────────────────────────────────────────────────────────
// Denormalised BSON document model
// ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MongoMovie {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub oid: Option<ObjectId>,
    pub numeric_id: i32,
    pub name: String,
    pub description: Option<String>,
    pub director: Option<MongoEmbeddedDirector>,
    pub actors: Vec<MongoEmbeddedActor>,
    pub genres: Vec<String>,
    pub embedding: Option<Vec<f64>>,
    pub added_on: Option<String>,
    pub location: Option<String>,
    pub release_year: i32,
    pub key_hash: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MongoEmbeddedActor {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MongoEmbeddedDirector {
    pub id: i32,
    pub name: String,
}

// ─────────────────────────────────────────────────────────────────
// Conversions: FullMovie ↔ MongoMovie
// ─────────────────────────────────────────────────────────────────

impl From<&FullMovie> for MongoMovie {
    fn from(m: &FullMovie) -> Self {
        Self {
            oid: None,
            numeric_id: m.id,
            name: m.name.clone(),
            description: m.description.clone(),
            director: m.director.as_ref().map(|d| MongoEmbeddedDirector {
                id: d.id,
                name: d.name.clone(),
            }),
            actors: m
                .actors
                .iter()
                .map(|a| MongoEmbeddedActor {
                    id: a.id,
                    name: a.name.clone(),
                })
                .collect(),
            genres: m.genres.clone(),
            embedding: m.embedding.as_ref().map(|v| v.iter().map(|&x| x as f64).collect()),
            added_on: m.added_on.clone(),
            location: m.location.clone(),
            release_year: m.release_year,
            key_hash: m.key_hash as i64,
        }
    }
}

impl From<MongoMovie> for FullMovie {
    fn from(m: MongoMovie) -> Self {
        FullMovie {
            id: m.numeric_id,
            key_hash: m.key_hash as u64,
            name: m.name,
            description: m.description,
            director: m.director.map(|d| Director { id: d.id, name: d.name }),
            actors: m
                .actors
                .into_iter()
                .map(|a| Actor { id: a.id, name: a.name })
                .collect(),
            genres: m.genres,
            embedding: m.embedding.map(|v| v.into_iter().map(|x| x as f32).collect()),
            added_on: m.added_on,
            location: m.location,
            release_year: m.release_year,
        }
    }
}

// ─────────────────────────────────────────────────────────────────
// Connection helpers
// ─────────────────────────────────────────────────────────────────

pub async fn get_mongo_client() -> Result<Client, DatabaseError> {
    let uri = env::var("MONGODB_URL")
        .or_else(|_| env::var("MONGO_URL"))
        .unwrap_or_else(|_| "mongodb://localhost:27017".to_string());
    let opts = ClientOptions::parse(&uri).await?;
    Client::with_options(opts).map_err(DatabaseError::from)
}

pub fn movie_collection(client: &Client) -> Collection<MongoMovie> {
    let db_name = env::var("MONGODB_DATABASE").unwrap_or_else(|_| "dvd_catalog".to_string());
    client.database(&db_name).collection("movies")
}

// ─────────────────────────────────────────────────────────────────
// Shared internal counter (numeric id via findOneAndUpdate on a counters coll)
// ─────────────────────────────────────────────────────────────────

pub async fn next_numeric_id(client: &Client) -> Result<i32, DatabaseError> {
    let db_name = env::var("MONGODB_DATABASE").unwrap_or_else(|_| "dvd_catalog".to_string());
    let counters: Collection<Document> = client.database(&db_name).collection("counters");
    let result = counters
        .find_one_and_update(
            doc! { "_id": "movies" },
            doc! { "$inc": { "seq": 1_i32 } },
        )
        .upsert(true)
        .return_document(mongodb::options::ReturnDocument::After)
        .await?
        .ok_or(DatabaseError::NotFound)?;
    let seq = result
        .get_i32("seq")
        .map_err(|_| DatabaseError::NotFound)?;
    Ok(seq)
}

// ─────────────────────────────────────────────────────────────────
// StructuredQuery → MongoDB filter document
// ─────────────────────────────────────────────────────────────────

pub fn structured_query_to_filter(query: &StructuredQuery) -> Document {
    let mut conditions: Vec<Document> = Vec::new();

    if !query.title_keywords.is_empty() {
        let regex_parts: Vec<Document> = query
            .title_keywords
            .iter()
            .map(|kw| doc! { "name": { "$regex": kw, "$options": "i" } })
            .collect();
        conditions.push(doc! { "$and": regex_parts });
    }

    if !query.directors.is_empty() {
        let director_conds: Vec<Document> = query
            .directors
            .iter()
            .map(|d| doc! { "director.name": { "$regex": d, "$options": "i" } })
            .collect();
        conditions.push(doc! { "$or": director_conds });
    }

    if !query.actors.is_empty() {
        let actor_conds: Vec<Document> = query
            .actors
            .iter()
            .map(|a| doc! { "actors.name": { "$regex": a, "$options": "i" } })
            .collect();
        conditions.push(doc! { "$or": actor_conds });
    }

    if !query.genres.is_empty() {
        let genre_conds: Vec<Document> = query
            .genres
            .iter()
            .map(|g| doc! { "genres": { "$regex": g, "$options": "i" } })
            .collect();
        conditions.push(doc! { "$or": genre_conds });
    }

    if !query.description_keywords.is_empty() {
        let desc_conds: Vec<Document> = query
            .description_keywords
            .iter()
            .map(|kw| doc! { "description": { "$regex": kw, "$options": "i" } })
            .collect();
        conditions.push(doc! { "$or": desc_conds });
    }

    if conditions.is_empty() {
        doc! {}
    } else {
        doc! { "$and": conditions }
    }
}

// ─────────────────────────────────────────────────────────────────
// MongoMovieRepository — CRUD (no vector search, provided by subtypes)
// ─────────────────────────────────────────────────────────────────

pub struct MongoMovieRepository {
    pub(crate) client: Client,
}

impl MongoMovieRepository {
    pub async fn new() -> Result<Self, DatabaseError> {
        let client = get_mongo_client().await?;
        Ok(Self { client })
    }

    pub(crate) fn collection(&self) -> Collection<MongoMovie> {
        movie_collection(&self.client)
    }
}

// ─────────────────────────────────────────────────────────────────
// Trait implementations on MongoMovieRepository
// ─────────────────────────────────────────────────────────────────

#[async_trait]
impl GetAllMovies for MongoMovieRepository {
    async fn get_all(&mut self) -> Result<Vec<FullMovie>, DatabaseError> {
        use futures::TryStreamExt;
        let cursor = self.collection().find(doc! {}).await?;
        let docs: Vec<MongoMovie> = cursor.try_collect().await?;
        debug!("MongoMovieRepository::get_all found {} movies", docs.len());
        Ok(docs.into_iter().map(FullMovie::from).collect())
    }
}

#[async_trait]
impl GetMovieById for MongoMovieRepository {
    async fn get_by_id(&mut self, id: i32) -> Result<FullMovie, DatabaseError> {
        self.collection()
            .find_one(doc! { "numeric_id": id })
            .await?
            .map(FullMovie::from)
            .ok_or(DatabaseError::NotFound)
    }
}

#[async_trait]
impl GetMoviesByIds for MongoMovieRepository {
    async fn get_by_ids(&mut self, ids: Vec<i32>) -> Result<Vec<FullMovie>, DatabaseError> {
        use futures::TryStreamExt;
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let bson_ids: Vec<Bson> = ids.iter().map(|&i| Bson::Int32(i)).collect();
        let cursor = self
            .collection()
            .find(doc! { "numeric_id": { "$in": bson_ids } })
            .await?;
        let docs: Vec<MongoMovie> = cursor.try_collect().await?;
        let mut map: std::collections::HashMap<i32, FullMovie> =
            docs.into_iter().map(|d| (d.numeric_id, FullMovie::from(d))).collect();
        Ok(ids.into_iter().filter_map(|id| map.remove(&id)).collect())
    }
}

#[async_trait]
impl InsertMovie for MongoMovieRepository {
    async fn insert(&mut self, full_movie: FullMovie) -> Result<FullMovie, DatabaseError> {
        let id = next_numeric_id(&self.client).await?;
        let mut doc = MongoMovie::from(&full_movie);
        doc.numeric_id = id;

        self.collection().insert_one(&doc).await?;
        debug!("MongoMovieRepository::insert id={}", id);

        GetMovieById::get_by_id(self, id).await
    }
}

#[async_trait]
impl InsertMovies for MongoMovieRepository {
    async fn insert_batch(
        &mut self,
        new_movies: &[NewMovie],
    ) -> Result<Vec<(i32, String)>, DatabaseError> {
        let mut results = Vec::with_capacity(new_movies.len());
        for nm in new_movies {
            let id = next_numeric_id(&self.client).await?;
            let doc = MongoMovie {
                oid: None,
                numeric_id: id,
                name: nm.name.clone(),
                description: nm.description.clone(),
                director: None,
                actors: Vec::new(),
                genres: Vec::new(),
                embedding: None,
                added_on: None,
                location: nm.location.clone(),
                release_year: nm.release_year,
                key_hash: FullMovie::generate_key_hash(&nm.name) as i64,
            };
            self.collection().insert_one(&doc).await?;
            results.push((id, nm.name.clone()));
        }
        Ok(results)
    }
}

#[async_trait]
impl UpdateMovieLocation for MongoMovieRepository {
    async fn update_location(
        &mut self,
        movie_id: i32,
        new_location: String,
    ) -> Result<(), DatabaseError> {
        self.collection()
            .update_one(
                doc! { "numeric_id": movie_id },
                doc! { "$set": { "location": &new_location } },
            )
            .await?;
        Ok(())
    }
}

#[async_trait]
impl GetRecentMovies for MongoMovieRepository {
    async fn get_recent(&mut self, limit: i64) -> Result<Vec<FullMovie>, DatabaseError> {
        use futures::TryStreamExt;
        let cursor = self
            .collection()
            .find(doc! {})
            .sort(doc! { "added_on": -1_i32 })
            .limit(limit)
            .await?;
        let docs: Vec<MongoMovie> = cursor.try_collect().await?;
        debug!("MongoMovieRepository::get_recent found {} movies", docs.len());
        Ok(docs.into_iter().map(FullMovie::from).collect())
    }
}

#[async_trait]
impl GetMoviesByReleaseYear for MongoMovieRepository {
    async fn get_by_release_year(
        &mut self,
        min_year: i32,
        max_year: i32,
        limit: i64,
    ) -> Result<Vec<FullMovie>, DatabaseError> {
        use futures::TryStreamExt;
        let cursor = self
            .collection()
            .find(doc! { "release_year": { "$gte": min_year, "$lte": max_year } })
            .sort(doc! { "release_year": -1_i32 })
            .limit(limit)
            .await?;
        let docs: Vec<MongoMovie> = cursor.try_collect().await?;
        debug!(
            "MongoMovieRepository::get_by_release_year({}-{}) found {}",
            min_year, max_year, docs.len()
        );
        Ok(docs.into_iter().map(FullMovie::from).collect())
    }
}

#[async_trait]
impl RandomMovies for MongoMovieRepository {
    async fn get_random(&mut self, count: i64) -> Result<Vec<FullMovie>, DatabaseError> {
        use futures::TryStreamExt;
        let pipeline = vec![doc! { "$sample": { "size": count } }];
        let cursor = self
            .collection()
            .aggregate(pipeline)
            .await?;
        let docs: Vec<Document> = cursor.try_collect().await?;
        let movies: Vec<FullMovie> = docs
            .into_iter()
            .filter_map(|d| {
                bson::from_document::<MongoMovie>(d)
                    .ok()
                    .map(FullMovie::from)
            })
            .collect();
        debug!("MongoMovieRepository::get_random returned {} movies", movies.len());
        Ok(movies)
    }
}

#[async_trait]
impl GetUnknownLocationMovies for MongoMovieRepository {
    async fn get_unknown_location(&mut self, limit: i64) -> Result<Vec<FullMovie>, DatabaseError> {
        use futures::TryStreamExt;
        let cursor = self
            .collection()
            .find(doc! { "location": "Unknown" })
            .sort(doc! { "added_on": -1_i32 })
            .limit(limit)
            .await?;
        let docs: Vec<MongoMovie> = cursor.try_collect().await?;
        Ok(docs.into_iter().map(FullMovie::from).collect())
    }
}

#[async_trait]
impl GetUniqueLocations for MongoMovieRepository {
    async fn unique_locations(&mut self) -> Result<Vec<String>, DatabaseError> {
        let results = self
            .collection()
            .distinct("location", doc! {})
            .await?;
        Ok(results
            .into_iter()
            .filter_map(|b| {
                if let Bson::String(s) = b {
                    Some(s)
                } else {
                    None
                }
            })
            .collect())
    }
}

#[async_trait]
impl MoviesByLocation for MongoMovieRepository {
    async fn movies_by_location(&mut self, location: &str) -> Result<Vec<FullMovie>, DatabaseError> {
        use futures::TryStreamExt;
        let cursor = self
            .collection()
            .find(doc! { "location": location })
            .await?;
        let docs: Vec<MongoMovie> = cursor.try_collect().await?;
        Ok(docs.into_iter().map(FullMovie::from).collect())
    }
}

#[async_trait]
impl SearchMoviesStructured for MongoMovieRepository {
    async fn search_structured(
        &mut self,
        query: &StructuredQuery,
    ) -> Result<Vec<FullMovie>, DatabaseError> {
        use futures::TryStreamExt;
        let filter = structured_query_to_filter(query);
        let cursor = self.collection().find(filter).await?;
        let docs: Vec<MongoMovie> = cursor.try_collect().await?;
        debug!(
            "MongoMovieRepository::search_structured returned {} movies",
            docs.len()
        );
        Ok(docs.into_iter().map(FullMovie::from).collect())
    }
}
