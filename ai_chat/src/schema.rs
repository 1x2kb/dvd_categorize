use models::{dvd_filters::DvdFilters, StructuredQuery};
use ollama_rs::generation::parameters::{FormatType, JsonStructure};

/// Generate JSON schema for DvdFilters struct
pub fn dvd_filters_schema() -> FormatType {
    FormatType::StructuredJson(Box::new(JsonStructure::new::<DvdFilters>()))
}

/// Generate JSON schema for StructuredQuery struct
pub fn structured_query_schema() -> FormatType {
    FormatType::StructuredJson(Box::new(JsonStructure::new::<StructuredQuery>()))
}
