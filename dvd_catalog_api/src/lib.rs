use axum::{extract::Path, Json};
use axum_macros::debug_handler;
use database::FullMovie;
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
