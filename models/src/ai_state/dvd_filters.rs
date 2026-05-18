use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, Default, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct DvdFilters {
    pub genres: Option<Vec<String>>,

    pub actors: Option<Vec<String>>,
    pub directors: Option<Vec<String>>,

    #[serde(rename = "movie_name")]
    pub movie_names: Option<Vec<String>>,

    #[serde(rename = "plot_keywords")]
    pub plot_terms: Option<Vec<String>>,
}
