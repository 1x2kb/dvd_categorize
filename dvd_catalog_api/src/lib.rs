use std::sync::Arc;

use axum::{extract::Path, response, Json};
use axum_macros::debug_handler;
use database::{dvd_filters::DvdFilters, movie_genre::movie_id, question::AiAction, FullMovie};
use log::error;
use tracing::instrument;

#[instrument]
pub async fn hello_world() -> &'static str {
    "Hello from DVD_CATALOG_API!"
}

#[instrument]
#[debug_handler]
pub async fn get_dvds() -> Json<Option<Vec<FullMovie>>> {
    Json(
        database::get_movies()
            .await
            .ok(),
    )
}

#[instrument]
#[debug_handler]
pub async fn get_dvd(Path(id): Path<i32>) -> Json<Option<FullMovie>> {
    Json(
        database::get_movie(id)
            .await
            .ok(),
    )
}

#[instrument]
#[debug_handler]
pub async fn insert_dvd(Json(dvd): Json<FullMovie>) -> Json<Option<FullMovie>> {
    Json(
        database::insert_full_movie(dvd)
            .await
            .ok(),
    )
}

#[instrument]
#[debug_handler]
pub async fn chat(Json(action): Json<AiAction>) -> Json<AiAction> {
    let dvds = database::get_movies()
        .await
        .unwrap_or_else(|_| Vec::new());

    let (uuid, question, model) = (
        action.uuid,
        action.action,
        action.model,
    );

    let movie_ids = match ai_chat::find_related_keys(question.as_str()).await {
        Ok(dvd_filters) => database::run_dvd_filters(dvd_filters)
            .await
            .ok(),
        Err(e) => {
            error!(
                "{:#?}",
                e
            );
            None
        }
    };

    let full_movies = if let Some(movie_ids) = movie_ids {
        database::get_movies_by_ids(movie_ids)
            .await
            .unwrap_or_else(|_| dvds)
    } else {
        dvds
    }; // For now fall back to all dvds

    let result = ai_chat::ai_message(
        AiAction {
            uuid: uuid.to_string(),
            action: question,
            model: model.clone(),
        },
        Arc::new(full_movies),
    )
    .await;

    let response = match result {
        Ok(response) => response,
        Err(e) => {
            error!(
                "{:#?}",
                e
            );
            "There was an error that made communication with the AI impossible.".to_string()
        }
    };

    Json(
        AiAction {
            uuid,
            action: response,
            model,
        },
    )
}
