// search_results.rs
use dioxus::prelude::*;
use log::error;
use models::{ScoredMovie, SearchMode, SearchRequest, SearchResponse};

use crate::api::post_json;
use crate::components::movie_results::MovieResults;

#[derive(Props, Clone, PartialEq)]
pub struct SearchResultsProps {
    pub query: String,
    pub search_mode: Signal<SearchMode>,
    pub disable_enhancement: Signal<bool>,
    pub selected_model: Signal<Option<String>>,
    pub enhanced_query: Signal<String>,
    pub original_query: Signal<String>,
    pub focused: Signal<Option<u64>>,
    pub is_loading: Signal<bool>,
}

#[component]
pub fn SearchResults(props: SearchResultsProps) -> Element {
    let mut movies = use_signal(Vec::<ScoredMovie>::new);

    let mut is_loading = props.is_loading;
    let focused = props.focused;
    let mut enhanced_query = props.enhanced_query;
    let mut original_query = props.original_query;

    let query = props
        .query
        .clone();
    let search_mode = props.search_mode;
    let disable_enhancement = props.disable_enhancement;
    let selected_model = props.selected_model;

    use_effect(
        move || {
            let request = SearchRequest {
                query: query.clone(),
                disable_enhancement: *disable_enhancement.read(),
                search_mode: *search_mode.read(),
                model: selected_model
                    .read()
                    .clone(),
            };

            spawn(
                async move {
                    is_loading.set(true);

                    match post_json::<SearchRequest, SearchResponse>(
                        "/ai/dvd-match",
                        &request,
                    )
                    .await
                    {
                        Ok(response) => {
                            movies.set(response.results);
                            original_query.set(response.original_query);
                            enhanced_query.set(response.enhanced_query);
                        }
                        Err(e) => {
                            error!("Failed to send search request: {e}");
                            movies.set(Vec::new());
                        }
                    }

                    is_loading.set(false);
                },
            );
        },
    );

    let mode = *props
        .search_mode
        .read();

    rsx! {
        MovieResults {
            movies,
            search_mode: mode,
            focused
        }
    }
}
