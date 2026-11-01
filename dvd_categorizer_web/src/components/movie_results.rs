// movie_results.rs
use dioxus::prelude::*;
use models::{MovieId, ScoredMovie, SearchMode};

use crate::components::movie_grid::MovieGrid;

#[derive(Props, Clone, PartialEq)]
pub struct MovieResultsProps {
    pub movies: Signal<Vec<ScoredMovie>>,
    pub search_mode: SearchMode,
    pub focused: Signal<Option<u64>>,
}

/// Grid plus the standard in-place location and delete handlers, so each live
/// view only has to own its `movies` signal.
#[component]
pub fn MovieResults(props: MovieResultsProps) -> Element {
    let mut movies = props.movies;

    rsx! {
        MovieGrid {
            movies: props.movies,
            search_mode: props.search_mode,
            on_location_updated: move |(movie_id, new_location): (MovieId, String)| {
                movies.with_mut(|list| {
                    if let Some(scored) = list.iter_mut().find(|m| m.movie.id == movie_id) {
                        scored.movie.location = Some(new_location);
                    }
                });
            },
            persist_deletes: true,
            on_movie_deleted: move |(movie_id, _key_hash): (MovieId, u64)| {
                movies.with_mut(|list| list.retain(|m| m.movie.id != movie_id));
            },
            focused: props.focused,
        }
    }
}