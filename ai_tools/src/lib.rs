//! AI Tools — calls `dvd_catalog_api` over HTTP via `reqwest`.
//!
//! Tools cannot use diesel-async directly because its futures aren't `Sync`,
//! and `ollama_rs::generation::tools::Tool::call` requires `Send + Sync`.
//! `reqwest::Response` futures *are* `Send + Sync`, so we self-call the API.

use log::{error, info};
use models::{FullMovie, StructuredQuery};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub use ollama_rs::generation::tools::Tool;

type ToolResult<T> = ollama_rs::generation::tools::Result<T>;
type BoxErr = Box<dyn std::error::Error + Send + Sync>;

const DEFAULT_API_URL: &str = "http://localhost:3000";
const DEFAULT_LIMIT: usize = 20;

/// Shared HTTP client + base URL for hitting `dvd_catalog_api`.
/// Cheap to `clone` (internally `Arc`).
#[derive(Debug, Clone)]
pub struct ApiClient {
    base_url: String,
    http: reqwest::Client,
}

impl ApiClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            http: reqwest::Client::new(),
        }
    }

    /// Build an `ApiClient` from the `DVD_CATALOG_API_URL` env var,
    /// falling back to `http://localhost:3000`.
    pub fn from_env() -> Self {
        let base_url =
            std::env::var("DVD_CATALOG_API_URL").unwrap_or_else(|_| DEFAULT_API_URL.to_string());
        Self::new(base_url)
    }

    async fn structured_search(&self, query: &StructuredQuery) -> Result<Vec<FullMovie>, BoxErr> {
        let url = format!(
            "{}/dvd/structured-search",
            self.base_url
        );
        let resp = self
            .http
            .post(&url)
            .json(query)
            .send()
            .await?;
        if !resp
            .status()
            .is_success()
        {
            return Err(
                format!(
                    "structured-search returned {}",
                    resp.status()
                )
                .into(),
            );
        }
        let movies: Vec<FullMovie> = resp
            .json()
            .await?;
        Ok(movies)
    }
}

impl Default for ApiClient {
    fn default() -> Self {
        Self::from_env()
    }
}

fn truncate(mut movies: Vec<FullMovie>, limit: usize) -> Vec<FullMovie> {
    if movies.len() > limit {
        movies.truncate(limit);
    }
    movies
}

fn def_limit() -> usize {
    DEFAULT_LIMIT
}

// ---------- Param types ----------

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct FilterByActorParams {
    #[schemars(description = "Actor name (partial matches allowed)")]
    pub actor_name: String,
    #[serde(default = "def_limit")]
    #[schemars(description = "Maximum number of movies to return")]
    pub limit: usize,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct FilterByGenreParams {
    #[schemars(description = "Genre (partial matches allowed)")]
    pub genre: String,
    #[serde(default = "def_limit")]
    #[schemars(description = "Maximum number of movies to return")]
    pub limit: usize,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct FilterByDirectorParams {
    #[schemars(description = "Director name (partial matches allowed)")]
    pub director_name: String,
    #[serde(default = "def_limit")]
    #[schemars(description = "Maximum number of movies to return")]
    pub limit: usize,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct GetMovieDetailsParams {
    #[schemars(description = "Movie title (partial matches allowed)")]
    pub title: String,
}

// ---------- Tools ----------

#[derive(Debug, Clone, Default)]
pub struct FilterByActorTool {
    pub api: ApiClient,
}

impl FilterByActorTool {
    pub fn new(api: ApiClient) -> Self {
        Self { api }
    }
}

impl Tool for FilterByActorTool {
    type Params = FilterByActorParams;
    fn name() -> &'static str {
        "filter_by_actor"
    }
    fn description() -> &'static str {
        "Returns movies featuring the given actor as a JSON array."
    }

    async fn call(&mut self, params: Self::Params) -> ToolResult<String> {
        info!(
            "filter_by_actor: {}",
            params.actor_name
        );
        let query = StructuredQuery {
            actors: vec![params.actor_name],
            ..Default::default()
        };
        let movies = self
            .api
            .structured_search(&query)
            .await
            .map_err(
                |e| {
                    error!(
                        "filter_by_actor failed: {}",
                        e
                    );
                    e
                },
            )?;
        let movie_titles: Vec<String> = movies
            .iter()
            .map(
                |m| {
                    format!(
                        "{} ({})",
                        m.name, m.release_year
                    )
                },
            )
            .collect();
        info!(
            "filter_by_actor sending {} movies to AI: {:?}",
            movie_titles.len(),
            movie_titles
        );
        Ok(
            serde_json::to_string(
                &truncate(
                    movies,
                    params.limit,
                ),
            )?,
        )
    }
}

#[derive(Debug, Clone, Default)]
pub struct FilterByGenreTool {
    pub api: ApiClient,
}

impl FilterByGenreTool {
    pub fn new(api: ApiClient) -> Self {
        Self { api }
    }
}

impl Tool for FilterByGenreTool {
    type Params = FilterByGenreParams;
    fn name() -> &'static str {
        "filter_by_genre"
    }
    fn description() -> &'static str {
        "Returns movies matching the given genre as a JSON array."
    }

    async fn call(&mut self, params: Self::Params) -> ToolResult<String> {
        info!(
            "filter_by_genre: {}",
            params.genre
        );
        let query = StructuredQuery {
            genres: vec![params.genre],
            ..Default::default()
        };
        let movies = self
            .api
            .structured_search(&query)
            .await
            .map_err(
                |e| {
                    error!(
                        "filter_by_genre failed: {}",
                        e
                    );
                    e
                },
            )?;
        let movie_titles: Vec<String> = movies
            .iter()
            .map(
                |m| {
                    format!(
                        "{} ({})",
                        m.name, m.release_year
                    )
                },
            )
            .collect();
        info!(
            "filter_by_genre sending {} movies to AI: {:?}",
            movie_titles.len(),
            movie_titles
        );
        Ok(
            serde_json::to_string(
                &truncate(
                    movies,
                    params.limit,
                ),
            )?,
        )
    }
}

#[derive(Debug, Clone, Default)]
pub struct FilterByDirectorTool {
    pub api: ApiClient,
}

impl FilterByDirectorTool {
    pub fn new(api: ApiClient) -> Self {
        Self { api }
    }
}

impl Tool for FilterByDirectorTool {
    type Params = FilterByDirectorParams;
    fn name() -> &'static str {
        "filter_by_director"
    }
    fn description() -> &'static str {
        "Returns movies by the given director as a JSON array."
    }

    async fn call(&mut self, params: Self::Params) -> ToolResult<String> {
        info!(
            "filter_by_director: {}",
            params.director_name
        );
        let query = StructuredQuery {
            directors: vec![params.director_name],
            ..Default::default()
        };
        let movies = self
            .api
            .structured_search(&query)
            .await
            .map_err(
                |e| {
                    error!(
                        "filter_by_director failed: {}",
                        e
                    );
                    e
                },
            )?;
        let movie_titles: Vec<String> = movies
            .iter()
            .map(
                |m| {
                    format!(
                        "{} ({})",
                        m.name, m.release_year
                    )
                },
            )
            .collect();
        info!(
            "filter_by_director sending {} movies to AI: {:?}",
            movie_titles.len(),
            movie_titles
        );
        Ok(
            serde_json::to_string(
                &truncate(
                    movies,
                    params.limit,
                ),
            )?,
        )
    }
}

#[derive(Debug, Clone, Default)]
pub struct GetMovieDetailsTool {
    pub api: ApiClient,
}

impl GetMovieDetailsTool {
    pub fn new(api: ApiClient) -> Self {
        Self { api }
    }
}

impl Tool for GetMovieDetailsTool {
    type Params = GetMovieDetailsParams;
    fn name() -> &'static str {
        "get_movie_details"
    }
    fn description() -> &'static str {
        "Returns full details for movies whose title matches the input as a JSON array (up to 5)."
    }

    async fn call(&mut self, params: Self::Params) -> ToolResult<String> {
        info!(
            "get_movie_details: {}",
            params.title
        );
        let query = StructuredQuery {
            title_keywords: vec![params.title],
            ..Default::default()
        };
        let movies = self
            .api
            .structured_search(&query)
            .await
            .map_err(
                |e| {
                    error!(
                        "get_movie_details failed: {}",
                        e
                    );
                    e
                },
            )?;
        let movie_titles: Vec<String> = movies
            .iter()
            .map(
                |m| {
                    format!(
                        "{} ({})",
                        m.name, m.release_year
                    )
                },
            )
            .collect();
        info!(
            "get_movie_details sending {} movies to AI: {:?}",
            movie_titles.len(),
            movie_titles
        );
        Ok(
            serde_json::to_string(
                &truncate(
                    movies, 5,
                ),
            )?,
        )
    }
}
