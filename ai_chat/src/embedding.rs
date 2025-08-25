use ollama_rs::{
    generation::embeddings::request::{EmbeddingsInput, GenerateEmbeddingsRequest},
    Ollama,
};
use std::error::Error;
use log::debug;

/// Get embeddings for a collection of text items using Ollama in bulk
/// 
/// # Arguments
/// * `texts` - Any iterable collection of string-like items
/// * `model` - The embedding model to use (e.g., "nomic-embed-text")
pub async fn get_embeddings<I, T>(
    texts: I,
    model: &str,
) -> Result<Vec<Vec<f32>>, Box<dyn Error>>
where
    I: IntoIterator<Item = T>,
    T: Into<String>,
{
    // Use ollama service name for Docker container communication
    let ollama_host = std::env::var("OLLAMA_HOST").unwrap_or_else(|_| "ollama".to_string());
    let ollama_port = std::env::var("OLLAMA_PORT").unwrap_or_else(|_| "11434".to_string());

    let ollama_url = format!("http://{}:{}", &ollama_host, &ollama_port);
    debug!("Connecting to ollama @ {}", &ollama_url);

    let ollama = Ollama::from_url(ollama_url.parse().unwrap());
    
    // Convert texts efficiently using Into<String>
    let string_texts: Vec<String> = texts.into_iter().map(|t| t.into()).collect();
    
    // Create a single bulk request with all texts
    let request = GenerateEmbeddingsRequest::new(
        model.to_string(), 
        EmbeddingsInput::Multiple(string_texts)
    );
    
    let response = ollama.generate_embeddings(request).await?;
    
    // Convert the embeddings from Vec<Vec<f32>> to Vec<Vec<f64>>
    let embeddings: Vec<Vec<f32>> = response.embeddings;
    
    Ok(embeddings)
}
