use dioxus::prelude::*;

use crate::{app_data::AppData, components::movie_card::MovieCard};

#[component]
pub fn DisplayMovie() -> Element {
    let app_data = consume_context::<Signal<AppData>>();

    rsx! {
        div {
            {app_data.read().movies.read().as_ref().map(|movies| rsx! {
                div {
                    for movie in movies.iter() {
                        MovieCard {
                            key: "{movie.id}",  // Use movie ID instead of index
                            movie: movie.clone()
                        }
                    }
                }
            })}
        }
    }
}
