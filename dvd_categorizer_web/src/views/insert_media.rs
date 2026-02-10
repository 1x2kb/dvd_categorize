use dioxus::prelude::*;
use log::error;
use models::{CsvInput, FullMovie, ScoredMovie};
use std::sync::Arc;

use crate::components::movie_grid::MovieGrid;

#[component]
pub fn InsertMedia() -> Element {
    let mut csv_data = use_signal(|| "".to_string());
    let mut dvd_data: Signal<Arc<Vec<FullMovie>>> = use_signal(|| Arc::new(Vec::new()));
    let mut is_previewing: Signal<bool> = use_signal(|| false);
    let mut error_message: Signal<Option<String>> = use_signal(|| None);

    // Helper for pluralization
    let dvd_count = dvd_data().len();
    let movie_word = if dvd_count == 1 { "movie" } else { "movies" };

    rsx! {
        div { class: "insert-media-container",
            // CSV Input Section
            div { class: "csv-section",
                // CSV Instructions
                div { class: "csv-instructions",
                    p { strong { "CSV Format (header row required):" } }
                    p { code { "Title,Description,Actors,Genres,Director,AddedOn,Location" } }
                    p { class: "csv-note",
                        "⚠️ Header row must be included with exactly these 7 column names. Order doesn't matter."
                    }
                }

                // Error/Success message display
                if let Some(msg) = error_message() {
                    div { class: "message",
                        style: if msg.starts_with("✓") {
                            "background-color: #efe; border: 1px solid #cfc; padding: 12px; margin: 10px 0; border-radius: 4px; color: #363;"
                        } else {
                            "background-color: #fee; border: 1px solid #fcc; padding: 12px; margin: 10px 0; border-radius: 4px; color: #c33;"
                        },
                        if !msg.starts_with("✓") {
                            strong { "Error: " }
                        }
                        "{msg}"
                    }
                }

                // Textarea for CSV Input
                textarea {
                    class: "csv-textarea",
                    placeholder: "Paste your CSV data here...",
                    oninput: move |e| csv_data.set(e.value()),
                    value: "{csv_data}"
                }

                // Action Buttons
                div { class: "button-group",
                    button {
                        class: "button button-success",
                        onclick: move |_| {
                            spawn(async move {
                                let window = web_sys::window().unwrap();
                                let location = window.location();
                                let hostname = location.hostname().unwrap_or_else(|_| "127.0.0.1".to_string());
                                let server_port = std::env::var("server_port").unwrap_or("3000".to_string());

                                let url = format!("http://{}:{}/csv/export", hostname, server_port);
                                window.open_with_url(&url).ok();
                            });
                        },
                        "Export All Movies"
                    }

                    button {
                        class: "button button-primary",
                        onclick: move |_| {
                            spawn(async move {
                                error_message.set(None);
                                let window = web_sys::window().unwrap();
                                let location = window.location();
                                let hostname = location.hostname().unwrap_or_else(|_| "127.0.0.1".to_string());
                                let server_port = std::env::var("server_port").unwrap_or("3000".to_string());

                                let client = reqwest::Client::new();
                                let result = client
                                    .post(format!("http://{}:{}/csv/preview", hostname, server_port))
                                    .json(&CsvInput { input: csv_data() })
                                    .send()
                                    .await;

                                match result {
                                    Ok(response) => {
                                        let status = response.status();
                                        if status.is_success() {
                                            match response.json::<Vec<FullMovie>>().await {
                                                Ok(dvds) => {
                                                    let count = dvds.len();
                                                    let movie_word = if count == 1 { "movie" } else { "movies" };
                                                    dvd_data.set(Arc::new(dvds));
                                                    is_previewing.set(true);
                                                    error_message.set(Some(format!("✓ Successfully loaded {} {} for preview", count, movie_word)));
                                                },
                                                Err(e) => {
                                                    let err_msg = format!("Error parsing response: {}", e);
                                                    error!("{}", err_msg);
                                                    error_message.set(Some(err_msg));
                                                }
                                            }
                                        } else {
                                            match response.text().await {
                                                Ok(error_text) => {
                                                    error!("CSV parsing failed: {}", error_text);
                                                    error_message.set(Some(error_text));
                                                },
                                                Err(e) => {
                                                    let err_msg = format!("Request failed with status {}: {}", status, e);
                                                    error!("{}", err_msg);
                                                    error_message.set(Some(err_msg));
                                                }
                                            }
                                        }
                                    },
                                    Err(e) => {
                                        let err_msg = format!("Request failed: {}", e);
                                        error!("{}", err_msg);
                                        error_message.set(Some(err_msg));
                                    }
                                }
                            });
                        },
                        "Preview"
                    }

                    if is_previewing() {
                        button {
                            class: "button button-danger",
                            onclick: move |_| {
                                dvd_data.set(Arc::new(vec![]));
                                is_previewing.set(false);
                                csv_data.set(String::new());
                            },
                            "Clear"
                        }

                        button {
                            class: "button button-success",
                            onclick: move |_| {
                                spawn(async move {
                                error_message.set(None);
                                let window = web_sys::window().unwrap();
                                let location = window.location();
                                let hostname = location.hostname().unwrap_or_else(|_| "127.0.0.1".to_string());
                                let server_port = std::env::var("server_port").unwrap_or("3000".to_string());

                                let client = reqwest::Client::new();
                                let result = client
                                    .post(format!("http://{}:{}/csv/parse", hostname, server_port))
                                    .json(&CsvInput { input: csv_data() })
                                    .send()
                                    .await;

                                match result {
                                    Ok(response) => {
                                        let status = response.status();
                                        if status.is_success() {
                                            let count = dvd_data().len();
                                            let movie_word = if count == 1 { "movie" } else { "movies" };
                                            dvd_data.set(Arc::new(vec![]));
                                            is_previewing.set(false);
                                            csv_data.set(String::new());
                                            error_message.set(Some(format!("✓ Successfully saved {} {} to database!", count, movie_word)));
                                        } else {
                                            match response.text().await {
                                                Ok(error_text) => {
                                                    error!("CSV parsing/insertion failed: {}", error_text);
                                                    error_message.set(Some(error_text));
                                                },
                                                Err(e) => {
                                                    let err_msg = format!("Request failed with status {}: {}", status, e);
                                                    error!("{}", err_msg);
                                                    error_message.set(Some(err_msg));
                                                }
                                            }
                                        }
                                    },
                                    Err(e) => {
                                        let err_msg = format!("Request failed: {}", e);
                                        error!("{}", err_msg);
                                        error_message.set(Some(err_msg));
                                    }
                                }
                            });
                            },
                            "Send to Database"
                        }
                    }
                }
            }

            // Movie Grid Preview
            if !dvd_data().is_empty() {
                div { class: "preview-section",
                    h3 { class: "preview-title", "Preview ({dvd_count} {movie_word} found)" }
                    {
                        // Convert FullMovie to ScoredMovie for display (with 0 score for preview)
                        let scored_movies: Arc<Vec<ScoredMovie>> = Arc::new(
                            dvd_data().iter().map(|movie| ScoredMovie {
                                movie: movie.clone(),
                                vector_score: 0.0,
                            }).collect()
                        );
                        rsx! {
                            MovieGrid { movies: scored_movies, search_mode: models::SearchMode::Both }
                        }
                    }
                }
            }
        }
    }
}
