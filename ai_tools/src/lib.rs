//! AI Tools - blocking diesel + spawn_blocking approach

use log::info;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub use ollama_rs::generation::tools::Tool;

// Param types
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct FilterByActorParams {
    #[schemars(description = "Actor name")]
    pub actor_name: String,
    #[serde(default = "def_20")]
    pub limit: usize,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct FilterByGenreParams {
    #[schemars(description = "Genre")]
    pub genre: String,
    #[serde(default = "def_20")]
    pub limit: usize,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct FilterByDirectorParams {
    #[schemars(description = "Director name")]
    pub director_name: String,
    #[serde(default = "def_20")]
    pub limit: usize,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct GetMovieDetailsParams {
    #[schemars(description = "Movie title")]
    pub title: String,
}

fn def_20() -> usize { 20 }

// Tools
#[derive(Default)]
pub struct FilterByActorTool;

impl Tool for FilterByActorTool {
    type Params = FilterByActorParams;
    fn name() -> &'static str { "filter_by_actor" }
    fn description() -> &'static str { "Filter by actor" }

    async fn call(&mut self, params: Self::Params) -> ollama_rs::generation::tools::Result<String> {
        info!("filter_by_actor: {}", params.actor_name);
        
        tokio::task::spawn_blocking(move || {
            tokio::runtime::Handle::current().block_on(async move {
                let pool = database::get_connection_pool().await?;
                let repo = database::postgres::PostgresMovieRepository::new(pool.clone());
                
                use database::schema::*;
                use diesel::prelude::*;
                use diesel_async::RunQueryDsl;
                
                let mut conn = pool.get().await.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
                let movie_ids: Vec<i32> = movie_actor::table
                    .inner_join(actor::table)
                    .select(movie_actor::movie_id)
                    .filter(actor::name.ilike(format!("%{}%", params.actor_name)))
                    .limit(params.limit as i64)
                    .load(&mut conn)
                    .await?;
                
                let movies = database::traits::GetMoviesByIds::get_by_ids(&repo, movie_ids).await?;
                Ok::<_, Box<dyn std::error::Error + Send + Sync>>(serde_json::to_string(&movies)?)
            })
        }).await?
    }
}

#[derive(Default)]
pub struct FilterByGenreTool;

impl Tool for FilterByGenreTool {
    type Params = FilterByGenreParams;
    fn name() -> &'static str { "filter_by_genre" }
    fn description() -> &'static str { "Filter by genre" }

    async fn call(&mut self, params: Self::Params) -> ollama_rs::generation::tools::Result<String> {
        info!("filter_by_genre: {}", params.genre);
        
        tokio::task::spawn_blocking(move || {
            tokio::runtime::Handle::current().block_on(async move {
                let pool = database::get_connection_pool().await?;
                let repo = database::postgres::PostgresMovieRepository::new(pool.clone());
                
                use database::schema::*;
                use diesel::prelude::*;
                use diesel_async::RunQueryDsl;
                
                let mut conn = pool.get().await.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
                let movie_ids: Vec<i32> = movie_genre::table
                    .select(movie_genre::movie_id)
                    .filter(movie_genre::genre.ilike(format!("%{}%", params.genre)))
                    .limit(params.limit as i64)
                    .load(&mut conn)
                    .await?;
                
                let movies = database::traits::GetMoviesByIds::get_by_ids(&repo, movie_ids).await?;
                Ok::<_, Box<dyn std::error::Error + Send + Sync>>(serde_json::to_string(&movies)?)
            })
        }).await?
    }
}

#[derive(Default)]
pub struct FilterByDirectorTool;

impl Tool for FilterByDirectorTool {
    type Params = FilterByDirectorParams;
    fn name() -> &'static str { "filter_by_director" }
    fn description() -> &'static str { "Filter by director" }

    async fn call(&mut self, params: Self::Params) -> ollama_rs::generation::tools::Result<String> {
        info!("filter_by_director: {}", params.director_name);
        
        tokio::task::spawn_blocking(move || {
            tokio::runtime::Handle::current().block_on(async move {
                let pool = database::get_connection_pool().await?;
                let repo = database::postgres::PostgresMovieRepository::new(pool.clone());
                
                use database::schema::*;
                use diesel::prelude::*;
                use diesel_async::RunQueryDsl;
                
                let mut conn = pool.get().await.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
                let movie_ids: Vec<i32> = movie::table
                    .inner_join(director::table)
                    .select(movie::id)
                    .filter(director::name.ilike(format!("%{}%", params.director_name)))
                    .limit(params.limit as i64)
                    .load(&mut conn)
                    .await?;
                
                let movies = database::traits::GetMoviesByIds::get_by_ids(&repo, movie_ids).await?;
                Ok::<_, Box<dyn std::error::Error + Send + Sync>>(serde_json::to_string(&movies)?)
            })
        }).await?
    }
}

#[derive(Default)]
pub struct GetMovieDetailsTool;

impl Tool for GetMovieDetailsTool {
    type Params = GetMovieDetailsParams;
    fn name() -> &'static str { "get_movie_details" }
    fn description() -> &'static str { "Get movie by title" }

    async fn call(&mut self, params: Self::Params) -> ollama_rs::generation::tools::Result<String> {
        info!("get_movie_details: {}", params.title);
        
        tokio::task::spawn_blocking(move || {
            tokio::runtime::Handle::current().block_on(async move {
                let pool = database::get_connection_pool().await?;
                let repo = database::postgres::PostgresMovieRepository::new(pool.clone());
                
                use database::schema::*;
                use diesel::prelude::*;
                use diesel_async::RunQueryDsl;
                
                let mut conn = pool.get().await.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
                let movie_ids: Vec<i32> = movie::table
                    .select(movie::id)
                    .filter(movie::name.ilike(format!("%{}%", params.title)))
                    .limit(5)
                    .load(&mut conn)
                    .await?;
                
                let movies = database::traits::GetMoviesByIds::get_by_ids(&repo, movie_ids).await?;
                Ok::<_, Box<dyn std::error::Error + Send + Sync>>(serde_json::to_string(&movies)?)
            })
        }).await?
    }
}
