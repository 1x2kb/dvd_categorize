use dioxus::prelude::*;
use log::{error, info};
use models::{PullModelRequest, PullModelResponse};

#[component]
pub fn ModelPull() -> Element {
    let mut model_name = use_signal(|| "".to_string());
    let mut is_pulling = use_signal(|| false);
    let mut status_message = use_signal(|| None::<String>);
    let mut is_success = use_signal(|| false);

    rsx! {
        div { class: "model-pull-container",
            div { class: "model-pull-section",
                h2 { class: "section-title", "Pull Ollama Model" }

                div { class: "model-instructions",
                    p {
                        "Enter the name of the Ollama model you want to pull. Common models include:"
                    }
                    ul { class: "model-examples",
                        li { code { "llama3.2" } " - Default chat model" }
                        li { code { "phi3.5" } " - Smaller, faster model" }
                        li { code { "nomic-embed-text" } " - Embedding model (currently used)" }
                        li { code { "mistral" } " - Alternative chat model" }
                        li { code { "codellama" } " - Code-focused model" }
                    }
                }

                div { class: "input-group",
                    label {
                        r#for: "model-name",
                        class: "input-label",
                        "Model Name:"
                    }
                    input {
                        id: "model-name",
                        class: "model-input",
                        r#type: "text",
                        placeholder: "e.g., llama3.2",
                        value: "{model_name}",
                        oninput: move |e| {
                            model_name.set(e.value());
                            status_message.set(None);
                        },
                        disabled: is_pulling()
                    }
                }

                div { class: "button-group",
                    button {
                        class: if is_pulling() { "button button-primary button-disabled" } else { "button button-primary" },
                        disabled: is_pulling() || model_name().trim().is_empty(),
                        onclick: move |_| {
                            let model = model_name().trim().to_string();
                            if model.is_empty() {
                                status_message.set(Some("Please enter a model name".to_string()));
                                is_success.set(false);
                                return;
                            }

                            is_pulling.set(true);
                            status_message.set(Some("Pulling model... This may take a few minutes.".to_string()));
                            is_success.set(false);

                            spawn(async move {
                                let window = web_sys::window().unwrap();
                                let location = window.location();
                                let hostname = location.hostname().unwrap_or_else(|_| "127.0.0.1".to_string());
                                let server_port = std::env::var("server_port").unwrap_or("3000".to_string());

                                let client = reqwest::Client::new();
                                let result = client
                                    .post(format!("http://{}:{}/ai/pull-model", hostname, server_port))
                                    .json(&PullModelRequest { model_name: model.clone() })
                                    .send()
                                    .await;

                                match result {
                                    Ok(response) => {
                                        if response.status().is_success() {
                                            match response.json::<PullModelResponse>().await {
                                                Ok(pull_response) => {
                                                    info!("Model pull response: {:?}", pull_response);
                                                    status_message.set(Some(pull_response.message));
                                                    is_success.set(pull_response.success);
                                                },
                                                Err(e) => {
                                                    error!("Error parsing response: {}", e);
                                                    status_message.set(Some("Error: Failed to parse response".to_string()));
                                                    is_success.set(false);
                                                }
                                            }
                                        } else {
                                            let error_msg = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                                            error!("Server error: {}", error_msg);
                                            status_message.set(Some(format!("Error: {}", error_msg)));
                                            is_success.set(false);
                                        }
                                    },
                                    Err(e) => {
                                        error!("Request failed: {}", e);
                                        status_message.set(Some(format!("Error: {}", e)));
                                        is_success.set(false);
                                    }
                                }

                                is_pulling.set(false);
                            });
                        },
                        if is_pulling() {
                            "Pulling..."
                        } else {
                            "Pull Model"
                        }
                    }

                    if !model_name().is_empty() {
                        button {
                            class: "button button-secondary",
                            disabled: is_pulling(),
                            onclick: move |_| {
                                model_name.set(String::new());
                                status_message.set(None);
                                is_success.set(false);
                            },
                            "Clear"
                        }
                    }
                }

                if let Some(msg) = status_message() {
                    div {
                        class: if is_success() { "status-message status-success" } else { "status-message status-error" },
                        p { "{msg}" }
                    }
                }

                div { class: "info-section",
                    h3 { "About Model Pulling" }
                    p {
                        "Pulling a model downloads it from the Ollama registry and makes it available for use. "
                        "The first time you pull a model, it may take several minutes depending on the model size "
                        "and your internet connection."
                    }
                    p { class: "info-note",
                        "⚠️ Note: Large models can be several gigabytes in size. Ensure you have sufficient disk space "
                        "and bandwidth before pulling."
                    }
                }
            }
        }
    }
}
