use dioxus::prelude::*;
use std::sync::Arc;

use crate::{
    app_data::AppData,
    components::movie_grid::MovieGrid,
};

#[component]
pub fn MoviesList() -> Element {
    let app_data = consume_context::<Signal<AppData>>();
    let movies = app_data.read().movies;

    rsx! {
        if let Some(movie_list) = movies.read().as_ref() {
            MovieGrid { movies: Arc::clone(movie_list) }
        }
    }
}
