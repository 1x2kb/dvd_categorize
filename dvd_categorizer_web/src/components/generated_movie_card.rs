use dioxus::prelude::*;
use models::AiMovieData;

// ---------------------------------------------------------------------------
// Read-only card — matches movie-card style, single card-level lock
// ---------------------------------------------------------------------------

#[derive(Props, Clone, PartialEq)]
pub struct GeneratedMovieCardProps {
    pub movie: AiMovieData,
    pub locked: bool,
    pub on_lock_toggle: EventHandler<bool>,
    pub on_edit: EventHandler<()>,
}

#[component]
pub fn GeneratedMovieCard(props: GeneratedMovieCardProps) -> Element {
    rsx! {
        div {
            class: if props.locked { "movie-card gen-card-locked" } else { "movie-card" },
            onclick: move |_| props.on_lock_toggle.call(!props.locked),
            style: "cursor: pointer;",

            div {
                class: "movie-poster",
                "{props.movie.title}"
                if props.movie.year > 0 {
                    span { class: "gen-poster-year", " ({props.movie.year})" }
                }
                if props.movie.already_in_catalog {
                    span { class: "gen-catalog-badge", "Already in catalog" }
                }
            }

            div {
                class: "movie-details",

                h3 { class: "movie-title", "{props.movie.title}" }

                if !props.movie.description.is_empty() {
                    p { class: "movie-description", "{props.movie.description}" }
                }

                if !props.movie.actors.is_empty() {
                    div { class: "movie-info-row",
                        span { class: "movie-info-label", "CAST: " }
                        span { class: "movie-info-value", "{props.movie.actors.join(\", \")}" }
                    }
                }

                if !props.movie.director.is_empty() {
                    div { class: "movie-info-row",
                        span { class: "movie-info-label", "DIRECTOR: " }
                        span { class: "movie-info-value", "{props.movie.director}" }
                    }
                }

                if !props.movie.genres.is_empty() {
                    div { class: "movie-genres-container",
                        div { class: "movie-genres",
                            for genre in props.movie.genres.iter() {
                                span { class: "movie-genre-tag", "{genre}" }
                            }
                        }
                    }
                }

                div { class: "gen-card-actions",
                    button {
                        class: "gen-btn gen-btn-edit",
                        onclick: move |evt| {
                            evt.stop_propagation();
                            props.on_edit.call(());
                        },
                        "✏ Edit"
                    }
                    button {
                        class: if props.locked { "gen-btn gen-btn-lock gen-btn-lock-active" } else { "gen-btn gen-btn-lock" },
                        title: if props.locked { "Locked — will not be regenerated" } else { "Unlocked — will be regenerated on next Generate" },
                        onclick: move |evt| {
                            evt.stop_propagation();
                            props.on_lock_toggle.call(!props.locked);
                        },
                        if props.locked { "🔒 Locked" } else { "🔓 Lock" }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Editable card — inputs for all fields, same movie-card shell
// ---------------------------------------------------------------------------

#[derive(Props, Clone, PartialEq)]
pub struct EditableMovieCardProps {
    pub movie: AiMovieData,
    pub on_save: EventHandler<AiMovieData>,
    pub on_cancel: EventHandler<()>,
}

#[component]
pub fn EditableMovieCard(props: EditableMovieCardProps) -> Element {
    let mut title = use_signal(|| props.movie.title.clone());
    let mut year = use_signal(|| props.movie.year.to_string());
    let mut director = use_signal(|| props.movie.director.clone());
    let mut actors = use_signal(|| props.movie.actors.join(", "));
    let mut genres = use_signal(|| props.movie.genres.join(", "));
    let mut description = use_signal(|| props.movie.description.clone());

    rsx! {
        div {
            class: "movie-card gen-card-editing",

            div {
                class: "movie-poster gen-card-poster-editing",
                "{title}"
                span { class: "gen-poster-edit-badge", " — EDITING" }
            }

            div {
                class: "movie-details",

                // Title
                div { class: "movie-info-row",
                    span { class: "movie-info-label", "TITLE: " }
                    input {
                        class: "gen-edit-input",
                        r#type: "text",
                        value: "{title}",
                        oninput: move |e| title.set(e.value()),
                    }
                }

                // Year
                div { class: "movie-info-row",
                    span { class: "movie-info-label", "YEAR: " }
                    input {
                        class: "gen-edit-input gen-edit-input-short",
                        r#type: "number",
                        value: "{year}",
                        oninput: move |e| year.set(e.value()),
                    }
                }

                // Director
                div { class: "movie-info-row",
                    span { class: "movie-info-label", "DIRECTOR: " }
                    input {
                        class: "gen-edit-input",
                        r#type: "text",
                        value: "{director}",
                        oninput: move |e| director.set(e.value()),
                    }
                }

                // Cast
                div { class: "movie-info-row",
                    span { class: "movie-info-label", "CAST: " }
                    input {
                        class: "gen-edit-input",
                        r#type: "text",
                        value: "{actors}",
                        placeholder: "Comma-separated",
                        oninput: move |e| actors.set(e.value()),
                    }
                }

                // Genres
                div { class: "movie-info-row",
                    span { class: "movie-info-label", "GENRES: " }
                    input {
                        class: "gen-edit-input",
                        r#type: "text",
                        value: "{genres}",
                        placeholder: "Comma-separated",
                        oninput: move |e| genres.set(e.value()),
                    }
                }

                // Description
                div { class: "movie-info-row gen-description-row",
                    span { class: "movie-info-label", "DESC: " }
                    textarea {
                        class: "gen-edit-textarea",
                        value: "{description}",
                        rows: "3",
                        oninput: move |e| description.set(e.value()),
                    }
                }

                // Actions
                div { class: "gen-card-actions",
                    button {
                        class: "gen-btn gen-btn-save",
                        onclick: move |_| {
                            props.on_save.call(AiMovieData {
                                title: title(),
                                year: year().parse::<i32>().unwrap_or(0),
                                description: description(),
                                actors: actors().split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect(),
                                genres: genres().split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect(),
                                director: director(),
                                already_in_catalog: false,
                                input_title: None,
                                position: 0,
                            });
                        },
                        "✓ Save"
                    }
                    button {
                        class: "gen-btn gen-btn-cancel",
                        onclick: move |_| props.on_cancel.call(()),
                        "✕ Cancel"
                    }
                }
            }
        }
    }
}
