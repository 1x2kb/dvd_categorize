use crate::components::movie_grid::MovieGrid;
use dioxus::prelude::*;
use models::{ScoredMovie, SearchRequest};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct AiAction {
    pub uuid: String,
    pub action: String,
    pub model: Option<String>,
}

async fn send_search_request(
    query: String,
    disable_enhancement: bool,
    search_mode: models::SearchMode,
    selected_model: Option<String>,
) -> Result<models::SearchResponse, reqwest::Error> {
    let window = web_sys::window().unwrap();
    let location = window.location();
    let hostname = location
        .hostname()
        .unwrap_or_else(|_| "127.0.0.1".to_string());

    // TODO: This is incorrect! Front-end does not have access to environment variables
    let server_port = std::env::var("server_port").unwrap_or("3000".to_string());

    let search_request = SearchRequest {
        query,
        disable_enhancement,
        search_mode,
        model: selected_model,
    };

    let client = reqwest::Client::new();
    let response: models::SearchResponse = client
        .post(format!("http://{hostname}:{server_port}/ai/dvd-match"))
        .json(&search_request)
        .send()
        .await?
        .json()
        .await?;

    Ok(response)
}

async fn fetch_recent_movies() -> Result<Vec<ScoredMovie>, reqwest::Error> {
    let window = web_sys::window().unwrap();
    let location = window.location();
    let hostname = location
        .hostname()
        .unwrap_or_else(|_| "127.0.0.1".to_string());

    let server_port = std::env::var("server_port").unwrap_or("3000".to_string());

    let client = reqwest::Client::new();
    let response: Vec<ScoredMovie> = client
        .get(format!("http://{hostname}:{server_port}/ai/recent"))
        .send()
        .await?
        .json()
        .await?;

    Ok(response)
}

async fn fetch_available_models() -> Result<models::AvailableModelsResponse, reqwest::Error> {
    let window = web_sys::window().unwrap();
    let location = window.location();
    let hostname = location
        .hostname()
        .unwrap_or_else(|_| "127.0.0.1".to_string());

    let server_port = std::env::var("server_port").unwrap_or("3000".to_string());

    let client = reqwest::Client::new();
    let response: models::AvailableModelsResponse = client
        .get(format!("http://{hostname}:{server_port}/ai/models"))
        .send()
        .await?
        .json()
        .await?;

    Ok(response)
}

#[component]
pub fn AiLiveResults() -> Element {
    let mut movies: Signal<Arc<Vec<ScoredMovie>>> = use_signal(|| Arc::new(Vec::new()));
    let mut input_value = use_signal(String::new);
    let mut is_loading = use_signal(|| false);
    let mut disable_enhancement = use_signal(|| false);
    let mut search_mode = use_signal(|| models::SearchMode::Text);
    let mut enhanced_query = use_signal(String::new);
    let mut original_query = use_signal(String::new);
    let mut selected_model = use_signal(|| None::<String>);
    let mut available_models = use_signal(Vec::<models::AvailableModel>::new);

    // Fetch available models on component mount
    use_effect(
        move || {
            spawn(
                async move {
                    match fetch_available_models().await {
                        Ok(response) => {
                            available_models.set(response.models);
                        }
                        Err(err) => {
                            log::error!(
                                "Failed to fetch available models: {:#?}",
                                err
                            );
                        }
                    }
                },
            );
        },
    );

    rsx! {
        div {
            class: "ai-live-results-container",

            // Browse actions bar at the top
            div {
                class: "browse-actions-bar",

                div {
                    class: "browse-label",
                    "Browse:"
                }

                button {
                    class: "browse-button",
                    onclick: move |_| {
                        spawn(async move {
                            if is_loading() {
                                return;
                            }

                            is_loading.set(true);
                            movies.set(Arc::new(vec![]));
                            enhanced_query.set(String::new());
                            original_query.set(String::new());

                            let result = fetch_recent_movies().await;
                            match result {
                                Ok(recent_movies) => {
                                    movies.set(Arc::new(recent_movies));
                                }
                                Err(err) => {
                                    log::error!("Failed to fetch recent movies {:#?}", err);
                                    movies.set(Arc::new(vec![]));
                                }
                            }
                            is_loading.set(false);
                        });
                    },
                    disabled: is_loading(),

                    if is_loading() {
                        "Loading..."
                    } else {
                        "Recent Movies"
                    }
                }
            }

            // Search section
            div {
                class: "search-section",

                // Search mode tabs
                div {
                    class: "search-mode-tabs",

                    span {
                        class: "mode-label",
                        "Mode:"
                    }

                    button {
                        class: if matches!(search_mode(), models::SearchMode::Text) { "mode-button active" } else { "mode-button" },
                        onclick: move |_| search_mode.set(models::SearchMode::Text),
                        "Text"
                    }

                    button {
                        class: if matches!(search_mode(), models::SearchMode::Both) { "mode-button active" } else { "mode-button" },
                        onclick: move |_| search_mode.set(models::SearchMode::Both),
                        "Both"
                    }

                    button {
                        class: if matches!(search_mode(), models::SearchMode::Vector) { "mode-button active" } else { "mode-button" },
                        onclick: move |_| search_mode.set(models::SearchMode::Vector),
                        "Vector"
                    }

                    button {
                        class: if matches!(search_mode(), models::SearchMode::Structured) { "mode-button active" } else { "mode-button" },
                        onclick: move |_| search_mode.set(models::SearchMode::Structured),
                        "Structured"
                    }

                    // Model selector (hidden for Text mode)
                    if !matches!(search_mode(), models::SearchMode::Text) {
                        div {
                            class: "model-selector-container",

                            span {
                                class: "model-label",
                                "Model:"
                            }

                            select {
                                class: "model-select",
                                value: match selected_model() {
                                    Some(ref model) => model.clone(),
                                    None => "default".to_string(),
                                },
                                onchange: move |evt| {
                                    let value = evt.value();
                                    if value == "default" {
                                        selected_model.set(None);
                                    } else {
                                        selected_model.set(Some(value));
                                    }
                                },

                                option { value: "default", "Default" }
                                for model in available_models().iter() {
                                    option {
                                        value: "{model.name}",
                                        "{model.name}"
                                    }
                                }
                            }
                        }
                    }
                }

                div {
                    class: "search-input-area",
                    div {
                        class: "search-input-group",

                    input {
                        class: "search-input",
                        r#type: "text",
                        placeholder: "Enter your search query...",
                        value: "{input_value}",
                        oninput: move |evt| input_value.set(evt.value()),
                        onkeypress: move |evt| {
                            if evt.key() == Key::Enter {
                                spawn(async move {
                                    if is_loading() {
                                        return;
                                    }

                                    is_loading.set(true);
                                    movies.set(Arc::new(vec![]));

                                    let result = send_search_request(input_value(), disable_enhancement(), search_mode(), selected_model()).await;
                                    match result {
                                        Ok(response) => {
                                            movies.set(Arc::new(response.results));
                                            original_query.set(response.original_query);
                                            enhanced_query.set(response.enhanced_query);
                                        }
                                        Err(err) => {
                                            log::error!("Failed to send search request {:#?}", err);
                                            movies.set(Arc::new(vec![]));
                                        }
                                    }
                                    is_loading.set(false);
                                });
                            }
                        }
                    }

                    button {
                        class: "search-button",
                        onclick: move |_| {
                            spawn(async move {
                                if is_loading() {
                                    return;
                                }

                                is_loading.set(true);
                                movies.set(Arc::new(vec![]));

                                let result = send_search_request(input_value(), disable_enhancement(), search_mode(), selected_model()).await;
                                match result {
                                    Ok(response) => {
                                        movies.set(Arc::new(response.results));
                                        original_query.set(response.original_query);
                                        enhanced_query.set(response.enhanced_query);
                                    }
                                    Err(err) => {
                                        log::error!("Failed to send search request {:#?}", err);
                                        movies.set(Arc::new(vec![]));
                                    }
                                }
                                is_loading.set(false);
                            });
                        },
                        disabled: is_loading(),

                        if is_loading() {
                            "Searching..."
                        } else {
                            "Search"
                        }
                    }
                }

                // Checkbox for disabling AI enhancement (only show for Vector/Both modes)
                if !matches!(search_mode(), models::SearchMode::Text) {
                    div {
                        class: "enhancement-checkbox-container",
                        input {
                            class: "enhancement-checkbox",
                            r#type: "checkbox",
                            id: "disable-enhancement",
                            checked: disable_enhancement(),
                            onchange: move |evt| disable_enhancement.set(evt.checked()),
                        }
                        label {
                            class: "enhancement-label",
                            r#for: "disable-enhancement",
                            "Disable AI query enhancement (use exact search query)"
                        }
                    }
                }

                // Display enhanced query if different from original
                if !enhanced_query().is_empty() && enhanced_query() != original_query() {
                    div {
                        class: "enhanced-query-display",
                        div {
                            class: "enhanced-query-title",
                            "AI Enhanced Query:"
                        }
                        div {
                            class: "enhanced-query-text",
                            "{enhanced_query()}"
                        }
                    }
                }
                }
            }

            // Movie grid section
            MovieGrid { 
                movies: Arc::clone(&movies()), 
                search_mode: search_mode(),
                on_location_updated: move |(movie_id, new_location): (i32, String)| {
                    log::info!("Location update callback called for movie {} with location: {}", movie_id, new_location);
                    // Update the movie in the list
                    let current_movies = movies();
                    let updated_movies: Vec<ScoredMovie> = current_movies
                        .iter()
                        .map(|scored_movie| {
                            if scored_movie.movie.id == movie_id {
                                let mut updated_movie = scored_movie.movie.clone();
                                updated_movie.location = Some(new_location.clone());
                                log::info!("Updated movie {} location to: {}", movie_id, new_location);
                                ScoredMovie {
                                    movie: updated_movie,
                                    vector_score: scored_movie.vector_score,
                                }
                            } else {
                                scored_movie.clone()
                            }
                        })
                        .collect();
                    let count = updated_movies.len();
                    movies.set(Arc::new(updated_movies));
                    log::info!("Movies signal updated, new count: {}", count);
                }
            }
        }
    }
}
