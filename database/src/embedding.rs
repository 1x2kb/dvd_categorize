use async_trait::async_trait;

use crate::traits::*;

pub struct OllamaEmbeddingProvider;

#[async_trait]
impl GenerateEmbedding for OllamaEmbeddingProvider {
    async fn generate_embedding(&self, _text: &str) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
        // TODO: Fix circular dependency - ai_chat should call database, not reverse
        unimplemented!("Use ai_chat::get_embedding directly instead")
    }
}

#[async_trait]
impl GenerateEmbeddings for OllamaEmbeddingProvider {
    async fn generate_embeddings(
        &self,
        _texts: Vec<String>,
    ) -> Result<Vec<Vec<f32>>, Box<dyn std::error::Error>> {
        // TODO: Fix circular dependency - ai_chat should call database, not reverse
        unimplemented!("Use ai_chat::get_embeddings directly instead")
    }
}
