use dioxus::prelude::*;
use log::error;
use models::{CsvInput, FullMovie, ScoredMovie};

use crate::components::movie_grid::MovieGrid;

#[component]
pub fn InsertMedia() -> Element {
    let mut csv_data = use_signal(|| "".to_string());
    let mut dvd_data: Signal<Vec<FullMovie>> = use_signal(Vec::new);
    let mut is_previewing: Signal<bool> = use_signal(|| false);
    let mut error_message: Signal<Option<String>> = use_signal(|| None);
    let mut is_sending: Signal<bool> = use_signal(|| false);
    let mut focused: Signal<Option<u64>> = use_signal(|| None::<u64>);

    // Preview rows derive from dvd_data; the memo only recomputes when it changes.
    let scored_movies = use_memo(move || {
        dvd_data
            .read()
            .iter()
            .map(|movie| ScoredMovie {
                movie: movie.clone(),
                vector_score: 0.0,
            })
            .collect::<Vec<_>>()
    });

    // Helper for pluralization
    let dvd_count = dvd_data.read().len();
    let movie_word = if dvd_count == 1 { "movie" } else { "movies" };

    rsx! {
        div {
            class: "insert-media-container",
            // Any click that reaches the page clears the card lock. Cards stop
            // propagation, so their own toggle is unaffected.
            onclick: move |_| focused.set(None),

            // CSV Input Section
            div { class: "csv-section",
                // CSV Instructions
                div { class: "csv-instructions",
                    p { strong { "CSV Format (header row required):" } }
                    p { code { "Title,Year,Description,Actors,Genres,Director,AddedOn,Location" } }
                    p { class: "csv-note",
                        "⚠️ Header row must be included with exactly these 8 column names. Order doesn't matter but must be consistent."
                    }
                }

                // Error/Success message display
                if let Some(msg) = error_message() {
                    div {
                        class: if msg.starts_with("✓") { "message message-success" } else { "message message-error" },
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
                                            // Get text first to log full response on error
                                            match response.text().await {
                                                Ok(text) => {
                                                    match serde_json::from_str::<Vec<FullMovie>>(&text) {
                                                        Ok(dvds) => {
                                                            let count = dvds.len();
                                                            let movie_word = if count == 1 { "movie" } else { "movies" };
                                                            dvd_data.set(dvds);
                                                            is_previewing.set(true);
                                                            error_message.set(Some(format!("✓ Successfully loaded {} {} for preview", count, movie_word)));
                                                        }
                                                        Err(e) => {
                                                            let err_msg = format!("Serde error: {}\n\nResponse body:\n{}", e, text);
                                                            error!("{}", err_msg);
                                                            error_message.set(Some(err_msg));
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    let err_msg = format!("Failed to get response text: {}", e);
                                                    error!("{}", err_msg);
                                                    error_message.set(Some(err_msg));
                                                }
                                            }
                                        } else {
                                            match response.text().await {
                                                Ok(error_text) => {
                                                    error!("CSV parsing failed: {}", error_text);
                                                    error_message.set(Some(error_text));
                                                }
                                                Err(e) => {
                                                    let err_msg = format!("Request failed with status {}: {}", status, e);
                                                    error!("{}", err_msg);
                                                    error_message.set(Some(err_msg));
                                                }
                                            }
                                        }
                                    }
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
                                dvd_data.set(Vec::new());
                                is_previewing.set(false);
                                csv_data.set(String::new());
                            },
                            "Clear"
                        }

                        button {
                            class: "button button-success",
                            disabled: is_sending(),
                            onclick: move |_| {
                                spawn(async move {
                                is_sending.set(true);
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
                                            let count = dvd_data.read().len();
                                            let movie_word = if count == 1 { "movie" } else { "movies" };
                                            dvd_data.set(Vec::new());
                                            is_previewing.set(false);
                                            csv_data.set(String::new());
                                            error_message.set(Some(format!("✓ Successfully saved {} {} to database!", count, movie_word)));
                                        } else {
                                            match response.text().await {
                                                Ok(error_text) => {
                                                    error!("CSV parsing/insertion failed: {}", error_text);
                                                    error_message.set(Some(error_text));
                                                }
                                                Err(e) => {
                                                    let err_msg = format!("Request failed with status {}: {}", status, e);
                                                    error!("{}", err_msg);
                                                    error_message.set(Some(err_msg));
                                                }
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        let err_msg = format!("Request failed: {}", e);
                                        error!("{}", err_msg);
                                        error_message.set(Some(err_msg));
                                    }
                                }
                                is_sending.set(false);
                            });
                            },
                            if is_sending() {
                                "Sending..."
                            } else {
                                "Send to Database"
                            }
                        }
                    }
                }
            }

            // Movie Grid Preview
            if !dvd_data.read().is_empty() {
                div { class: "preview-section",
                    h3 { class: "preview-title", "Preview ({dvd_count} {movie_word} found)" }
                    div { class: "preview-wrapper",
                        MovieGrid {
                            movies: scored_movies,
                            search_mode: models::SearchMode::Text,
                            on_location_updated: move |(movie_id, new_location): (models::MovieId, String)| {
                                // In-place edit; the memo recomputes from this.
                                dvd_data.with_mut(|movies| {
                                    if let Some(movie) = movies.iter_mut().find(|m| m.id == movie_id) {
                                        movie.location = Some(new_location);
                                    }
                                });
                            },
                            // Preview rows aren't persisted yet, so removal is local only.
                            persist_deletes: false,
                            on_movie_deleted: move |(_movie_id, key_hash): (models::MovieId, u64)| {
                                dvd_data.with_mut(|movies| {
                                    movies.retain(|m| m.key_hash != key_hash);
                                    if movies.is_empty() {
                                        is_previewing.set(false);
                                    }
                                });
                            },
                            focused,
                        }

                        if is_sending() {
                            div { class: "loading-overlay",
                                div { class: "loading-content",
                                    div { class: "spinner" }
                                    p { "Sending to database..." }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}