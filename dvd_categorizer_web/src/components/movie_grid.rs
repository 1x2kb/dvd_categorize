use dioxus::prelude::*;
use models::FullMovie;
use std::rc::Rc;

#[derive(Props, Clone, PartialEq)]
pub struct MovieCardProps {
    pub movie: Rc<FullMovie>,
}

#[component]
pub fn MovieGrid(props: MovieCardProps) -> Element {
    let movie = &props.movie;

    rsx! {
        div {
            class: "movie-card",

            // Movie poster placeholder
            div {
                class: "movie-poster",
                "{movie.name}"
            }

            // Movie details
            div {
                class: "movie-details",

                // Title
                h3 {
                    class: "movie-title",
                    "{movie.name}"
                }

                // Description
                if let Some(description) = &movie.description {
                    p {
                        class: "movie-description",
                        "{description}"
                    }
                }

                // Actors
                if !movie.actors.is_empty() {
                    div {
                        class: "movie-info-row",
                        span {
                            class: "movie-info-label",
                            "Cast: "
                        }
                        span {
                            class: "movie-info-value",
                            {
                                movie.actors.iter().map(|a| a.name.as_str()).collect::<Vec<_>>().join(", ")
                            }
                        }
                    }
                }

                // Director
                if let Some(director) = &movie.director {
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
                if !movie.genres.is_empty() {
                    div {
                        class: "movie-genres-container",
                        div {
                            class: "movie-genres",
                            // Show first 4 genres normally
                            for genre in movie.genres.iter().take(4) {
                                span {
                                    key: "{genre}",
                                    class: "movie-genre-tag",
                                    "{genre}"
                                }
                            }
                            // Show additional genres (hidden by default, shown on hover)
                            if movie.genres.len() > 4 {
                                for genre in movie.genres.iter().skip(4) {
                                    span {
                                        key: "{genre}",
                                        class: "movie-genre-tag movie-genre-extra",
                                        "{genre}"
                                    }
                                }
                                span {
                                    class: "movie-genre-tag movie-genre-more",
                                    "+{movie.genres.len() - 4}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
