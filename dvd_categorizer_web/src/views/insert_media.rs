use std::rc::Rc;

use dioxus::prelude::*;
use log::error;
use models::{CsvInput, FullMovie};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::components::{
    display_movie::DisplayMovie, movie_card::MovieCard, movie_grid::MovieGrid,
};

#[component]
pub fn InsertMedia() -> Element {
    let mut csv_data = use_signal(|| "".to_string());
    let mut dvd_data: Signal<Option<Vec<Rc<FullMovie>>>> = use_signal(|| None);
    let mut is_previewing: Signal<bool> = use_signal(|| false);

    rsx! {
        div {
            h3 { "CSV Headers" }
            span { "Title," }
            span { "Description," }
            span { "Actors," }
            span { "Genres," }
            span { "Director" }
        }

        div {
            class: "csv-button-container",
            button {
                class: "good",
                onclick: move |_| {
                    spawn(async move {
                        // Use the current page's hostname instead of hardcoded localhost
                        let window = web_sys::window().unwrap();
                        let location = window.location();
                        let hostname = location.hostname().unwrap_or_else(|_| "127.0.0.1".to_string());
                        let server_port = std::env::var("server_port").unwrap_or("3000".to_string());

                        let client = reqwest::Client::new();
                        let result = client.post(format!("http://{hostname}:{server_port}/csv/preview")).json(&CsvInput {
                            input: csv_data()
                        }).send().await;

                        match result {
                            Ok(response) => {
                                let response: Result<Vec<Rc<FullMovie>>, String> = response.json::<Vec<FullMovie>>()
                                    .await
                                    .map_err(|e| e.to_string())
                                    .map(|movies| movies
                                        .into_iter()
                                        .map(|movie| Rc::new(movie))
                                        .collect()
                                    );

                                match response {
                                    Ok(dvds) => {dvd_data.set(Some(dvds));
                                        is_previewing.set(true);
                                    },
                                    Err(e) => error!("{}", e),
                                }
                            },
                            Err(_) => {}, // Do nothing for now
                        }
                    });
                },
                "Preview"
            }
            if is_previewing() {
                button {
                    class: "bad",
                    onclick: move |_| {
                        dvd_data.set(None);
                        is_previewing.set(false);
                    },
                    "Clear"
                }
                button {
                    class: "good",
                    onclick: move |_| {

                    },
                    "Send"
                }
            }
        }

        if let Some(movies) = dvd_data() {
            div {
                class: "movie-grid movie-grid-cols-3",
                for movie in movies.iter() {
                    MovieGrid { movie: Rc::clone(&movie) }
                }
            }
        }

        div {
            textarea {
                onchange: move |e| {
                    csv_data.set(e.value());
                },
                placeholder: "Csv Data"
             }
        }
    }
}
