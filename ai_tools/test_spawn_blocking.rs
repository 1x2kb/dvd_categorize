// Test if spawn_blocking approach compiles with Tool trait

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ollama_rs::generation::tools::Tool;

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct TestParams {
    pub query: String,
}

#[derive(Default)]
pub struct TestTool;

impl Tool for TestTool {
    type Params = TestParams;

    fn name() -> &'static str {
        "test"
    }

    fn description() -> &'static str {
        "test"
    }

    async fn call(&mut self, params: Self::Params) -> ollama_rs::generation::tools::Result<String> {
        // This is the pattern - does it satisfy Sync requirement?
        let result = tokio::task::spawn_blocking(move || {
            tokio::runtime::Handle::current().block_on(async move {
                // Simulate diesel-async call
                let pool = database::get_connection_pool().await?;
                let repo = database::postgres::PostgresMovieRepository::new(pool);
                
                // This would be non-Sync diesel future
                use database::schema::*;
                use diesel::prelude::*;
                use diesel_async::RunQueryDsl;
                
                let mut conn = pool.get().await.map_err(|e| {
                    Box::new(e) as Box<dyn std::error::Error + Send + Sync>
                })?;
                
                let _: Vec<i32> = movie::table
                    .select(movie::id)
                    .limit(1)
                    .load(&mut conn)
                    .await?;
                
                Ok::<_, Box<dyn std::error::Error + Send + Sync>>("test".to_string())
            })
        }).await.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)??;
        
        Ok(result)
    }
}
