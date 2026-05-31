use dioxus::prelude::*;
use models::AiMovieData;
use crate::components::generated_movie_card::{EditableMovieCard, GeneratedMovieCard};
use crate::views::movie_generator::stream_generated_movies;

/// Per-movie state tracked by the grid.
#[derive(Debug, Clone, PartialEq)]
pub struct MovieEntry {
    pub data: AiMovieData,
    /// The title the user originally typed — used for lock matching across re-generates.
    pub input_title: String,
    /// When true, this card is preserved on the next Generate.
    pub locked: bool,
    pub editing: bool,
    /// True while a per-movie regenerate request is in-flight.
    pub regenerating: bool,
}

impl MovieEntry {
    pub fn new(input_title: String, data: AiMovieData) -> Self {
        Self {
            input_title,
            data,
            locked: false,
            editing: false,
            regenerating: false,
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct GeneratedMovieGridProps {
    /// Live signal of generated movies — the grid subscribes to append new entries.
    pub movies: ReadSignal<Vec<AiMovieData>>,
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
}

#[component]
pub fn GeneratedMovieGrid(props: GeneratedMovieGridProps) -> Element {
    let mut entries: Signal<Vec<MovieEntry>> = use_signal(|| vec![]);

    // Subscribe to the movies signal — append any entries not yet in the grid.
    // Using the signal directly means this effect re-runs on every push.
    // entries.peek() is non-reactive so writing to entries doesn't loop.
    let incoming_titles = props.input_titles.clone();
    let movies_signal = props.movies;
    use_effect(move || {
        let incoming = movies_signal.read(); // reactive read — subscribes to the signal
        let current_len = entries.peek().len();
        if incoming.len() > current_len {
            entries.with_mut(|v| {
                for i in current_len..incoming.len() {
                    let m = &incoming[i];
                    let input = incoming_titles.get(i).cloned().unwrap_or_else(|| m.title.clone());
                    v.push(MovieEntry::new(input, m.clone()));
                }
            });
        }
    });

    // React to generate_trigger increments — reconcile entries with current titles,
    // then stream-regenerate all unlocked cards in-place.
    let trigger_signal = props.generate_trigger;
    let model = props.model.clone();
    let on_loading = props.on_loading.clone();
    let on_error = props.on_error.clone();
    let input_titles_regen = props.input_titles.clone();
    use_effect(move || {
        let trigger = *trigger_signal.read(); // reactive subscription
        if trigger == 0 { return; } // skip initial mount

        // Reconcile: add/remove entries to match the current title list.
        entries.with_mut(|v| {
            // Remove entries whose input_title is no longer in the list.
            v.retain(|e| input_titles_regen.iter().any(|t| t.to_lowercase() == e.input_title.to_lowercase()));
            // Add placeholder entries for new titles not yet in entries.
            for t in &input_titles_regen {
                if !v.iter().any(|e| e.input_title.to_lowercase() == t.to_lowercase()) {
                    let placeholder = AiMovieData {
                        title: t.clone(),
                        year: 0,
                        description: String::new(),
                        actors: vec![],
                        genres: vec![],
                        director: String::new(),
                    };
                    v.push(MovieEntry::new(t.clone(), placeholder));
                }
            }
            // Re-sort to match input_titles order.
            let order = input_titles_regen.clone();
            v.sort_by_key(|e| order.iter().position(|t| t.to_lowercase() == e.input_title.to_lowercase()).unwrap_or(usize::MAX));
        });

        // Collect unlocked slots to regenerate.
        let to_generate: Vec<(usize, String)> = entries.peek()
            .iter()
            .enumerate()
            .filter(|(_, e)| !e.locked)
            .map(|(i, e)| (i, e.input_title.clone()))
            .collect();
        if to_generate.is_empty() { return; }

        // Mark unlocked cards as regenerating.
        entries.with_mut(|v| {
            for (i, _) in &to_generate { v[*i].regenerating = true; }
        });
        on_loading.call(true);

        let model = model.clone();
        let on_error = on_error.clone();
        let on_loading = on_loading.clone();
        spawn(async move {
            // Stream one at a time, patching each entry as it arrives.
            let titles: Vec<String> = to_generate.iter().map(|(_, t)| t.clone()).collect();
            let mut slot_iter = to_generate.iter();
            let mut finished = false;
            stream_generated_movies(
                titles,
                model,
                |movie| {
                    if let Some((i, _)) = slot_iter.next() {
                        entries.with_mut(|v| {
                            if let Some(entry) = v.get_mut(*i) {
                                entry.data = movie;
                                entry.regenerating = false;
                            }
                        });
                    }
                },
                || { finished = true; },
                |e| { on_error.call(e); },
            ).await;
            on_loading.call(false);
        });
    });


    let all_locked = entries().iter().all(|e| e.locked);

    rsx! {
        div { class: "gen-grid-toolbar",
            button {
                class: "gen-btn gen-btn-lock-all",
                onclick: move |_| entries.with_mut(|v| v.iter_mut().for_each(|e| e.locked = !all_locked)),
                if all_locked { "🔓 Unlock All" } else { "🔒 Lock All" }
            }
        }

        div {
            class: "gen-movie-grid",

            for (idx, entry) in entries().iter().enumerate() {
                div {
                    key: "{idx}-{entry.data.title}",
                    class: "gen-movie-grid-item",

                    if entry.regenerating {
                        div { class: "movie-card gen-card-regenerating",
                            div { class: "movie-poster", "{entry.data.title}" }
                            div { class: "movie-details gen-regen-overlay",
                                div { class: "spinner" }
                                p { "Regenerating..." }
                            }
                        }
                    } else if entry.editing {
                        EditableMovieCard {
                            movie: entry.data.clone(),
                            on_save: {
                                let on_changed = props.on_movies_changed.clone();
                                move |updated: AiMovieData| {
                                    entries.with_mut(|v| {
                                        v[idx].data = updated;
                                        v[idx].editing = false;
                                    });
                                    let snapshot: Vec<AiMovieData> =
                                        entries().iter().map(|e| e.data.clone()).collect();
                                    on_changed.call(snapshot);
                                }
                            },
                            on_cancel: move |_| {
                                entries.with_mut(|v| v[idx].editing = false);
                            },
                        }
                    } else {
                        GeneratedMovieCard {
                            movie: entry.data.clone(),
                            locked: entry.locked,
                            on_lock_toggle: move |new_locked: bool| {
                                entries.with_mut(|v| v[idx].locked = new_locked);
                            },
                            on_edit: move |_| {
                                entries.with_mut(|v| v[idx].editing = true);
                            },
                        }
                    }
                }
            }
        }
    }
}

/// Call this from the parent once a regenerate API response comes back.
pub fn apply_regenerate_result(
    entries: &mut Vec<MovieEntry>,
    index: usize,
    result: AiMovieData,
) {
    if let Some(entry) = entries.get_mut(index) {
        entry.data = result;
        entry.regenerating = false;
    }
}
