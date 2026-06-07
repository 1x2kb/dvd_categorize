use crate::components::generated_movie_card::{EditableMovieCard, GeneratedMovieCard};
use crate::components::toast::{ToastKind, ToastMessage};
use crate::views::movie_generator::stream_generated_movies;
use dioxus::prelude::*;
use models::{AiMovieData, FullMovie, NeedsInputReason};

/// Why a pending card is waiting for user input.
#[derive(Debug, Clone, PartialEq)]
pub enum PendingReason {
    /// AI suggested a corrected title — waiting for user decision.
    Suggestion(String),
    /// User made a decision; holds the resolved title to generate.
    /// Waiting for all other pending cards to resolve before firing.
    Resolved(String),
}

impl From<NeedsInputReason> for PendingReason {
    fn from(r: NeedsInputReason) -> Self {
        match r {
            NeedsInputReason::Suggestion(s) => PendingReason::Suggestion(s),
        }
    }
}

/// Per-movie state tracked by the grid.
#[derive(Debug, Clone, PartialEq)]
pub struct MovieEntry {
    pub data: AiMovieData,
    /// The title the user originally typed — used for lock matching across re-generates.
    pub input_title: String,
    /// When true, this card is preserved on the next Generate.
    pub locked: bool,
    pub editing: bool,
    /// True while this card's generation stream is in-flight.
    pub generating: bool,
    /// Set when the backend says this title needs user input before generating.
    pub pending: Option<PendingReason>,
}

impl MovieEntry {
    pub fn new(input_title: String, data: AiMovieData) -> Self {
        Self {
            input_title,
            data,
            locked: false,
            editing: false,
            generating: false,
            pending: None,
        }
    }

    pub fn new_pending(input_title: String, reason: PendingReason) -> Self {
        let placeholder = AiMovieData {
            title: input_title.clone(),
            year: 0,
            description: String::new(),
            actors: vec![],
            genres: vec![],
            director: String::new(),
            already_in_catalog: false,
            input_title: Some(input_title.clone()),
            position: 0,
        };
        Self {
            input_title: input_title.clone(),
            data: placeholder,
            locked: false,
            editing: false,
            generating: false,
            pending: Some(reason),
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct GeneratedMovieGridProps {
    /// Live signal of generated movies — the grid subscribes to append new entries.
    pub movies: ReadSignal<Vec<AiMovieData>>,
    /// Titles that need user input — streamed as NeedsInput SSE events.
    pub pending_inputs: ReadSignal<
        Vec<(
            String,
            NeedsInputReason,
        )>,
    >,
    /// The raw titles the user typed — parallel to movies.
    pub input_titles: Vec<String>,
    /// Incremented by the parent to trigger a re-generate of unlocked cards.
    pub generate_trigger: ReadSignal<u32>,
    /// Model name to use for generation.
    pub model: String,
    /// Called whenever movie data changes (edit save or regenerate).
    pub on_movies_changed: EventHandler<Vec<AiMovieData>>,
    /// Called when generation starts/finishes so parent can show a spinner.
    pub on_loading: EventHandler<bool>,
    /// Called with any error string.
    pub on_error: EventHandler<String>,
    /// Called when a title during re-generate needs user input.
    pub on_pending: EventHandler<(
        String,
        NeedsInputReason,
    )>,
    /// Push a toast notification to the parent's toast stack.
    pub on_toast: EventHandler<ToastMessage>,
    /// Called when user accepts a suggestion — (original, corrected) so parent can update chips.
    pub on_title_corrected: EventHandler<(
        String,
        String,
    )>,
    /// Called when a card is removed — parent should remove the matching chip.
    pub on_title_removed: EventHandler<String>,
    /// Called to dismiss a toast by id.
    pub on_dismiss_toast: EventHandler<u32>,
}

#[component]
pub fn GeneratedMovieGrid(props: GeneratedMovieGridProps) -> Element {
    let mut entries: Signal<Vec<MovieEntry>> = use_signal(Vec::new);
    let mut next_toast_id: Signal<u32> = use_signal(|| 100u32);
    let mut pending_toast_id: Signal<Option<u32>> = use_signal(|| None);

    let on_toast_pending = props.on_toast;
    let on_dismiss_pending = props.on_dismiss_toast;

    // React to generate_trigger increments — reconcile entries with current titles,
    // then stream-regenerate all unlocked cards in-place.
    // Auto-fire: when all pending cards have been resolved (no Suggestion remaining),
    // batch-generate all Resolved titles at once.
    let on_toast_auto = props.on_toast;
    let on_dismiss_auto = props.on_dismiss_toast;
    let model_auto = props.model.clone();
    let on_error_auto = props.on_error;
    let on_loading_auto = props.on_loading;
    use_effect(
        move || {
            let snapshot = entries.read(); // reactive
            let has_suggestion = snapshot
                .iter()
                .any(
                    |e| {
                        matches!(
                            e.pending,
                            Some(PendingReason::Suggestion(_))
                        )
                    },
                );
            let resolved: Vec<(
                usize,
                String,
            )> = snapshot
                .iter()
                .enumerate()
                .filter_map(
                    |(i, e)| {
                        if let Some(PendingReason::Resolved(t)) = &e.pending {
                            Some((
                                i,
                                t.clone(),
                            ))
                        } else {
                            None
                        }
                    },
                )
                .collect();
            drop(snapshot);
            if !has_suggestion && !resolved.is_empty() {
                // Dismiss the sticky pending toast.
                let maybe_tid = *pending_toast_id.peek();
                if let Some(tid) = maybe_tid {
                    on_dismiss_auto.call(tid);
                    pending_toast_id.set(None);
                }
                // All pending decided — fire the batch.
                entries.with_mut(
                    |v| {
                        for (i, _) in &resolved {
                            if let Some(e) = v.get_mut(*i) {
                                e.pending = None;
                                e.generating = true;
                            }
                        }
                    },
                );
                let id = *next_toast_id.peek();
                next_toast_id.with_mut(|n| *n += 1);
                on_toast_auto.call(
                    ToastMessage {
                        id,
                        kind: ToastKind::Info,
                        message: format!(
                            "Generating {} pending title{}…",
                            resolved.len(),
                            if resolved.len() == 1 { "" } else { "s" }
                        ),
                        duration_ms: 3000,
                    },
                );
                let model = model_auto.clone();
                let on_error = on_error_auto;
                let on_loading = on_loading_auto;
                let on_toast = on_toast_auto;
                let mut next_id = next_toast_id;
                spawn(
                    async move {
                        on_loading.call(true);
                        let titles: Vec<String> = resolved
                            .iter()
                            .map(|(_, t)| t.clone())
                            .collect();
                        let positions: Vec<usize> = resolved
                            .iter()
                            .map(|(i, _)| *i)
                            .collect();
                        stream_generated_movies(
                            titles,
                            positions,
                            model,
                            move |movie, _| {
                                let pos = movie.position;
                                entries.with_mut(
                                    |v| {
                                        if let Some(e) = v.get_mut(pos) {
                                            e.data = movie;
                                            e.generating = false;
                                        }
                                    },
                                );
                            },
                            move || {
                                let id = *next_id.peek();
                                next_id.with_mut(|n| *n += 1);
                                on_toast.call(
                                    ToastMessage {
                                        id,
                                        kind: ToastKind::Success,
                                        message: "All pending titles generated.".to_string(),
                                        duration_ms: 3000,
                                    },
                                );
                            },
                            move |e| on_error.call(e),
                        )
                        .await;
                        on_loading.call(false);
                    },
                );
            }
        },
    );

    let trigger_signal = props.generate_trigger;
    let model = props.model.clone();
    let on_loading = props.on_loading;
    let on_error = props.on_error;
    let on_pending = props.on_pending;
    let on_title_corrected = props.on_title_corrected;
    let input_titles_regen = props.input_titles.clone();
    use_effect(
        move || {
            let trigger = *trigger_signal.read(); // reactive subscription
            if trigger == 0 {
                return;
            } // skip initial mount

            // Pre-allocate exactly one skeleton slot per chip, in chip order with position = chip index.
            entries.with_mut(
                |v| {
                    // Remove slots whose chip was deleted.
                    v.retain(
                        |e| {
                            input_titles_regen
                                .iter()
                                .any(
                                    |t| {
                                        t.to_lowercase()
                                            == e.input_title
                                                .to_lowercase()
                                    },
                                )
                        },
                    );
                    // Rebuild to exactly match chip list, preserving locked entries.
                    let new_v: Vec<MovieEntry> = input_titles_regen
                        .iter()
                        .enumerate()
                        .map(
                            |(pos, t)| {
                                if let Some(existing) = v
                                    .iter()
                                    .find(
                                        |e| {
                                            e.input_title
                                                .to_lowercase()
                                                == t.to_lowercase()
                                        },
                                    )
                                {
                                    let mut e = existing.clone();
                                    e.data
                                        .position = pos; // ensure position is canonical chip index
                                    e
                                } else {
                                    let placeholder = AiMovieData {
                                        title: t.clone(),
                                        year: 0,
                                        description: String::new(),
                                        actors: vec![],
                                        genres: vec![],
                                        director: String::new(),
                                        already_in_catalog: false,
                                        input_title: Some(t.clone()),
                                        position: pos,
                                    };
                                    MovieEntry::new(
                                        t.clone(),
                                        placeholder,
                                    )
                                }
                            },
                        )
                        .collect();
                    *v = new_v;
                },
            );

            // Collect unlocked slots to regenerate.
            let to_generate: Vec<(
                usize,
                String,
            )> = entries
                .peek()
                .iter()
                .enumerate()
                .filter(|(_, e)| !e.locked)
                .map(
                    |(i, e)| {
                        (
                            i,
                            e.input_title
                                .clone(),
                        )
                    },
                )
                .collect();
            if to_generate.is_empty() {
                return;
            }

            // Mark unlocked cards as generating.
            entries.with_mut(
                |v| {
                    for (i, _) in &to_generate {
                        v[*i].generating = true;
                    }
                },
            );
            on_loading.call(true);

            let model = model.clone();
            let on_error = on_error;
            let on_loading = on_loading;
            let on_toast_cb = on_toast_pending;
            let on_dismiss_cb = on_dismiss_pending;
            spawn(
                async move {
                    let titles: Vec<String> = to_generate
                        .iter()
                        .map(|(_, t)| t.clone())
                        .collect();
                    let positions: Vec<usize> = to_generate
                        .iter()
                        .map(|(i, _)| *i)
                        .collect();
                    stream_generated_movies(
                        titles,
                        positions,
                        model,
                        move |movie, pending| {
                            let pos = movie.position;
                            if let Some(reason) = pending {
                                // Mark the pre-allocated slot at this position as pending.
                                entries.with_mut(
                                    |v| {
                                        if let Some(e) = v.get_mut(pos) {
                                            e.generating = false;
                                            e.pending = Some(PendingReason::from(reason.clone()));
                                        }
                                    },
                                );
                                // Update the sticky warning toast with all pending titles.
                                let old_id = *pending_toast_id.peek();
                                if let Some(tid) = old_id {
                                    on_dismiss_cb.call(tid);
                                }
                                let all_pending_titles: Vec<String> = entries
                                    .peek()
                                    .iter()
                                    .filter(
                                        |e| {
                                            matches!(
                                                e.pending,
                                                Some(PendingReason::Suggestion(_))
                                            )
                                        },
                                    )
                                    .map(
                                        |e| {
                                            format!(
                                                "\"{}\"",
                                                e.input_title
                                            )
                                        },
                                    )
                                    .collect();
                                let new_id = *next_toast_id.peek();
                                next_toast_id.with_mut(|n| *n += 1);
                                pending_toast_id.set(Some(new_id));
                                on_toast_cb.call(
                                    ToastMessage {
                                        id: new_id,
                                        kind: ToastKind::Warning,
                                        message: format!(
                                            "Needs input: {}",
                                            all_pending_titles.join(", ")
                                        ),
                                        duration_ms: 0,
                                    },
                                );
                                on_pending.call((
                                    movie
                                        .input_title
                                        .unwrap_or(movie.title),
                                    reason,
                                ));
                                return;
                            }
                            // Fill the pre-allocated slot at this position.
                            entries.with_mut(
                                |v| {
                                    if let Some(entry) = v.get_mut(pos) {
                                        entry.data = movie;
                                        entry.generating = false;
                                    }
                                },
                            );
                        },
                        || {},
                        |e| {
                            on_error.call(e);
                        },
                    )
                    .await;
                    on_loading.call(false);
                },
            );
        },
    );

    let snapshot = entries();
    let all_locked = snapshot
        .iter()
        .filter(
            |e| {
                !e.data
                    .already_in_catalog
            },
        )
        .all(|e| e.locked);
    let any_in_catalog = snapshot
        .iter()
        .any(
            |e| {
                e.data
                    .already_in_catalog
            },
        );
    let is_generating = snapshot
        .iter()
        .any(|e| e.generating);
    drop(snapshot);

    rsx! {
        div { class: "gen-grid-toolbar",
            button {
                class: "gen-btn gen-btn-lock-all",
                disabled: is_generating,
                onclick: move |_| handle_lock_all_toggle(
                    is_generating,
                    all_locked,
                    entries,
                    next_toast_id,
                    props.on_toast,
                ),
                if all_locked { "🔓 Unlock All" } else { "🔒 Lock All" }
            }
            {
                let saveable: Vec<(String, AiMovieData)> = entries().iter()
                    .filter(|e| !e.data.already_in_catalog && e.pending.is_none() && !e.generating && e.data.year > 0)
                    .map(|e| (e.input_title.clone(), e.data.clone()))
                    .collect();
                let save_all_disabled = is_generating || saveable.is_empty();
                rsx! {
                    button {
                        class: if save_all_disabled { "gen-btn gen-btn-save gen-btn-disabled" } else { "gen-btn gen-btn-save" },
                        disabled: save_all_disabled,
                        onclick: move |_| {
                            spawn(handle_save_all(
                                saveable.clone(),
                                entries,
                                next_toast_id,
                                props.on_toast,
                            ));
                        },
                        "💾 Save All"
                    }
                }
            }
            if any_in_catalog {
                {rsx! {
                    button {
                        class: "gen-btn gen-btn-remove-catalog",
                        disabled: is_generating,
                        onclick: move |_| handle_remove_catalog_items(
                            is_generating,
                            entries,
                            next_toast_id,
                            props.on_toast,
                        ),
                        "🗑 Remove Already in DB"
                    }
                }}
            }
        }

        div {
            class: "gen-movie-grid",

            for (idx, entry) in entries().iter().enumerate() {
                div {
                    key: "{idx}-{entry.data.title}",
                    class: "gen-movie-grid-item",

                    if entry.generating || (entry.data.year == 0 && entry.data.description.is_empty() && is_generating && entry.pending.is_none()) {
                        div { class: "movie-card gen-card-regenerating",
                            div { class: "movie-poster", "{entry.data.title}" }
                            div { class: "movie-details gen-regen-overlay",
                                div { class: "spinner" }
                                p { "Generating..." }
                            }
                        }
                    } else if let Some(reason) = entry.pending.clone() {
                        {
                            let original_title = entry.input_title.clone();

                            match reason {
                                PendingReason::Suggestion(sug) => {
                                    // Clone all values needed by closures here, bound to this
                                    // specific iteration — avoids stale idx / last-value capture.
                                    let sug_display = sug.clone();
                                    let key = original_title.clone(); // stable identity for lookups
                                    let sug_for_accept = sug.clone();
                                    let key_accept = key.clone();
                                    let key_keep = key.clone();
                                    let key_remove = key.clone();
                                    let on_corrected = on_title_corrected;
                                    rsx! {
                                        div { class: "movie-card gen-card-pending",
                                            div { class: "movie-poster gen-poster-suggestion", "{original_title}" }
                                            div { class: "movie-details gen-pending-body",
                                                p { class: "gen-pending-msg",
                                                    "Did you mean "
                                                    strong { "{sug_display}" }
                                                    "?"
                                                }
                                                div { class: "gen-pending-actions",
                                                    button {
                                                        class: "gen-btn gen-btn-accept",
                                                        onclick: move |_| {
                                                            on_corrected.call((key_accept.clone(), sug_for_accept.clone()));
                                                            entries.with_mut(|v| {
                                                                if let Some(e) = v.iter_mut().find(|e| e.input_title == key_accept) {
                                                                    e.input_title = sug_for_accept.clone();
                                                                    e.data.title = sug_for_accept.clone();
                                                                    e.pending = Some(PendingReason::Resolved(sug_for_accept.clone()));
                                                                }
                                                            });
                                                        },
                                                        "✓ Yes, use \"{sug_display}\""
                                                    }
                                                    button {
                                                        class: "gen-btn gen-btn-secondary",
                                                        onclick: move |_| entries.with_mut(|v| {
                                                            if let Some(e) = v.iter_mut().find(|e| e.input_title == key_keep) {
                                                                e.pending = Some(PendingReason::Resolved(key_keep.clone()));
                                                            }
                                                        }),
                                                        "Keep original"
                                                    }
                                                    button {
                                                        class: "gen-btn gen-btn-remove",
                                                        onclick: move |_| entries.with_mut(|v| {
                                                            v.retain(|e| e.input_title != key_remove);
                                                        }),
                                                        "✕ Remove"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                },
                                PendingReason::Resolved(_) => {
                                    rsx! {
                                        div { class: "movie-card gen-card-regenerating",
                                            div { class: "movie-poster gen-poster-suggestion", "{original_title}" }
                                            div { class: "movie-details gen-regen-overlay",
                                                div { class: "spinner" }
                                                p { "Waiting for others…" }
                                            }
                                        }
                                    }
                                },
                            }
                        }
                    } else if entry.editing {
                        {
                            let edit_key = entry.input_title.clone();
                            let edit_key2 = edit_key.clone();
                            rsx! { EditableMovieCard {
                                movie: entry.data.clone(),
                                on_save: {
                                    let on_changed = props.on_movies_changed;
                                    move |updated: AiMovieData| {
                                        entries.with_mut(|v| {
                                            if let Some(e) = v.iter_mut().find(|e| e.input_title == edit_key) {
                                                e.data = updated;
                                                e.editing = false;
                                            }
                                        });
                                        let snapshot: Vec<AiMovieData> =
                                            entries().iter().map(|e| e.data.clone()).collect();
                                        on_changed.call(snapshot);
                                    }
                                },
                                on_cancel: move |_| {
                                    entries.with_mut(|v| {
                                        if let Some(e) = v.iter_mut().find(|e| e.input_title == edit_key2) {
                                            e.editing = false;
                                        }
                                    });
                                },
                            } }
                        }
                    } else {
                        {
                            let lock_key = entry.input_title.clone();
                            let edit_key = entry.input_title.clone();
                            let remove_key = entry.input_title.clone();
                            let on_toast_card = props.on_toast;
                            let mut next_id_card = next_toast_id;
                            rsx! { GeneratedMovieCard {
                                movie: entry.data.clone(),
                                locked: entry.locked,
                                disabled: is_generating,
                                on_lock_toggle: move |new_locked: bool| {
                                    entries.with_mut(|v| {
                                        if let Some(e) = v.iter_mut().find(|e| e.input_title == lock_key) {
                                            e.locked = new_locked;
                                        }
                                    });
                                },
                                on_edit: move |_| {
                                    entries.with_mut(|v| {
                                        if let Some(e) = v.iter_mut().find(|e| e.input_title == edit_key) {
                                            e.editing = true;
                                        }
                                    });
                                },
                                on_remove: move |_| {
                                    entries.with_mut(|v| v.retain(|e| e.input_title != remove_key));
                                    props.on_title_removed.call(remove_key.clone());
                                },
                                on_save: {
                                    let on_toast_save = props.on_toast;
                                    let save_key = entry.input_title.clone();
                                    move |movie: AiMovieData| {
                                        let save_key = save_key.clone();
                                        let on_toast = on_toast_save;
                                        let mut next_id = next_toast_id;
                                        spawn(async move {
                                            let Ok(base) = crate::views::movie_generator::get_api_base().await else {
                                                let id = *next_id.peek();
                                                next_id.with_mut(|n| *n += 1);
                                                on_toast.call(ToastMessage {
                                                    id,
                                                    kind: ToastKind::Warning,
                                                    message: "Failed to get API base URL".to_string(),
                                                    duration_ms: 3000,
                                                });
                                                return;
                                            };
                                            let id = *next_id.peek();
                                            next_id.with_mut(|n| *n += 1);
                                            let msg = match save_movie_to_catalog(movie, base).await {
                                                Ok(saved) => {
                                                    entries.with_mut(|v| v.retain(|e| e.input_title != save_key));
                                                    ToastMessage {
                                                        id,
                                                        kind: ToastKind::Success,
                                                        message: format!("'{}' saved to catalog", saved.name),
                                                        duration_ms: 4000,
                                                    }
                                                }
                                                Err(e) => ToastMessage {
                                                    id,
                                                    kind: ToastKind::Warning,
                                                    message: format!("Save failed: {}", e),
                                                    duration_ms: 5000,
                                                },
                                            };
                                            on_toast.call(msg);
                                        });
                                    }
                                },
                                on_disabled_click: move |_| {
                                    let id = *next_id_card.peek();
                                    next_id_card.with_mut(|n| *n += 1);
                                    on_toast_card.call(ToastMessage {
                                        id,
                                        kind: ToastKind::Warning,
                                        message: "Card buttons disabled until generation completes".to_string(),
                                        duration_ms: 3000,
                                    });
                                },
                            } }
                        }
                    }
                }
            }
        }
    }
}

/// POST /saveMovie — saves an AiMovieData to the catalog, returns the saved FullMovie.
async fn save_movie_to_catalog(movie: AiMovieData, base: String) -> Result<FullMovie, String> {
    let body = serde_json::to_string(&movie).map_err(|e| e.to_string())?;
    let response = gloo_net::http::Request::post(
        &format!(
            "{}/saveMovie",
            base
        ),
    )
    .header(
        "Content-Type",
        "application/json",
    )
    .body(body)
    .map_err(|e| e.to_string())?
    .send()
    .await
    .map_err(|e| e.to_string())?;
    if !response.ok() {
        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_default();
        return Err(
            format!(
                "Server error {}: {}",
                status, body
            ),
        );
    }
    response
        .json::<FullMovie>()
        .await
        .map_err(|e| e.to_string())
}

/// Handle Lock All / Unlock All button click.
fn handle_lock_all_toggle(
    is_generating: bool,
    all_locked: bool,
    mut entries: Signal<Vec<MovieEntry>>,
    mut next_toast_id: Signal<u32>,
    on_toast: EventHandler<ToastMessage>,
) {
    if is_generating {
        let id = *next_toast_id.peek();
        next_toast_id.with_mut(|n| *n += 1);
        on_toast.call(
            ToastMessage {
                id,
                kind: ToastKind::Warning,
                message: "Card buttons disabled until generation completes".to_string(),
                duration_ms: 3000,
            },
        );
        return;
    }
    entries.with_mut(
        |v| {
            v.iter_mut()
                .filter(
                    |e| {
                        !e.data
                            .already_in_catalog
                    },
                )
                .for_each(|e| e.locked = !all_locked)
        },
    );
}

/// Handle Remove Already in DB button click — removes all catalog items from the grid.
fn handle_remove_catalog_items(
    is_generating: bool,
    mut entries: Signal<Vec<MovieEntry>>,
    mut next_toast_id: Signal<u32>,
    on_toast: EventHandler<ToastMessage>,
) {
    if is_generating {
        let id = *next_toast_id.peek();
        next_toast_id.with_mut(|n| *n += 1);
        on_toast.call(
            ToastMessage {
                id,
                kind: ToastKind::Warning,
                message: "Card buttons disabled until generation completes".to_string(),
                duration_ms: 3000,
            },
        );
        return;
    }
    let removed = entries
        .peek()
        .iter()
        .filter(
            |e| {
                e.data
                    .already_in_catalog
            },
        )
        .count();
    entries.with_mut(
        |v| {
            v.retain(
                |e| {
                    !e.data
                        .already_in_catalog
                },
            )
        },
    );
    let id = *next_toast_id.peek();
    next_toast_id.with_mut(|n| *n += 1);
    on_toast.call(
        ToastMessage {
            id,
            kind: ToastKind::Info,
            message: format!(
                "Removed {} item{} already in catalog",
                removed,
                if removed == 1 { "" } else { "s" }
            ),
            duration_ms: 3000,
        },
    );
}

/// Handle Save All button click — saves all saveable movies to catalog.
async fn handle_save_all(
    items: Vec<(
        String,
        AiMovieData,
    )>,
    mut entries: Signal<Vec<MovieEntry>>,
    mut next_toast_id: Signal<u32>,
    on_toast: EventHandler<ToastMessage>,
) {
    let base = match crate::views::movie_generator::get_api_base().await {
        Ok(b) => b,
        Err(_) => {
            let id = *next_toast_id.peek();
            next_toast_id.with_mut(|n| *n += 1);
            on_toast.call(
                ToastMessage {
                    id,
                    kind: ToastKind::Warning,
                    message: "Failed to get API base URL".to_string(),
                    duration_ms: 3000,
                },
            );
            return;
        }
    };
    let mut saved_count = 0usize;
    let mut failed: Vec<String> = vec![];
    for (key, movie) in items {
        let title = movie
            .title
            .clone();
        match save_movie_to_catalog(
            movie,
            base.clone(),
        )
        .await
        {
            Ok(_) => {
                saved_count += 1;
                entries.with_mut(|v| v.retain(|e| e.input_title != key));
            }
            Err(e) => failed.push(
                format!(
                    "{}: {}",
                    title, e
                ),
            ),
        }
    }
    let id = *next_toast_id.peek();
    next_toast_id.with_mut(|n| *n += 1);
    if failed.is_empty() {
        on_toast.call(
            ToastMessage {
                id,
                kind: ToastKind::Success,
                message: format!(
                    "{} movie{} saved to catalog",
                    saved_count,
                    if saved_count == 1 { "" } else { "s" }
                ),
                duration_ms: 4000,
            },
        );
    } else {
        on_toast.call(
            ToastMessage {
                id,
                kind: ToastKind::Warning,
                message: format!(
                    "{} saved, {} failed: {}",
                    saved_count,
                    failed.len(),
                    failed.join(", ")
                ),
                duration_ms: 6000,
            },
        );
    }
}

/// Call this from the parent once a regenerate API response comes back.
pub fn apply_regenerate_result(entries: &mut [MovieEntry], index: usize, result: AiMovieData) {
    if let Some(entry) = entries.get_mut(index) {
        entry.data = result;
        entry.generating = false;
    }
}
