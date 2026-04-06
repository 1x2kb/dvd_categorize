use chrono::{DateTime, Local, NaiveDateTime};
use dioxus::prelude::*;
use models::ScoredMovie;
use std::sync::Arc;

use crate::components::location_editor::LocationEditor;

#[derive(Props, Clone, PartialEq)]
pub struct SingleMovieCardProps {
    pub scored_movie: ScoredMovie,
    pub search_mode: models::SearchMode,
    pub on_location_updated: EventHandler<(i32, String)>,
}

#[component]
fn MovieCard(props: SingleMovieCardProps) -> Element {
    let movie_id = props.scored_movie.movie.id;

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
                    class: "movie-info-row movie-score-container",
                    span {
                        class: "movie-info-label movie-score-badge",
                        {
                            match props.search_mode {
                                models::SearchMode::Text => format!("Text Score: {:.2}", props.scored_movie.vector_score),
                                models::SearchMode::Vector => format!("Vector Score: {:.4}", props.scored_movie.vector_score),
                                models::SearchMode::Both => format!("RRF Score: {:.4}", props.scored_movie.vector_score),
                                models::SearchMode::Structured => format!("Relevance: {:.4}", props.scored_movie.vector_score),
                            }
                        }
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
                        class: "movie-info-row movie-added-on",
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
                if let Some(ref location) = props.scored_movie.movie.location {
                    LocationEditor {
                        key: "{movie_id}-{location}",
                        movie_id: movie_id,
                        initial_location: location.clone(),
                        on_location_updated: props.on_location_updated.clone()
                    }
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct MovieGridProps {
    pub movies: Arc<Vec<ScoredMovie>>,
    pub search_mode: models::SearchMode,
    pub on_location_updated: EventHandler<(i32, String)>,
}

#[component]
pub fn MovieGrid(props: MovieGridProps) -> Element {
    rsx! {
        div {
            class: "movie-grid movie-grid-cols-3",
            for scored_movie in props.movies.iter() {
                MovieCard { 
                    key: "{scored_movie.movie.id}",
                    scored_movie: scored_movie.clone(), 
                    search_mode: props.search_mode,
                    on_location_updated: props.on_location_updated.clone()
                }
            }
        }
    }
}
