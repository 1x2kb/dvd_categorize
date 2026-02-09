use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use log::info;
use models::{schema, Actor, Director, FullMovie, StructuredQuery};

/// Searches for movies using structured query criteria with dynamic Diesel query building
///
/// # Arguments
/// * `structured_query` - The parsed structured query containing actors, directors, genres, etc.
/// * `connection` - Database connection
///
/// # Returns
/// A vector of FullMovie objects matching the criteria
pub async fn search_movies_structured(
    structured_query: &StructuredQuery,
    connection: &mut AsyncPgConnection,
) -> Result<Vec<FullMovie>, diesel::result::Error> {
    use schema::{actor, director, movie, movie_actor, movie_genre};

    info!("Executing structured search with criteria: {:?}", structured_query);

    // Step 1: Build the main query with director and title filters (AND logic)
    let mut base_query = movie::table
        .left_join(director::table.on(movie::director_id.eq(director::id.nullable())))
        .distinct()
        .into_boxed();

    // Apply director filters (AND logic - all must match)
    if !structured_query.directors.is_empty() {
        info!("Filtering by directors: {:?}", structured_query.directors);
        for director_name in &structured_query.directors {
            let pattern = format!("%{}%", director_name.to_lowercase());
            base_query = base_query.filter(director::name.ilike(pattern));
        }
    }

    // Apply title keyword filters (AND logic - all must match)
    if !structured_query.title_keywords.is_empty() {
        info!("Filtering by title keywords: {:?}", structured_query.title_keywords);
        for keyword in &structured_query.title_keywords {
            let pattern = format!("%{}%", keyword.to_lowercase());
            base_query = base_query.filter(movie::name.ilike(pattern));
        }
    }

    // Apply description keyword filters (OR logic - any must match)
    if !structured_query.description_keywords.is_empty() {
        info!("Filtering by description keywords (OR): {:?}", structured_query.description_keywords);
        
        use diesel::BoolExpressionMethods;
        use diesel::expression::BoxableExpression;
        use diesel::sql_types::Nullable;
        
        // Note: description is nullable, so the expression type is Nullable<Bool>
        let mut or_condition: Option<Box<dyn BoxableExpression<_, diesel::pg::Pg, SqlType = Nullable<diesel::sql_types::Bool>>>> = None;
        
        for keyword in &structured_query.description_keywords {
            let pattern = format!("%{}%", keyword.to_lowercase());
            let expr = movie::description.ilike(pattern);
            
            or_condition = Some(match or_condition {
                None => Box::new(expr),
                Some(prev) => Box::new(prev.or(expr)),
            });
        }
        
        if let Some(condition) = or_condition {
            base_query = base_query.filter(condition);
        }
    }

    // Apply actor filters (OR logic - match ANY actor) using Diesel's exists()
    if !structured_query.actors.is_empty() {
        info!("Filtering by actors (OR): {:?}", structured_query.actors);
        
        use diesel::BoolExpressionMethods;
        use diesel::expression::BoxableExpression;
        use diesel::dsl::exists;
        
        // Build OR condition for actor names
        let mut actor_or_condition: Option<Box<dyn BoxableExpression<_, diesel::pg::Pg, SqlType = diesel::sql_types::Bool>>> = None;
        
        for actor_name in &structured_query.actors {
            let pattern = format!("%{}%", actor_name.to_lowercase());
            let expr = actor::name.ilike(pattern);
            
            actor_or_condition = Some(match actor_or_condition {
                None => Box::new(expr),
                Some(prev) => Box::new(prev.or(expr)),
            });
        }
        
        if let Some(condition) = actor_or_condition {
            let actor_subquery = movie_actor::table
                .inner_join(actor::table)
                .filter(movie_actor::movie_id.eq(movie::id))
                .filter(condition)
                .select(movie_actor::movie_id);
            
            base_query = base_query.filter(exists(actor_subquery));
        }
    }

    // Apply genre filters (OR logic - match ANY genre) using Diesel's exists()
    if !structured_query.genres.is_empty() {
        info!("Filtering by genres (OR): {:?}", structured_query.genres);
        
        use diesel::BoolExpressionMethods;
        use diesel::expression::BoxableExpression;
        use diesel::dsl::exists;
        
        // Build OR condition for genres
        let mut genre_or_condition: Option<Box<dyn BoxableExpression<_, diesel::pg::Pg, SqlType = diesel::sql_types::Bool>>> = None;
        
        for genre_name in &structured_query.genres {
            let pattern = format!("%{}%", genre_name.to_lowercase());
            let expr = movie_genre::genre.ilike(pattern);
            
            genre_or_condition = Some(match genre_or_condition {
                None => Box::new(expr),
                Some(prev) => Box::new(prev.or(expr)),
            });
        }
        
        if let Some(condition) = genre_or_condition {
            let genre_subquery = movie_genre::table
                .filter(movie_genre::movie_id.eq(movie::id))
                .filter(condition)
                .select(movie_genre::movie_id);
            
            base_query = base_query.filter(exists(genre_subquery));
        }
    }

    // Step 5: Execute the final query
    let movies: Vec<(models::Movie, Option<Director>)> =
        base_query.load::<(models::Movie, Option<Director>)>(connection).await?;

    info!("Found {} movies matching all criteria", movies.len());

    // Step 6: Hydrate with actors and genres
    let mut full_movies = Vec::new();
    for (movie, director) in movies {
        let actors: Vec<Actor> = movie_actor::table
            .inner_join(actor::table)
            .filter(movie_actor::movie_id.eq(movie.id))
            .select(Actor::as_select())
            .load::<Actor>(connection).await?;

        let genres: Vec<String> = movie_genre::table
            .filter(movie_genre::movie_id.eq(movie.id))
            .select(movie_genre::genre)
            .load::<String>(connection).await?;

        full_movies.push(FullMovie::from((movie, director, actors, genres)));
    }

    info!("Returning {} full movies", full_movies.len());
    Ok(full_movies)
}
