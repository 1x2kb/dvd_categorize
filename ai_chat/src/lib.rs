use std::future::Future;

use ollama_rs::{generation::completion::request::GenerationRequest, Ollama};

pub trait GenerateMessage {
    fn generate_message(prompt: String) -> impl Future<Output = String>;
}

pub trait Chat {
    fn chat(prompt: String) -> impl Future<Output = String>;
}

pub struct OllamaClient {
    pub host: String,
    pub port: String,
}

pub async fn bot_message(question: String) -> Result<String, Box<dyn std::error::Error>> {
    // Initialize Ollama (default connects to localhost:11434)
    let ollama = Ollama::default();

    // Set up the generation request
    let model = "falcon3".to_string();
    let request = GenerationRequest::new(
        model, question,
    );

    // Generate a response
    let response = ollama
        .generate(request)
        .await
        .map(|response| response.response)?;

    Ok(response)
}

#[cfg(test)]
mod tests {
    
}
