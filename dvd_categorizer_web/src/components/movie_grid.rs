// movie_grid.rs
use chrono::{DateTime, Local, NaiveDateTime};
use dioxus::prelude::*;
use models::{MovieId, ScoredMovie};

use crate::components::delete_button::DeleteButton;
use crate::components::location_editor::LocationEditor;

/// The card currently holding focus. `Some(key_hash)` puts the grid in
/// locked-focus mode: that card stays sharp, the rest blur until focus is
/// released by clicking any card.
pub fn use_focused_context() -> Signal<Option<u64>> {
    use_context()
}

#[derive(Props, Clone, PartialEq)]
pub struct SingleMovieCardProps {
    pub movies: ReadSignal<Vec<ScoredMovie>>,
    pub index: usize,
    pub search_mode: models::SearchMode,
    pub on_location_updated: EventHandler<(
        MovieId,
        String,
    )>,
    /// `false` = local-only removal (CSV preview rows aren't persisted).
    pub persist_deletes: bool,
    pub on_movie_deleted: EventHandler<(
        MovieId,
        u64,
    )>,
}

#[component]
fn MovieCard(props: SingleMovieCardProps) -> Element {
    let movies = props
        .movies
        .read();
    let Some(scored) = movies.get(props.index) else {
        return rsx! {};
    };

    let movie_id = scored
        .movie
        .id
        .clone();
    let key_hash = scored
        .movie
        .key_hash;
    let display_name = scored
        .movie
        .display_name();

    let mut focused = use_focused_context();
    let is_focused = focused
        .read()
        .as_ref()
        == Some(&key_hash);

    rsx! {
        div {
            class: if is_focused { "movie-card movie-card-focused" } else { "movie-card" },
            onclick: move |evt| {
                // Stop the page-level handler from clearing the lock we just set.
                evt.stop_propagation();
                // Locked: any card click releases the lock. Unlocked: lock this one.
                focused.with_mut(|current| {
                    *current = if current.is_some() { None } else { Some(key_hash) };
                });
            },

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
                        movie_id: movie_id.clone(),
                        initial_location: location.clone(),
                        on_location_updated: props.on_location_updated
                    }
                }

                DeleteButton {
                    key: "{movie_id}-delete",
                    movie_id,
                    key_hash,
                    persist: props.persist_deletes,
                    on_deleted: props.on_movie_deleted
                }

                if is_focused {
                    p {
                        class: "movie-focus-hint",
                        "Focus locked on this card. Click any card to unlock focus"
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
    pub on_location_updated: EventHandler<(
        MovieId,
        String,
    )>,
    /// `false` = local-only removal (CSV preview rows aren't persisted).
    pub persist_deletes: bool,
    pub on_movie_deleted: EventHandler<(
        MovieId,
        u64,
    )>,
    /// Focus lock, owned by the page so clicks outside the grid can clear it.
    pub focused: Signal<Option<u64>>,
}

#[component]
pub fn MovieGrid(props: MovieGridProps) -> Element {
    let movies_sig = props.movies;
    let search_mode = props.search_mode;
    let on_location_updated = props.on_location_updated;
    let persist_deletes = props.persist_deletes;
    let on_movie_deleted = props.on_movie_deleted;

    let is_editing = use_signal(|| false);
    use_context_provider(|| is_editing);

    let mut focused = props.focused;
    use_context_provider(|| focused);

    // Drop the lock when the visible set changes, or a stale key would leave
    // every card blurred and unclickable after a new search.
    let mut last_set = use_signal(
        || {
            (
                0usize, 0u64,
            )
        },
    );
    use_effect(
        move || {
            let current = {
                let movies = movies_sig.read();
                (
                    movies.len(),
                    movies
                        .first()
                        .map(
                            |m| {
                                m.movie
                                    .key_hash
                            },
                        )
                        .unwrap_or(0),
                )
            };
            if last_set() != current {
                last_set.set(current);
                focused.set(None);
            }
        },
    );

    let movies = movies_sig.read();
    let locked = focused
        .read()
        .is_some()
        || is_editing();

    rsx! {
        div {
            class: if locked { "movie-grid movie-grid-cols-3 editing-active" } else { "movie-grid movie-grid-cols-3" },
            for (index, movie) in movies.iter().enumerate() {
                MovieCard {
                    key: "{movie.movie.key_hash}",
                    movies: movies_sig,
                    index,
                    search_mode,
                    on_location_updated,
                    persist_deletes,
                    on_movie_deleted
                }
            }
        }
    }
}
