// by_location.rs
use dioxus::prelude::*;
use log::error;
use models::{FullMovie, ScoredMovie, SearchMode};

use crate::api::{encode_segment, get_json};
use crate::components::movie_results::MovieResults;

#[derive(Props, Clone, PartialEq)]
pub struct ByLocationProps {
    pub location: String,
    pub focused: Signal<Option<u64>>,
    pub is_loading: Signal<bool>,
}

#[component]
pub fn ByLocationView(props: ByLocationProps) -> Element {
    let mut movies = use_signal(Vec::<ScoredMovie>::new);
    let mut is_loading = props.is_loading;
    let focused = props.focused;
    let location = props
        .location
        .clone();

    use_effect(
        move || {
            let location = location.clone();
            spawn(
                async move {
                    is_loading.set(true);
                    let path = format!(
                        "/location/{}",
                        encode_segment(&location)
                    );
                    match get_json::<Vec<FullMovie>>(&path).await {
                        Ok(found) => movies.set(
                            found
                                .into_iter()
                                .map(
                                    |movie| ScoredMovie {
                                        movie,
                                        vector_score: 0.0,
                                    },
                                )
                                .collect(),
                        ),
                        Err(e) => {
                            error!("Failed to fetch movies for location {location}: {e}");
                            movies.set(Vec::new());
                        }
                    }
                    is_loading.set(false);
                },
            );
        },
    );

    rsx! {
        MovieResults {
            movies,
            search_mode: SearchMode::Text,
            focused
        }
    }
}
