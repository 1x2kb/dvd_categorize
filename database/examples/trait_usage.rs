#[cfg(feature = "postgres")]
use database::{
    get_database_connection, GetMovieById, GetRecentMovies, MockEmbeddingProvider,
    MockMovieRepository, PostgresMovieRepository, SearchMoviesStructured, StructuredQuery,
};
use models::FullMovie;

async fn search_recent_movies<R: GetRecentMovies>(
    repo: &mut R,
    limit: i64,
) -> Result<Vec<FullMovie>, Box<dyn std::error::Error>> {
    let movies = repo
        .get_recent(limit)
        .await?;
    println!(
        "Found {} recent movies",
        movies.len()
    );
    Ok(movies)
}

async fn example_with_postgres() -> Result<(), Box<dyn std::error::Error>> {
    let conn = get_database_connection().await?;
    let mut repo = PostgresMovieRepository::new(conn);

    let movies = search_recent_movies(
        &mut repo, 5,
    )
    .await?;
    println!(
        "PostgreSQL returned {} movies",
        movies.len()
    );

    Ok(())
}

async fn example_with_mock() -> Result<(), Box<dyn std::error::Error>> {
    let test_movies = vec![FullMovie {
        id: 1,
        key_hash: FullMovie::generate_key_hash("Test Movie"),
        name: "Test Movie".to_string(),
        director: None,
        description: Some("A test movie".to_string()),
        actors: vec![],
        genres: vec!["Action".to_string()],
        embedding: None,
        added_on: None,
        location: None,
        release_year: 2024,
    }];

    let mut repo = MockMovieRepository::with_movies(test_movies);

    let movies = search_recent_movies(
        &mut repo, 5,
    )
    .await?;
    println!(
        "Mock returned {} movies",
        movies.len()
    );

    let movie = repo
        .get_by_id(1)
        .await?;
    println!(
        "Retrieved movie: {}",
        movie.name
    );

    let query = StructuredQuery {
        actors: vec![],
        directors: vec![],
        genres: vec!["Action".to_string()],
        title_keywords: vec![],
        description_keywords: vec![],
    };

    let results = repo
        .search_structured(&query)
        .await?;
    println!(
        "Structured search found {} movies",
        results.len()
    );

    Ok(())
}

async fn example_embedding_mock() -> Result<(), Box<dyn std::error::Error>> {
    use database::GenerateEmbedding;

    let provider = MockEmbeddingProvider::new().with_embedding(
        "action movie".to_string(),
        vec![1.0, 2.0, 3.0],
    );

    let embedding = provider
        .generate_embedding("action movie")
        .await?;
    println!(
        "Generated embedding with {} dimensions",
        embedding.len()
    );
    println!(
        "Embedding values: {:?}",
        embedding
    );

    let default_embedding = provider
        .generate_embedding("unknown text")
        .await?;
    println!(
        "Default embedding has {} dimensions",
        default_embedding.len()
    );

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Example: Using Mocks ===");
    example_with_mock().await?;

    println!("\n=== Example: Mock Embedding Provider ===");
    example_embedding_mock().await?;

    println!("\n=== Example: Using PostgreSQL (requires DATABASE_URL) ===");
    match example_with_postgres().await {
        Ok(_) => println!("PostgreSQL example completed successfully"),
        Err(e) => println!(
            "PostgreSQL example failed (expected if no DB): {}",
            e
        ),
    }

    Ok(())
}
