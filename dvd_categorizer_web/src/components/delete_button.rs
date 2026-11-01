// delete_button.rs
use dioxus::prelude::*;
use models::MovieId;

use crate::api::delete_movie;
use crate::components::movie_grid::use_focused_context;

#[derive(Props, Clone, PartialEq)]
pub struct DeleteButtonProps {
    pub movie_id: MovieId,
    /// Stable per-row key. CSV preview rows all share a placeholder id.
    pub key_hash: u64,
    /// `true` issues `DELETE /dvd/{id}`; `false` only notifies the parent
    /// (used by the CSV preview grid, whose rows aren't persisted yet).
    pub persist: bool,
    pub on_deleted: EventHandler<(
        MovieId,
        u64,
    )>,
}

/// Two-step delete: the first click arms the confirm row, the second deletes.
#[component]
pub fn DeleteButton(props: DeleteButtonProps) -> Element {
    let mut confirming = use_signal(|| false);
    let mut is_deleting = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);

    // Arming the confirm row locks this card's focus so it doesn't blur out
    // from under its own buttons. The prior lock is restored on cancel.
    let mut focused = use_focused_context();
    let mut previous_focus = use_signal(|| None::<u64>);

    let on_deleted = props.on_deleted;
    let movie_id_delete = props
        .movie_id
        .clone();
    let key_hash = props.key_hash;
    let persist = props.persist;

    rsx! {
        div {
            class: "delete-confirm-container",
            onclick: move |evt| evt.stop_propagation(),

            if confirming() {
                button {
                    class: "delete-button delete-confirm-button",
                    disabled: is_deleting(),
                    onclick: move |_| {
                        let movie_id = movie_id_delete.clone();
                        spawn(async move {
                            is_deleting.set(true);
                            error.set(None);

                            if !persist {
                                // Preview rows aren't in the database — just drop the row.
                                focused.set(None);
                                on_deleted.call((movie_id, key_hash));
                                return;
                            }

                            match delete_movie(movie_id.clone()).await {
                                Ok(()) => {
                                    // Card is about to unmount — release the lock.
                                    focused.set(None);
                                    on_deleted.call((movie_id, key_hash));
                                }
                                Err(e) => {
                                    log::error!("Failed to delete movie: {}", e);
                                    error.set(Some(e));
                                    is_deleting.set(false);
                                    confirming.set(false);
                                    focused.set(previous_focus());
                                }
                            }
                        });
                    },
                    if is_deleting() { "Deleting..." } else { "Confirm delete" }
                }
                button {
                    class: "delete-cancel-button",
                    disabled: is_deleting(),
                    onclick: move |_| {
                        confirming.set(false);
                        error.set(None);
                        focused.set(previous_focus());
                    },
                    "Cancel"
                }
            } else {
                button {
                    class: "delete-button",
                    onclick: move |_| {
                        previous_focus.set(focused());
                        focused.set(Some(key_hash));
                        confirming.set(true);
                        error.set(None);
                    },
                    "Delete"
                }
            }

            if let Some(message) = error() {
                div {
                    class: "delete-error",
                    "{message}"
                }
            }
        }
    }
}
