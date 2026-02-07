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

#[component]
pub fn AiLiveResults() -> Element {
    let mut movies: Signal<Arc<Vec<ScoredMovie>>> = use_signal(|| Arc::new(Vec::new()));
    let mut input_value = use_signal(String::new);
    let mut is_loading = use_signal(|| false);
    let mut disable_enhancement = use_signal(|| false);
    let mut search_mode = use_signal(|| models::SearchMode::Both);
    let mut enhanced_query = use_signal(String::new);
    let mut original_query = use_signal(String::new);

    rsx! {
        div {
            class: "ai-live-results-container",

            // Input and button section
            div {
                class: "search-section",
                style: "margin-bottom: 20px; border-radius: 8px; overflow: hidden; box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.3);",

                // Tab selector at the top
                div {
                    style: "display: flex; background: #111827; border-bottom: 3px solid #374151;",
                    
                    button {
                        onclick: move |_| search_mode.set(models::SearchMode::Text),
                        style: format!(
                            "flex: 1; padding: 14px 20px; background: {}; color: {}; border: none; cursor: pointer; font-weight: 600; font-size: 14px; transition: all 0.2s; border-bottom: 3px solid {}; position: relative;",
                            if matches!(search_mode(), models::SearchMode::Text) { "#1f2937" } else { "transparent" },
                            if matches!(search_mode(), models::SearchMode::Text) { "#0891b2" } else { "#6b7280" },
                            if matches!(search_mode(), models::SearchMode::Text) { "#0891b2" } else { "transparent" }
                        ),
                        "Text Search"
                    }

                    button {
                        onclick: move |_| search_mode.set(models::SearchMode::Vector),
                        style: format!(
                            "flex: 1; padding: 14px 20px; background: {}; color: {}; border: none; cursor: pointer; font-weight: 600; font-size: 14px; transition: all 0.2s; border-bottom: 3px solid {}; position: relative;",
                            if matches!(search_mode(), models::SearchMode::Vector) { "#1f2937" } else { "transparent" },
                            if matches!(search_mode(), models::SearchMode::Vector) { "#0891b2" } else { "#6b7280" },
                            if matches!(search_mode(), models::SearchMode::Vector) { "#0891b2" } else { "transparent" }
                        ),
                        "Vector Search"
                    }

                    button {
                        onclick: move |_| search_mode.set(models::SearchMode::Both),
                        style: format!(
                            "flex: 1; padding: 14px 20px; background: {}; color: {}; border: none; cursor: pointer; font-weight: 600; font-size: 14px; transition: all 0.2s; border-bottom: 3px solid {}; position: relative;",
                            if matches!(search_mode(), models::SearchMode::Both) { "#1f2937" } else { "transparent" },
                            if matches!(search_mode(), models::SearchMode::Both) { "#0891b2" } else { "#6b7280" },
                            if matches!(search_mode(), models::SearchMode::Both) { "#0891b2" } else { "transparent" }
                        ),
                        "Both (RRF)"
                    }
                }

                div {
                    style: "padding: 20px; background: #1f2937;",
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
                                    movies.set(Arc::new(vec![]));

                                    let result = send_search_request(input_value(), disable_enhancement(), search_mode()).await;
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
                        onclick: move |_| {
                            spawn(async move {
                                if is_loading() {
                                    return;
                                }

                                is_loading.set(true);
                                movies.set(Arc::new(vec![]));

                                let result = send_search_request(input_value(), disable_enhancement(), search_mode()).await;
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
                        style: "padding: 12px 24px; background: linear-gradient(135deg, #0891b2, #0e7490); color: white; border: none; border-radius: 6px; cursor: pointer; font-weight: 600; min-width: 100px;",

                        if is_loading() {
                            "Searching..."
                        } else {
                            "Search"
                        }
                    }

                    button {
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
                        style: "padding: 12px 24px; background: linear-gradient(135deg, #6366f1, #4f46e5); color: white; border: none; border-radius: 6px; cursor: pointer; font-weight: 600; min-width: 100px;",

                        if is_loading() {
                            "Loading..."
                        } else {
                            "Recent"
                        }
                    }
                }

                // Checkbox for disabling AI enhancement
                div {
                    style: "margin-top: 12px; display: flex; align-items: center; gap: 8px;",
                    input {
                        r#type: "checkbox",
                        id: "disable-enhancement",
                        checked: disable_enhancement(),
                        onchange: move |evt| disable_enhancement.set(evt.checked()),
                        style: "cursor: pointer;",
                    }
                    label {
                        r#for: "disable-enhancement",
                        style: "color: #9ca3af; font-size: 14px; cursor: pointer; user-select: none;",
                        "Disable AI query enhancement (use exact search query)"
                    }
                }

                // Display enhanced query if different from original
                if !enhanced_query().is_empty() && enhanced_query() != original_query() {
                    div {
                        style: "margin-top: 12px; padding: 10px; background: rgba(8, 145, 178, 0.1); border-left: 3px solid #0891b2; border-radius: 4px;",
                        div {
                            style: "color: #0891b2; font-size: 12px; font-weight: 600; margin-bottom: 4px;",
                            "AI Enhanced Query:"
                        }
                        div {
                            style: "color: #d1d5db; font-size: 14px;",
                            "{enhanced_query()}"
                        }
                    }
                }
                }
            }

            // Movie grid section
            MovieGrid { movies: Arc::clone(&movies()), search_mode: search_mode() }
        }
    }
}
