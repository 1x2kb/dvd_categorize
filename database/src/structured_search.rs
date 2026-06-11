use diesel::expression::BoxableExpression;
use diesel::prelude::*;
use diesel::BoolExpressionMethods;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use log::info;
use models::{schema, Actor, Director, FullMovie, StructuredQuery};

fn to_ilike_patterns(names: &[String]) -> Vec<String> {
    names
        .iter()
        .map(
            |n| {
                format!(
                    "%{}%",
                    n.to_lowercase()
                )
            },
        )
        .collect()
}

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
    use models::Movie;
    use schema::{actor, director, movie, movie_actor, movie_genre};

    info!(
        "Executing structured search with criteria: {:?}",
        structured_query
    );

    // Step 1: Build the main query with director and title filters (AND logic)
    let mut base_query = movie::table
        .left_join(director::table.on(movie::director_id.eq(director::id.nullable())))
        .distinct()
        .into_boxed();

    // Apply director filters (AND logic - all must match)
    if !structured_query
        .directors
        .is_empty()
    {
        info!(
            "Filtering by directors: {:?}",
            structured_query.directors
        );
        for director_name in &structured_query.directors {
            let pattern = format!(
                "%{}%",
                director_name.to_lowercase()
            );
            base_query = base_query.filter(director::name.ilike(pattern));
        }
    }

    // Apply title keyword filters (AND logic - all must match)
    if !structured_query
        .title_keywords
        .is_empty()
    {
        info!(
            "Filtering by title keywords: {:?}",
            structured_query.title_keywords
        );
        for keyword in &structured_query.title_keywords {
            let pattern = format!(
                "%{}%",
                keyword.to_lowercase()
            );
            base_query = base_query.filter(movie::name.ilike(pattern));
        }
    }

    // Apply description keyword filters (OR logic - any must match)
    if !structured_query
        .description_keywords
        .is_empty()
    {
        info!(
            "Filtering by description keywords (OR): {:?}",
            structured_query.description_keywords
        );

        use diesel::sql_types::Nullable;

        let patterns = to_ilike_patterns(&structured_query.description_keywords);
        let or_condition: Option<
            Box<
                dyn BoxableExpression<
                    _,
                    diesel::pg::Pg,
                    SqlType = Nullable<diesel::sql_types::Bool>,
                >,
            >,
        > = patterns
            .into_iter()
            .fold(
                None,
                |acc, pattern| {
                    let expr = movie::description.ilike(pattern);
                    Some(
                        match acc {
                            None => Box::new(expr),
                            Some(prev) => Box::new(prev.or(expr)),
                        },
                    )
                },
            );

        if let Some(condition) = or_condition {
            base_query = base_query.filter(condition);
        }
    }

    // Apply actor filters (OR logic - match ANY actor) using Diesel's exists()
    if !structured_query
        .actors
        .is_empty()
    {
        info!(
            "Filtering by actors (OR): {:?}",
            structured_query.actors
        );

        use diesel::dsl::exists;

        let patterns = to_ilike_patterns(&structured_query.actors);
        let actor_or_condition: Option<
            Box<dyn BoxableExpression<_, diesel::pg::Pg, SqlType = diesel::sql_types::Bool>>,
        > = patterns
            .into_iter()
            .fold(
                None,
                |acc, pattern| {
                    let expr = actor::name.ilike(pattern);
                    Some(
                        match acc {
                            None => Box::new(expr),
                            Some(prev) => Box::new(prev.or(expr)),
                        },
                    )
                },
            );

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
    if !structured_query
        .genres
        .is_empty()
    {
        info!(
            "Filtering by genres (OR): {:?}",
            structured_query.genres
        );

        use diesel::dsl::exists;

        let patterns = to_ilike_patterns(&structured_query.genres);
        let genre_or_condition: Option<
            Box<dyn BoxableExpression<_, diesel::pg::Pg, SqlType = diesel::sql_types::Bool>>,
        > = patterns
            .into_iter()
            .fold(
                None,
                |acc, pattern| {
                    let expr = movie_genre::genre.ilike(pattern);
                    Some(
                        match acc {
                            None => Box::new(expr),
                            Some(prev) => Box::new(prev.or(expr)),
                        },
                    )
                },
            );

        if let Some(condition) = genre_or_condition {
            let genre_subquery = movie_genre::table
                .filter(movie_genre::movie_id.eq(movie::id))
                .filter(condition)
                .select(movie_genre::movie_id);

            base_query = base_query.filter(exists(genre_subquery));
        }
    }

    // Step 5: Execute the final query
    let movies: Vec<(
        models::Movie,
        Option<Director>,
    )> = base_query
        .load::<(
            models::Movie,
            Option<Director>,
        )>(connection)
        .await?;

    info!(
        "Found {} movies matching all criteria",
        movies.len()
    );

    // Step 6: Hydrate with actors and genres — 2 bulk queries instead of 2N
    let movie_refs: Vec<&Movie> = movies
        .iter()
        .map(|(m, _)| m)
        .collect();
    let movie_ids: Vec<i32> = movie_refs
        .iter()
        .map(|m| m.id)
        .collect();

    let raw_actors = crate::load_actors_for_movies(
        &movie_ids, connection,
    )
    .await
    .map_err(
        |e| match e {
            crate::DatabaseError::DieselError(de) => de,
            _ => diesel::result::Error::NotFound,
        },
    )?;
    let raw_genres = crate::load_genres_for_movies(
        &movie_ids, connection,
    )
    .await
    .map_err(
        |e| match e {
            crate::DatabaseError::DieselError(de) => de,
            _ => diesel::result::Error::NotFound,
        },
    )?;

    let actors_per_movie: Vec<Vec<Actor>> = raw_actors
        .grouped_by(&movie_refs)
        .into_iter()
        .map(
            |group| {
                group
                    .into_iter()
                    .map(|(_, actor)| actor)
                    .collect()
            },
        )
        .collect();

    let genres_per_movie: Vec<Vec<String>> = raw_genres
        .grouped_by(&movie_refs)
        .into_iter()
        .map(
            |group| {
                group
                    .into_iter()
                    .map(|mg| mg.genre)
                    .collect()
            },
        )
        .collect();

    let full_movies: Vec<FullMovie> = movies
        .into_iter()
        .zip(actors_per_movie)
        .zip(genres_per_movie)
        .map(
            |(((movie, director), actors), genres)| {
                FullMovie::from((
                    movie, director, actors, genres,
                ))
            },
        )
        .collect();

    info!(
        "Returning {} full movies",
        full_movies.len()
    );
    Ok(full_movies)
}
