use crate::components::movie_grid::MovieGrid;
use dioxus::prelude::*;
use models::{FullMovie, SearchRequest};
use serde::{Deserialize, Serialize};
use std::rc::Rc;

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct AiAction {
    pub uuid: String,
    pub action: String,
    pub model: Option<String>,
}

async fn send_search_request(query: String) -> Result<Vec<FullMovie>, reqwest::Error> {
    let window = web_sys::window().unwrap();
    let location = window.location();
    let hostname = location
        .hostname()
        .unwrap_or_else(|_| "127.0.0.1".to_string());

    // TODO: This is incorrect! Front-end does not have access to environment variables
    let server_port = std::env::var("server_port").unwrap_or("3000".to_string());

    let search_request = SearchRequest { query };

    let client = reqwest::Client::new();
    let response = client
        .post(format!("http://{hostname}:{server_port}/ai/dvd-match"))
        .json(&search_request)
        .send()
        .await?
        .json()
        .await?;

    Ok(response)
}

#[component]
pub fn AiLiveResults() -> Element {
    let mut movies: Signal<Vec<Rc<FullMovie>>> = use_signal(std::vec::Vec::new);
    let mut input_value = use_signal(String::new);
    let mut is_loading = use_signal(|| false);

    rsx! {
        div {
            class: "ai-live-results-container",

            // Input and button section
            div {
                class: "search-section",
                style: "margin-bottom: 20px; padding: 20px; background: #1f2937; border-radius: 8px;",

                div {
                    class: "search-input-group",
                    style: "display: flex; gap: 10px; align-items: center;",

                    input {
                        r#type: "text",
                        placeholder: "Enter your search query...",
                        value: "{input_value}",
                        oninput: move |evt| input_value.set(evt.value()),
                        style: "flex: 1; padding: 12px; border: 1px solid #374151; border-radius: 6px; background: #111827; color: white; font-size: 14px;",
                        onkeypress: move |evt| {
                            if evt.key() == Key::Enter {
                                spawn(async move {
                                    if is_loading() {
                                        return;
                                    }

                                    is_loading.set(true);
                                    movies.set(vec![]);

                                    let result = send_search_request(input_value()).await;
                                    match result {
                                        Ok(movie_list) => {
                                            movies.set(movie_list.into_iter().map(Rc::new).collect());
                                        }
                                        Err(err) => {
                                            log::error!("Failed to send search request {:#?}", err);
                                            movies.set(vec![]);
                                        }
                                    }
                                    is_loading.set(false);
                                });
                            }
                        }
                    }

                    button {
                        onclick: move |_| {
                            spawn(async move {
                                if is_loading() {
                                    return;
                                }

                                is_loading.set(true);
                                movies.set(vec![]);

                                let result = send_search_request(input_value()).await;
                                match result {
                                    Ok(movie_list) => {
                                        movies.set(movie_list.into_iter().map(Rc::new).collect());
                                    }
                                    Err(err) => {
                                        log::error!("Failed to send search request {:#?}", err);
                                        movies.set(vec![]);
                                    }
                                }
                                is_loading.set(false);
                            });
                        },
                        disabled: is_loading(),
                        style: "padding: 12px 24px; background: linear-gradient(135deg, #0891b2, #0e7490); color: white; border: none; border-radius: 6px; cursor: pointer; font-weight: 600; min-width: 100px;",

                        if is_loading() {
                            "Searching..."
                        } else {
                            "Search"
                        }
                    }
                }
            }

            // Movie grid section
            div {
                class: "movie-grid movie-grid-cols-3",
                for movie in movies().iter() {
                    MovieGrid { movie: Rc::clone(movie) }
                }
            }
        }
    }
}
