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

    let mut base_query = movie::table
        .left_join(director::table.on(movie::director_id.eq(director::id.nullable())))
        .into_boxed();

    let has_criteria = !structured_query.actors.is_empty()
        || !structured_query.directors.is_empty()
        || !structured_query.genres.is_empty()
        || !structured_query.title_keywords.is_empty()
        || !structured_query.description_keywords.is_empty();

    if !has_criteria {
        info!("No search criteria provided, returning all movies");
    }

    if !structured_query.directors.is_empty() {
        info!("Filtering by directors: {:?}", structured_query.directors);
        for director_name in &structured_query.directors {
            let pattern = format!("%{}%", director_name.to_lowercase());
            base_query = base_query.filter(director::name.ilike(pattern));
        }
    }

    if !structured_query.title_keywords.is_empty() {
        info!("Filtering by title keywords: {:?}", structured_query.title_keywords);
        for keyword in &structured_query.title_keywords {
            let pattern = format!("%{}%", keyword.to_lowercase());
            base_query = base_query.filter(movie::name.ilike(pattern));
        }
    }

    if !structured_query.description_keywords.is_empty() {
        info!("Filtering by description keywords: {:?}", structured_query.description_keywords);
        for keyword in &structured_query.description_keywords {
            let pattern = format!("%{}%", keyword.to_lowercase());
            base_query = base_query.filter(movie::description.ilike(pattern));
        }
    }

    let movies: Vec<(models::Movie, Option<Director>)> =
        base_query.load::<(models::Movie, Option<Director>)>(connection).await?;

    info!("Found {} movies before actor/genre filtering", movies.len());

    let mut movie_ids: Vec<i32> = movies.iter().map(|(m, _)| m.id).collect();

    if !structured_query.actors.is_empty() {
        info!("Filtering by actors: {:?}", structured_query.actors);
        
        let mut matching_movie_ids = Vec::new();
        for actor_name in &structured_query.actors {
            let pattern = format!("%{}%", actor_name.to_lowercase());
            let ids: Vec<i32> = movie_actor::table
                .inner_join(actor::table)
                .filter(actor::name.ilike(pattern))
                .select(movie_actor::movie_id)
                .distinct()
                .load::<i32>(connection).await?;
            matching_movie_ids.extend(ids);
        }

        movie_ids.retain(|id| matching_movie_ids.contains(id));
        info!("After actor filtering: {} movies", movie_ids.len());
    }

    if !structured_query.genres.is_empty() {
        info!("Filtering by genres: {:?}", structured_query.genres);
        
        let mut matching_movie_ids = Vec::new();
        for genre_name in &structured_query.genres {
            let pattern = format!("%{}%", genre_name.to_lowercase());
            let ids: Vec<i32> = movie_genre::table
                .filter(movie_genre::genre.ilike(pattern))
                .select(movie_genre::movie_id)
                .distinct()
                .load::<i32>(connection).await?;
            matching_movie_ids.extend(ids);
        }

        movie_ids.retain(|id| matching_movie_ids.contains(id));
        info!("After genre filtering: {} movies", movie_ids.len());
    }

    let final_movies: Vec<(models::Movie, Option<Director>)> = movies
        .into_iter()
        .filter(|(m, _)| movie_ids.contains(&m.id))
        .collect();

    info!("Building full movie objects for {} results", final_movies.len());

    let mut full_movies = Vec::new();
    for (movie, director) in final_movies {
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
