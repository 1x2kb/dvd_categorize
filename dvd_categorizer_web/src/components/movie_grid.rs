use dioxus::prelude::*;
use models::ScoredMovie;
use std::sync::Arc;

#[derive(Props, Clone, PartialEq)]
pub struct MovieCardProps {
    pub movies: Arc<Vec<ScoredMovie>>,
}

#[component]
pub fn MovieGrid(props: MovieCardProps) -> Element {
    rsx! {
        div {
            class: "movie-grid movie-grid-cols-3",
            for scored_movie in props.movies.iter() {
                div {
                    class: "movie-card",

                    // Movie poster placeholder
                    div {
                        class: "movie-poster",
                        "{scored_movie.movie.name}"
                    }

                    // Movie details
                    div {
                        class: "movie-details",

                        // Title
                        h3 {
                            class: "movie-title",
                            "{scored_movie.movie.name}"
                        }

                        // Scores
                        div {
                            class: "movie-info-row",
                            style: "display: flex; gap: 15px; margin-bottom: 8px;",
                            span {
                                class: "movie-info-label",
                                style: "background: rgba(8, 145, 178, 0.2); padding: 2px 8px; border-radius: 4px; font-size: 12px;",
                                "Text: {scored_movie.text_score}"
                            }
                            span {
                                class: "movie-info-label",
                                style: "background: rgba(14, 116, 144, 0.2); padding: 2px 8px; border-radius: 4px; font-size: 12px;",
                                "Vector: {scored_movie.vector_score:.2}"
                            }
                        }

                        // Description
                        if let Some(description) = &scored_movie.movie.description {
                            p {
                                class: "movie-description",
                                "{description}"
                            }
                        }

                        // Actors
                        if !scored_movie.movie.actors.is_empty() {
                            div {
                                class: "movie-info-row",
                                span {
                                    class: "movie-info-label",
                                    "Cast: "
                                }
                                span {
                                    class: "movie-info-value",
                                    {
                                        scored_movie.movie.actors.iter().map(|a| a.name.as_str()).collect::<Vec<_>>().join(", ")
                                    }
                                }
                            }
                        }

                        // Director
                        if let Some(director) = &scored_movie.movie.director {
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
                        if !scored_movie.movie.genres.is_empty() {
                            div {
                                class: "movie-genres-container",
                                div {
                                    class: "movie-genres",
                                    // Show first 4 genres normally
                                    for genre in scored_movie.movie.genres.iter().take(4) {
                                        span {
                                            key: "{genre}",
                                            class: "movie-genre-tag",
                                            "{genre}"
                                        }
                                    }
                                    // Show additional genres (hidden by default, shown on hover)
                                    if scored_movie.movie.genres.len() > 4 {
                                        for genre in scored_movie.movie.genres.iter().skip(4) {
                                            span {
                                                key: "{genre}",
                                                class: "movie-genre-tag movie-genre-extra",
                                                "{genre}"
                                            }
                                        }
                                        span {
                                            class: "movie-genre-tag movie-genre-more",
                                            "+{scored_movie.movie.genres.len() - 4}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
