use models::{dvd_filters::DvdFilters, AiMovieData, QuerySpec, StructuredQuery};
use ollama_rs::generation::parameters::{FormatType, JsonStructure};

/// Generate JSON schema for DvdFilters struct
pub fn dvd_filters_schema() -> FormatType {
    FormatType::StructuredJson(Box::new(JsonStructure::new::<DvdFilters>()))
}

/// Generate JSON schema for StructuredQuery struct
pub fn structured_query_schema() -> FormatType {
    FormatType::StructuredJson(Box::new(JsonStructure::new::<StructuredQuery>()))
}

/// Generate JSON schema for actor array (Vec<String>)
/// Used by actor_reorder crate to enforce valid JSON array responses
pub fn actor_array_schema() -> FormatType {
    FormatType::StructuredJson(Box::new(JsonStructure::new::<Vec<String>>()))
}

/// Generate JSON schema for QuerySpec
/// Used by tool calling to let AI generate database queries
pub fn query_spec_schema() -> FormatType {
    FormatType::StructuredJson(Box::new(JsonStructure::new::<QuerySpec>()))
}

/// Generate JSON schema for AiMovieData
/// Used for structured movie generation from AI
pub fn ai_movie_data_schema() -> FormatType {
    FormatType::StructuredJson(Box::new(JsonStructure::new::<AiMovieData>()))
}

/// Generate JSON schema for Vec<AiMovieData> (array of movie data)
/// Used for batch movie generation from AI
pub fn ai_movie_data_array_schema() -> FormatType {
    FormatType::StructuredJson(Box::new(JsonStructure::new::<Vec<AiMovieData>>()))
}
