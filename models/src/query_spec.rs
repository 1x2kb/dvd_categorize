use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Query specification that AI generates to search movie library
/// Combines vector search with structured filters
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct QuerySpec {
    /// Vector search component - semantic similarity
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vector_search: Option<VectorSearchSpec>,

    /// Structured filters - exact matches
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<MovieFilters>,

    /// How to combine vector + filter results
    pub combine_strategy: CombineStrategy,

    /// Maximum results to return
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    10
}

/// Vector search parameters
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct VectorSearchSpec {
    /// Text to embed and search for similar movies
    pub search_text: String,

    /// Minimum similarity score (0.0 to 1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_similarity: Option<f32>,

    /// Max results from vector search
    #[serde(default = "default_vector_limit")]
    pub limit: usize,
}

fn default_vector_limit() -> usize {
    20
}

/// Movie filter criteria
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MovieFilters {
    /// Actor names to filter by (OR logic)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actors: Vec<String>,

    /// Director names to filter by (OR logic)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub directors: Vec<String>,

    /// Genres to filter by (OR logic)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub genres: Vec<String>,

    /// Title keywords (AND logic)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub title_keywords: Vec<String>,

    /// Year range filter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year_range: Option<YearRange>,
}

/// Year range for filtering
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct YearRange {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<i32>,
}

/// How to combine vector search and filter results
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CombineStrategy {
    /// Only vector search results
    VectorOnly,
    /// Only filter results
    FilterOnly,
    /// Intersection of both (AND)
    Intersect,
    /// Union of both (OR)
    Union,
    /// Vector results filtered by criteria
    VectorThenFilter,
}
