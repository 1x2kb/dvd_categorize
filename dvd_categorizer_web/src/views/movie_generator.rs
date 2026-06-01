use dioxus::prelude::*;
use log::error;
use models::{AiMovieData, AvailableModel, AvailableModelsResponse, GenerateStreamEvent, NeedsInputReason};
use crate::components::toast::{ToastContainer, ToastMessage};
use serde::Serialize;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::ReadableStreamDefaultReader;

use crate::components::generated_movie_grid::GeneratedMovieGrid;

/// Request to generate movies from titles
#[derive(Serialize)]
struct GenerateMoviesRequest {
    titles: Vec<String>,
    model: Option<String>,
    positions: Vec<usize>,
}

#[component]
pub fn MoviePrompt() -> Element {
    let mut title_input = use_signal(|| "".to_string());
    let mut chips: Signal<Vec<String>> = use_signal(Vec::new);
    let mut selected_model = use_signal(|| "".to_string());
    let mut available_models = use_signal(Vec::<AvailableModel>::new);
    let mut is_loading = use_signal(|| false);
    let mut error_message = use_signal(|| None::<String>);
    let mut generated_movies: Signal<Vec<AiMovieData>> = use_signal(Vec::new);
    let mut pending_inputs: Signal<Vec<(String, NeedsInputReason)>> = use_signal(Vec::new);
    let mut toasts: Signal<Vec<ToastMessage>> = use_signal(Vec::new);
    let mut has_results = use_signal(|| false);
    let mut generate_trigger: Signal<u32> = use_signal(|| 0);

    let mut push_toast = move |msg: ToastMessage| toasts.with_mut(|v| v.push(msg));
    let mut dismiss_toast = move |id: u32| toasts.with_mut(|v| v.retain(|t| t.id != id));

    // Fetch available models on component mount
    use_effect(move || {
        spawn(async move {
            let window = web_sys::window().unwrap();
            let location = window.location();
            let hostname = location.hostname().unwrap_or_else(|_| "127.0.0.1".to_string());
            let server_port = std::env::var("server_port").unwrap_or("3000".to_string());
            let url = format!("http://{}:{}/ai/models", hostname, server_port);
            match gloo_net::http::Request::get(&url).send().await {
                Ok(response) => {
                    if let Ok(parsed) = response.json::<AvailableModelsResponse>().await {
                        if let Some(first_model) = parsed.models.first() {
                            if selected_model().is_empty() {
                                selected_model.set(first_model.name.clone());
                            }
                        }
                        available_models.set(parsed.models);
                    }
                }
                Err(e) => error!("Failed to load available models: {:?}", e),
            }
        });
    });

    // Add titles as simple string chips.
    let add_title = move |_: MouseEvent| {
        let raw = title_input();
        let new_titles: Vec<String> = raw
            .split('|')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if new_titles.is_empty() { return; }
        let added: Vec<String> = new_titles.into_iter().filter(|t| {
            !chips().iter().any(|c| c.to_lowercase() == t.to_lowercase())
        }).collect();
        if added.is_empty() { title_input.set("".to_string()); return; }
        chips.with_mut(|v| v.extend(added));
        title_input.set("".to_string());
    };

    let generate_movies = move |_| {
        if chips().is_empty() {
            error_message.set(Some("Add at least one movie title first".to_string()));
            return;
        }
        error_message.set(None);
        generated_movies.set(Vec::new());
        pending_inputs.set(Vec::new());
        has_results.set(true);
        generate_trigger.with_mut(|t| *t += 1);
    };

    rsx! {
        ToastContainer {
            toasts: toasts,
            on_dismiss: move |id| dismiss_toast(id),
        }
        div {
            class: "movie-prompt-container",
            onkeydown: move |e: KeyboardEvent| {
                if e.key() == Key::Enter && e.modifiers().ctrl() {
                    if chips().is_empty() {
                        error_message.set(Some("Add at least one movie title first".to_string()));
                        return;
                    }
                    error_message.set(None);
                    generated_movies.set(Vec::new());
                    pending_inputs.set(Vec::new());
                    has_results.set(true);
                    generate_trigger.with_mut(|t| *t += 1);
                }
            },
            h2 { class: "page-title", "Movie Generator" }
            p { class: "page-description",
                "Add movie titles below, then click Generate. Lock cards you're happy with and Generate again to redo the rest."
            }

            // Model Selection
            div { class: "model-section",
                label { class: "input-label", "AI Model:" }
                select {
                    class: "model-select",
                    onchange: move |e| selected_model.set(e.value()),
                    if !available_models().iter().any(|m| m.name == selected_model()) && !selected_model().is_empty() {
                        option { value: "{selected_model}", selected: true, "{selected_model}" }
                    }
                    for model in available_models().iter() {
                        option {
                            key: "{model.name}",
                            value: "{model.name}",
                            selected: model.name == selected_model(),
                            "{model.name}"
                        }
                    }
                }
            }

            // Error/Success Display
            if let Some(error) = error_message() {
                div { class: "message message-error",
                    strong { "Error: " }
                    "{error}"
                }
            }

            // Title chip input
            div { class: "input-section",
                h3 { class: "section-title", "Movie Titles" }

                div { class: "title-add-row",
                    input {
                        class: "title-add-input",
                        r#type: "text",
                        placeholder: "e.g. The Matrix",
                        value: "{title_input}",
                        oninput: move |e| title_input.set(e.value()),
                        onkeydown: move |e: KeyboardEvent| {
                            if e.key() == Key::Enter {
                                let raw = title_input();
                                let new_titles: Vec<String> = raw.split('|').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
                                if new_titles.is_empty() { return; }
                                let added: Vec<String> = new_titles.into_iter().filter(|t| !chips().iter().any(|c| c.to_lowercase() == t.to_lowercase())).collect();
                                if added.is_empty() { title_input.set("".to_string()); return; }
                                chips.with_mut(|v| v.extend(added));
                                title_input.set("".to_string());
                            }
                        },
                    }
                    button {
                        class: "button button-secondary",
                        onclick: add_title,
                        "+ Add"
                    }
                }

                // Chip list
                if !chips().is_empty() {
                    div { class: "title-chip-list",
                        for (idx, chip) in chips().iter().enumerate() {
                            div { class: "title-chip", key: "{idx}-{chip}",
                                span { class: "title-chip-text", "{chip}" }
                                button {
                                    class: "title-chip-remove",
                                    title: "Remove",
                                    onclick: move |_| chips.with_mut(|v| { v.remove(idx); }),
                                    "×"
                                }
                            }
                        }
                    }
                }

                div { class: "input-buttons",
                    button {
                        class: "button button-primary",
                        disabled: is_loading() || chips().is_empty(),
                        onclick: generate_movies,
                        if is_loading() { "Generating..." } else { "Generate" }
                    }
                    button {
                        class: "button button-secondary",
                        disabled: is_loading(),
                        onclick: move |_| {
                            chips.set(Vec::new());
                            generated_movies.set(Vec::new());
                            has_results.set(false);
                            error_message.set(None);
                        },
                        "Clear All"
                    }
                }
            }

            // Generated Movie Grid
            if has_results() {
                div { class: "preview-section",
                    h3 { class: "preview-title",
                        "🔒 Lock cards to keep, Generate again to redo the rest"
                    }
                    div { class: "preview-wrapper",
                        GeneratedMovieGrid {
                            movies: generated_movies,
                            pending_inputs: pending_inputs,
                            input_titles: chips(),
                            generate_trigger: generate_trigger,
                            model: selected_model(),
                            on_movies_changed: move |updated: Vec<AiMovieData>| {
                                generated_movies.set(updated);
                            },
                            on_loading: move |loading: bool| {
                                is_loading.set(loading);
                            },
                            on_error: move |e: String| {
                                error_message.set(Some(e));
                            },
                            on_pending: move |(title, reason): (String, NeedsInputReason)| {
                                pending_inputs.with_mut(|v| v.push((title, reason)));
                            },
                            on_toast: move |msg: ToastMessage| push_toast(msg),
                            on_dismiss_toast: move |id: u32| dismiss_toast(id),
                            on_title_corrected: move |(original, corrected): (String, String)| {
                                chips.with_mut(|v| {
                                    if let Some(c) = v.iter_mut().find(|c| c.to_lowercase() == original.to_lowercase()) {
                                        *c = corrected;
                                    }
                                });
                            },
                            on_title_removed: move |title: String| {
                                chips.with_mut(|v| v.retain(|c| c.to_lowercase() != title.to_lowercase()));
                            },
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async fn get_api_base() -> String {
    let window = web_sys::window().unwrap();
    let location = window.location();
    let hostname = location.hostname().unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("server_port").unwrap_or("3000".to_string());
    format!("http://{}:{}", hostname, port)
}

/// Stream movie generation via SSE. Calls `on_movie` for each generated card,
/// `on_needs_input` for titles requiring user action, `on_done` when done, `on_err` on failure.
pub async fn stream_generated_movies(
    titles: Vec<String>,
    positions: Vec<usize>,
    model: String,
    mut on_movie: impl FnMut(AiMovieData, Option<NeedsInputReason>),
    on_done: impl FnOnce(),
    on_err: impl FnOnce(String),
) {
    let base = get_api_base().await;
    let request = GenerateMoviesRequest {
        titles,
        model: if model.is_empty() { None } else { Some(model) },
        positions,
    };

    let body = match serde_json::to_string(&request) {
        Ok(b) => b,
        Err(e) => { on_err(format!("Serialization error: {}", e)); return; }
    };

    let response = match gloo_net::http::Request::post(&format!("{}/ai/generate-movies-stream", base))
        .header("Content-Type", "application/json")
        .body(body)
        .map_err(|e| format!("Failed to build request: {}", e))
        .and_then(|r| Ok(r))
    {
        Ok(req) => match req.send().await {
            Ok(r) => r,
            Err(e) => { on_err(format!("Request failed: {}", e)); return; }
        },
        Err(e) => { on_err(e); return; }
    };

    if !response.ok() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        on_err(format!("Server error {}: {}", status, body));
        return;
    }

    // Read the SSE stream line-by-line from the raw response body.
    let body_stream = match response.body() {
        Some(s) => s,
        None => { on_err("No response body".to_string()); return; }
    };

    let reader: ReadableStreamDefaultReader = body_stream
        .get_reader()
        .dyn_into()
        .expect("reader cast");
    let mut buf = String::new();

    loop {
        let chunk = JsFuture::from(reader.read()).await;
        match chunk {
            Err(e) => {
                on_err(format!("Stream read error: {:?}", e));
                return;
            }
            Ok(val) => {
                let done = js_sys::Reflect::get(&val, &"done".into())
                    .ok()
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                if done { break; }

                let chunk_val = js_sys::Reflect::get(&val, &"value".into()).unwrap();
                let chunk_u8: js_sys::Uint8Array = chunk_val.dyn_into().expect("Uint8Array");
                let text = String::from_utf8_lossy(&chunk_u8.to_vec()).into_owned();
                buf.push_str(&text);

                // SSE lines look like: "data: {...}\n\n" or "event: done\n\n"
                while let Some(pos) = buf.find("\n\n") {
                    let block = buf[..pos].trim().to_string();
                    buf = buf[pos + 2..].to_string();

                    for line in block.lines() {
                        if let Some(data) = line.strip_prefix("data:") {
                            let data = data.trim();
                            if data.is_empty() { continue; }
                            match serde_json::from_str::<GenerateStreamEvent>(data) {
                                Ok(GenerateStreamEvent::Movie(movie)) => on_movie(movie, None),
                                Ok(GenerateStreamEvent::NeedsInput { original, reason, position }) => {
                                    let placeholder = AiMovieData {
                                        title: original.clone(),
                                        year: 0,
                                        description: String::new(),
                                        actors: vec![],
                                        genres: vec![],
                                        director: String::new(),
                                        already_in_catalog: false,
                                        input_title: Some(original.clone()),
                                        position,
                                    };
                                    on_movie(placeholder, Some(reason));
                                },
                                Err(e) => error!("Failed to parse SSE event: {} — {}", e, data),
                            }
                        } else if line.starts_with("event: done") {
                            on_done();
                            return;
                        } else if let Some(err) = line.strip_prefix("event: error") {
                            on_err(err.trim().to_string());
                            return;
                        }
                    }
                }
            }
        }
    }
    on_done();
}

/// Kept for the grid's re-generate path (unlocked titles only, returns all at once).
pub async fn fetch_generated_movies(
    titles: Vec<String>,
    model: String,
) -> Result<Vec<AiMovieData>, String> {
    let mut results = Vec::new();
    let mut error: Option<String> = None;
    stream_generated_movies(
        titles,
        vec![],
        model,
        |m, _| results.push(m),
        || {},
        |e| error = Some(e),
    ).await;
    if let Some(e) = error { Err(e) } else { Ok(results) }
}
