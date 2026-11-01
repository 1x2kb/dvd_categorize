// recent_releases.rs
use dioxus::prelude::*;
use log::error;
use models::{ScoredMovie, SearchMode};

use crate::api::get_json;
use crate::components::movie_results::MovieResults;
use crate::components::ReleaseTimelineChart;
use crate::views::live::BrowseViewProps;

#[component]
pub fn RecentReleases(props: BrowseViewProps) -> Element {
    let mut movies = use_signal(Vec::<ScoredMovie>::new);
    let mut is_loading = props.is_loading;
    let focused = props.focused;

    use_effect(
        move || {
            spawn(
                async move {
                    is_loading.set(true);
                    match get_json::<Vec<ScoredMovie>>("/dvd/recent-releases").await {
                        Ok(found) => movies.set(found),
                        Err(e) => {
                            error!("Failed to fetch recent releases: {e}");
                            movies.set(Vec::new());
                        }
                    }
                    is_loading.set(false);
                },
            );
        },
    );

    rsx! {
        div {
            class: "recent-movies-chart-wrapper",
            ReleaseTimelineChart { movies }
        }

        MovieResults {
            movies,
            search_mode: SearchMode::Text,
            focused
        }
    }
}
