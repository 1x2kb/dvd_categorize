use chrono::{DateTime, Local, NaiveDateTime};
use dioxus::prelude::*;
use models::{ScoredMovie, UpdateLocationRequest};
use std::sync::Arc;

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
pub struct SingleMovieCardProps {
    pub scored_movie: ScoredMovie,
}

#[component]
fn MovieCard(props: SingleMovieCardProps) -> Element {
    let mut editing = use_signal(|| false);
    let mut location_input = use_signal(String::new);
    let mut is_saving = use_signal(|| false);
    let mut save_error = use_signal(|| None::<String>);

    let movie_id = props
        .scored_movie
        .movie
        .id;
    let location_opt = use_memo(
        move || {
            props
                .scored_movie
                .movie
                .location
                .clone()
        },
    );

    rsx! {
        div {
            class: "movie-card",

            // Movie poster placeholder
            div {
                class: "movie-poster",
                "{props.scored_movie.movie.name}"
            }

            // Movie details
            div {
                class: "movie-details",

                // Title
                h3 {
                    class: "movie-title",
                    "{props.scored_movie.movie.name}"
                }

                // Scores
                div {
                    class: "movie-info-row",
                    style: "display: flex; gap: 15px; margin-bottom: 8px;",
                    span {
                        class: "movie-info-label",
                        style: "background: rgba(8, 145, 178, 0.2); padding: 2px 8px; border-radius: 4px; font-size: 12px;",
                        "RRF Score: {props.scored_movie.vector_score:.4}"
                    }
                }

                // Description
                if let Some(description) = &props.scored_movie.movie.description {
                    p {
                        class: "movie-description",
                        "{description}"
                    }
                }

                // Actors
                if !props.scored_movie.movie.actors.is_empty() {
                    div {
                        class: "movie-info-row",
                        span {
                            class: "movie-info-label",
                            "Cast: "
                        }
                        span {
                            class: "movie-info-value",
                            {
                                props.scored_movie.movie.actors.iter().map(|a| a.name.as_str()).collect::<Vec<_>>().join(", ")
                            }
                        }
                    }
                }

                // Director
                if let Some(director) = &props.scored_movie.movie.director {
                    div {
                        class: "movie-info-row",
                        span {
                            class: "movie-info-label",
                            "Director: "
                        }
                        span {
                            class: "movie-info-value",
                            "{director.name}"
                        }
                    }
                }

                // Genres
                if !props.scored_movie.movie.genres.is_empty() {
                    div {
                        class: "movie-genres-container",
                        div {
                            class: "movie-genres",
                            // Show first 4 genres normally
                            for genre in props.scored_movie.movie.genres.iter().take(4) {
                                span {
                                    key: "{genre}",
                                    class: "movie-genre-tag",
                                    "{genre}"
                                }
                            }
                            // Show additional genres (hidden by default, shown on hover)
                            if props.scored_movie.movie.genres.len() > 4 {
                                for genre in props.scored_movie.movie.genres.iter().skip(4) {
                                    span {
                                        key: "{genre}",
                                        class: "movie-genre-tag movie-genre-extra",
                                        "{genre}"
                                    }
                                }
                                span {
                                    class: "movie-genre-tag movie-genre-more",
                                    "+{props.scored_movie.movie.genres.len() - 4}"
                                }
                            }
                        }
                    }
                }

                // Added On timestamp
                if let Some(added_on) = &props.scored_movie.movie.added_on {
                    div {
                        class: "movie-info-row",
                        style: "margin-top: 8px; font-size: 12px; opacity: 0.8;",
                        span {
                            class: "movie-info-label",
                            "Added: "
                        }
                        span {
                            class: "movie-info-value",
                            {
                                // Parse the timestamp and format it to local time
                                if let Ok(naive_dt) = NaiveDateTime::parse_from_str(added_on, "%Y-%m-%d %H:%M:%S%.f") {
                                    let utc_dt = DateTime::<chrono::Utc>::from_naive_utc_and_offset(naive_dt, chrono::Utc);
                                    let local_dt: DateTime<Local> = DateTime::from(utc_dt);
                                    local_dt.format("%B %d, %Y at %I:%M %p").to_string()
                                } else {
                                    // Fallback to raw string if parsing fails
                                    added_on.to_string()
                                }
                            }
                        }
                    }
                }

                // Location - Editable
                if location_opt().is_some() {
                    div {
                        class: "movie-info-row",
                        style: "margin-top: 8px; font-size: 12px;",

                        if editing() {
                            // Edit mode
                            div {
                                style: "display: flex; gap: 8px; align-items: center;",
                                span {
                                    class: "movie-info-label",
                                    "Location: "
                                }
                                input {
                                    r#type: "text",
                                    value: "{location_input}",
                                    oninput: move |evt| location_input.set(evt.value()),
                                    style: "flex: 1; padding: 4px 8px; border: 1px solid #0891b2; border-radius: 4px; background: #111827; color: white; font-size: 12px;",
                                    disabled: is_saving(),
                                }
                                button {
                                    onclick: move |_| {
                                        let new_location = location_input();
                                        spawn(async move {
                                            is_saving.set(true);
                                            save_error.set(None);

                                            match update_location_on_server(movie_id, new_location).await {
                                                Ok(_) => {
                                                    log::info!("Successfully updated location for movie {}", movie_id);
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
                                    style: "padding: 4px 12px; background: linear-gradient(135deg, #0891b2, #0e7490); color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 11px; font-weight: 600;",
                                    if is_saving() {
                                        "Saving..."
                                    } else {
                                        "Save"
                                    }
                                }
                                button {
                                    onclick: move |_| {
                                        editing.set(false);
                                        save_error.set(None);
                                    },
                                    disabled: is_saving(),
                                    style: "padding: 4px 12px; background: #374151; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 11px;",
                                    "Cancel"
                                }
                            }

                            // Show error if present
                            if let Some(error) = save_error() {
                                div {
                                    style: "color: #ef4444; font-size: 11px; margin-top: 4px;",
                                    "Error: {error}"
                                }
                            }
                        } else {
                            // Display mode
                            div {
                                style: "display: flex; gap: 8px; align-items: center;",
                                span {
                                    class: "movie-info-label",
                                    "Location: "
                                }
                                span {
                                    class: "movie-info-value",
                                    style: "font-weight: 600; color: #0891b2; flex: 1;",
                                    "{location_opt().unwrap()}"
                                }
                                button {
                                    onclick: move |_| {
                                        if let Some(loc) = location_opt() {
                                            location_input.set(loc);
                                        }
                                        editing.set(true);
                                        save_error.set(None);
                                    },
                                    style: "padding: 2px 8px; background: rgba(8, 145, 178, 0.2); color: #0891b2; border: 1px solid #0891b2; border-radius: 4px; cursor: pointer; font-size: 11px;",
                                    "Edit"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct MovieGridProps {
    pub movies: Arc<Vec<ScoredMovie>>,
}

#[component]
pub fn MovieGrid(props: MovieGridProps) -> Element {
    rsx! {
        div {
            class: "movie-grid movie-grid-cols-3",
            for scored_movie in props.movies.iter() {
                MovieCard { scored_movie: scored_movie.clone() }
            }
        }
    }
}
