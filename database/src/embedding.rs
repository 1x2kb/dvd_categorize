use async_trait::async_trait;

use crate::traits::*;

pub struct OllamaEmbeddingProvider;

#[async_trait]
impl GenerateEmbedding for OllamaEmbeddingProvider {
    async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
        Ok(ai_chat::get_embedding(text).await?)
    }
}

#[async_trait]
impl GenerateEmbeddings for OllamaEmbeddingProvider {
    async fn generate_embeddings(
        &self,
        texts: Vec<String>,
    ) -> Result<Vec<Vec<f32>>, Box<dyn std::error::Error>> {
        Ok(ai_chat::get_embeddings(texts, ai_chat::EMBEDDING_MODEL).await?)
    }
}
