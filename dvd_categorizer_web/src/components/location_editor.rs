use dioxus::prelude::*;
use models::UpdateLocationRequest;
use wasm_bindgen::JsCast;

// Context to track if any location editor is currently active
pub fn use_editing_context() -> Signal<bool> {
    use_context::<Signal<bool>>()
}

async fn update_location_on_server(movie_id: i32, location: String) -> Result<(), String> {
    let window = web_sys::window().ok_or("No window")?;
    let hostname = window
        .location()
        .hostname()
        .unwrap_or_else(|_| "127.0.0.1".to_string());
    let server_port = std::env::var("server_port").unwrap_or("3000".to_string());

    let request = UpdateLocationRequest { movie_id, location };

    let client = reqwest::Client::new();
    client
        .post(format!("http://{hostname}:{server_port}/movie/location"))
        .json(&request)
        .send()
        .await
        .map_err(
            |e| {
                format!(
                    "Request failed: {}",
                    e
                )
            },
        )?
        .error_for_status()
        .map_err(
            |e| {
                format!(
                    "Server error: {}",
                    e
                )
            },
        )?;

    Ok(())
}

#[derive(Props, Clone, PartialEq)]
pub struct LocationEditorProps {
    pub movie_id: i32,
    pub initial_location: String,
    pub on_location_updated: EventHandler<(
        i32,
        String,
    )>,
}

#[component]
pub fn LocationEditor(props: LocationEditorProps) -> Element {
    // Track the current displayed location separately from props for reactivity
    let mut current_location = use_signal(
        || {
            props
                .initial_location
                .clone()
        },
    );
    let mut editing = use_signal(|| false);
    let mut location_input = use_signal(String::new);
    let mut is_saving = use_signal(|| false);
    let mut save_error = use_signal(|| None::<String>);

    let input_id = use_signal(
        || {
            format!(
                "location-input-{}",
                props.movie_id
            )
        },
    );

    // Get global editing context
    let mut global_editing = use_editing_context();

    // Update global editing state when local editing changes
    use_effect(
        move || {
            global_editing.set(editing());
        },
    );

    // Sync current_location when props change (Option 2B: parent updates flow to child)
    use_effect(
        move || {
            let new_location = props
                .initial_location
                .clone();
            if current_location() != new_location {
                current_location.set(new_location);
            }
        },
    );

    // Focus input when editing mode is activated
    use_effect(
        move || {
            if editing() {
                let id = input_id();
                spawn(
                    async move {
                        // Small delay to ensure DOM is updated
                        gloo_timers::future::TimeoutFuture::new(10).await;
                        if let Some(window) = web_sys::window() {
                            if let Some(document) = window.document() {
                                if let Some(element) = document.get_element_by_id(&id) {
                                    if let Some(input) =
                                        element.dyn_ref::<web_sys::HtmlInputElement>()
                                    {
                                        let _ = input.focus();
                                    }
                                }
                            }
                        }
                    },
                );
            }
        },
    );

    rsx! {
        div {
            class: "movie-info-row movie-location-row",

            if editing() {
                // Edit mode
                div {
                    class: "location-edit-container",
                    span {
                        class: "movie-info-label",
                        "Location: "
                    }
                    input {
                        id: "{input_id()}",
                        class: "location-input",
                        r#type: "text",
                        value: "{location_input}",
                        oninput: move |evt| location_input.set(evt.value()),
                        disabled: is_saving(),
                        onkeydown: move |evt| {
                            if evt.key() == Key::Enter && !is_saving() {
                                let on_updated = props.on_location_updated.clone();
                                let movie_id = props.movie_id;
                                spawn(async move {
                                    is_saving.set(true);
                                    save_error.set(None);

                                    match update_location_on_server(movie_id, location_input()).await {
                                        Ok(_) => {
                                            let new_loc = location_input();
                                            current_location.set(new_loc.clone());
                                            on_updated.call((movie_id, new_loc));
                                            editing.set(false);
                                        }
                                        Err(e) => {
                                            log::error!("Failed to update location: {}", e);
                                            save_error.set(Some(e));
                                        }
                                    }

                                    is_saving.set(false);
                                });
                            } else if evt.key() == Key::Escape && !is_saving() {
                                // Cancel on Escape
                                editing.set(false);
                                save_error.set(None);
                                location_input.set(current_location());
                            }
                        },
                    }
                    button {
                        class: "location-save-button",
                        onclick: move |_| {
                            let on_updated = props.on_location_updated.clone();
                            let movie_id = props.movie_id;
                            spawn(async move {
                                is_saving.set(true);
                                save_error.set(None);

                                match update_location_on_server(movie_id, location_input()).await {
                                    Ok(_) => {
                                        let new_loc = location_input();
                                        current_location.set(new_loc.clone());
                                        on_updated.call((movie_id, new_loc));
                                        editing.set(false);
                                    }
                                    Err(e) => {
                                        log::error!("Failed to update location: {}", e);
                                        save_error.set(Some(e));
                                    }
                                }

                                is_saving.set(false);
                            });
                        },
                        disabled: is_saving(),
                        if is_saving() {
                            "Saving..."
                        } else {
                            "Save"
                        }
                    }
                    button {
                        class: "location-cancel-button",
                        onclick: move |_| {
                            editing.set(false);
                            save_error.set(None);
                            location_input.set(current_location());
                        },
                        disabled: is_saving(),
                        "Cancel"
                    }
                }

                // Show error if present
                if let Some(error) = save_error() {
                    div {
                        class: "location-error",
                        "Error: {error}"
                    }
                }
            } else {
                // Display mode
                div {
                    class: "location-display-container",
                    span {
                        class: "movie-info-label",
                        "Location: "
                    }
                    span {
                        class: "movie-info-value location-value",
                        "{current_location()}"
                    }
                    button {
                        class: "location-edit-button",
                        onclick: move |_| {
                            location_input.set(current_location());
                            editing.set(true);
                            save_error.set(None);
                        },
                        "Edit"
                    }
                }
            }
        }
    }
}
