// movie_grid.rs
use chrono::{DateTime, Local, NaiveDateTime};
use dioxus::prelude::*;
use models::{MovieId, ScoredMovie};

use crate::components::location_editor::LocationEditor;

#[derive(Props, Clone, PartialEq)]
pub struct SingleMovieCardProps {
    pub movies: ReadSignal<Vec<ScoredMovie>>,
    pub index: usize,
    pub search_mode: models::SearchMode,
    pub on_location_updated: EventHandler<(MovieId, String)>,
}

#[component]
fn MovieCard(props: SingleMovieCardProps) -> Element {
    let movies = props.movies.read();
    let Some(scored) = movies.get(props.index) else {
        return rsx! {};
    };

    let movie_id = scored.movie.id.clone();
    let display_name = scored.movie.display_name();

    rsx! {
        div {
            class: "movie-card",

            div {
                class: "movie-poster",
                "{display_name}"
            }

            div {
                class: "movie-details",

                h3 {
                    class: "movie-title",
                    "{display_name}"
                }

                div {
                    class: "movie-info-row movie-score-container",
                    span {
                        class: "movie-info-label movie-score-badge",
                        {
                            match props.search_mode {
                                models::SearchMode::Text => format!("Text Score: {:.2}", scored.vector_score),
                                models::SearchMode::Vector => format!("Vector Score: {:.4}", scored.vector_score),
                                models::SearchMode::Both => format!("RRF Score: {:.4}", scored.vector_score),
                                models::SearchMode::Structured => format!("Relevance: {:.4}", scored.vector_score),
                            }
                        }
                    }
                }

                if let Some(description) = &scored.movie.description {
                    p {
                        class: "movie-description",
                        "{description}"
                    }
                }

                if !scored.movie.actors.is_empty() {
                    div {
                        class: "movie-info-row",
                        span {
                            class: "movie-info-label",
                            "Cast: "
                        }
                        span {
                            class: "movie-info-value",
                            {
                                scored.movie.actors.iter().map(|a| a.name.as_str()).collect::<Vec<_>>().join(", ")
                            }
                        }
                    }
                }

                if let Some(director) = &scored.movie.director {
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

                if !scored.movie.genres.is_empty() {
                    div {
                        class: "movie-genres-container",
                        div {
                            class: "movie-genres",
                            for genre in scored.movie.genres.iter().take(4) {
                                span {
                                    key: "{genre}",
                                    class: "movie-genre-tag",
                                    "{genre}"
                                }
                            }
                            if scored.movie.genres.len() > 4 {
                                for genre in scored.movie.genres.iter().skip(4) {
                                    span {
                                        key: "{genre}",
                                        class: "movie-genre-tag movie-genre-extra",
                                        "{genre}"
                                    }
                                }
                                span {
                                    class: "movie-genre-tag movie-genre-more",
                                    "+{scored.movie.genres.len() - 4}"
                                }
                            }
                        }
                    }
                }

                if let Some(added_on) = &scored.movie.added_on {
                    div {
                        class: "movie-info-row movie-added-on",
                        span {
                            class: "movie-info-label",
                            "Added: "
                        }
                        span {
                            class: "movie-info-value",
                            {
                                if let Ok(naive_dt) = NaiveDateTime::parse_from_str(added_on, "%Y-%m-%d %H:%M:%S%.f") {
                                    let utc_dt = DateTime::<chrono::Utc>::from_naive_utc_and_offset(naive_dt, chrono::Utc);
                                    let local_dt: DateTime<Local> = DateTime::from(utc_dt);
                                    local_dt.format("%B %d, %Y at %I:%M %p").to_string()
                                } else {
                                    added_on.to_string()
                                }
                            }
                        }
                    }
                }

                if let Some(location) = &scored.movie.location {
                    LocationEditor {
                        key: "{movie_id}-{location}",
                        movie_id,
                        initial_location: location.clone(),
                        on_location_updated: props.on_location_updated
                    }
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct MovieGridProps {
    pub movies: ReadSignal<Vec<ScoredMovie>>,
    pub search_mode: models::SearchMode,
    pub on_location_updated: EventHandler<(MovieId, String)>,
}

#[component]
pub fn MovieGrid(props: MovieGridProps) -> Element {
    let is_editing = use_signal(|| false);
    use_context_provider(|| is_editing);

    let movies = props.movies.read();

    rsx! {
        div {
            class: if is_editing() { "movie-grid movie-grid-cols-3 editing-active" } else { "movie-grid movie-grid-cols-3" },
            for (index, movie) in movies.iter().enumerate() {
                MovieCard {
                    key: "{movie.movie.key_hash}",
                    movies: props.movies,
                    index,
                    search_mode: props.search_mode,
                    on_location_updated: props.on_location_updated
                }
            }
        }
    }
}
