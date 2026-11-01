// ai_live_results.rs
use crate::api::get_json;
use crate::components::browse_bar::BrowseBar;
use crate::components::search_bar::SearchBar;
use crate::components::search_mode_selector::SearchModeSelector;
use crate::views::live::{
    ByLocationView, LiveView, RandomMovies, RecentMovies, RecentReleases, SearchResults,
    UnknownLocation,
};
use dioxus::prelude::*;
use log::error;
use models::{AvailableModel, AvailableModelsResponse, SearchMode};

#[derive(Clone, Copy)]
struct LiveNav {
    view: Signal<LiveView>,
    nonce: Signal<u64>,
    focused: Signal<Option<u64>>,
    is_loading: Signal<bool>,
    enhanced_query: Signal<String>,
    original_query: Signal<String>,
}

impl LiveNav {
    /// Switch views. The nonce remounts the target so picking the same view
    /// twice re-runs its fetch instead of leaving stale results on screen.
    fn open(&mut self, target: LiveView) {
        self.focused
            .set(None);
        self.enhanced_query
            .set(String::new());
        self.original_query
            .set(String::new());
        self.is_loading
            .set(false);
        self.nonce
            .with_mut(|n| *n = n.wrapping_add(1));
        self.view
            .set(target);
    }
}

#[component]
pub fn LiveResults() -> Element {
    let mut nav = LiveNav {
        view: use_signal(|| LiveView::Idle),
        nonce: use_signal(|| 0u64),
        focused: use_signal(|| None::<u64>),
        is_loading: use_signal(|| false),
        enhanced_query: use_signal(String::new),
        original_query: use_signal(String::new),
    };

    let mut input_value = use_signal(String::new);
    let mut search_mode = use_signal(|| SearchMode::Text);
    let mut disable_enhancement = use_signal(|| false);
    let mut selected_model = use_signal(|| None::<String>);
    let mut available_models = use_signal(Vec::<AvailableModel>::new);
    let mut available_locations = use_signal(Vec::<String>::new);

    use_effect(
        move || {
            spawn(
                async move {
                    match get_json::<AvailableModelsResponse>("/ai/models").await {
                        Ok(response) => available_models.set(response.models),
                        Err(e) => error!("Failed to fetch available models: {e}"),
                    }
                },
            );
        },
    );

    use_effect(
        move || {
            spawn(
                async move {
                    match get_json::<Vec<String>>("/uniqueLocations()").await {
                        Ok(mut locations) => {
                            locations.sort();
                            available_locations.set(locations);
                        }
                        Err(e) => error!("Failed to fetch unique locations: {e}"),
                    }
                },
            );
        },
    );

    rsx! {
        div {
            class: "ai-live-results-container",
            // Any click that reaches the page clears the card lock. Cards stop
            // propagation, so their own toggle is unaffected.
            onclick: move |_| nav.focused.set(None),

            BrowseBar {
                is_loading: nav.is_loading,
                locations: available_locations,
                on_by_location: move |location: String| {
                    nav.open(LiveView::ByLocation(location));
                },
                on_browse: move |_| {
                    nav.open(LiveView::Recent);
                },
                on_recent_releases: move |_| {
                    nav.open(LiveView::RecentReleases);
                },
                on_random: move |_| {
                    nav.open(LiveView::Random);
                },
                on_unknown_location: move |_| {
                    nav.open(LiveView::UnknownLocation);
                },
            }

            div {
                class: "search-section",

                SearchModeSelector {
                    search_mode: search_mode(),
                    on_mode_change: move |mode| search_mode.set(mode),
                    selected_model: selected_model,
                    on_model_change: move |model| selected_model.set(model),
                    available_models: available_models,
                }

                SearchBar {
                    input_value: input_value,
                    on_search: move |_| {
                        let query = input_value.read().trim().to_string();
                        nav.open(LiveView::Search(query));
                    },
                    is_loading: nav.is_loading,
                    search_mode: search_mode,
                    disable_enhancement: disable_enhancement,
                    on_enhancement_toggle: move |checked| disable_enhancement.set(checked),
                    enhanced_query: nav.enhanced_query,
                    original_query: nav.original_query,
                }
            }

            {
                let nonce = (nav.nonce)();
                match (nav.view)() {
                    LiveView::Idle => rsx! {},
                    LiveView::Recent => rsx! {
                        RecentMovies {
                            key: "{nonce}",
                            focused: nav.focused,
                            is_loading: nav.is_loading,
                        }
                    },
                    LiveView::RecentReleases => rsx! {
                        RecentReleases {
                            key: "{nonce}",
                            focused: nav.focused,
                            is_loading: nav.is_loading,
                        }
                    },
                    LiveView::Random => rsx! {
                        RandomMovies {
                            key: "{nonce}",
                            focused: nav.focused,
                            is_loading: nav.is_loading,
                        }
                    },
                    LiveView::UnknownLocation => rsx! {
                        UnknownLocation {
                            key: "{nonce}",
                            focused: nav.focused,
                            is_loading: nav.is_loading,
                        }
                    },
                    LiveView::ByLocation(location) => rsx! {
                        ByLocationView {
                            key: "{nonce}",
                            location,
                            focused: nav.focused,
                            is_loading: nav.is_loading,
                        }
                    },
                    LiveView::Search(query) => rsx! {
                        SearchResults {
                            key: "{nonce}",
                            query,
                            search_mode,
                            disable_enhancement,
                            selected_model,
                            enhanced_query: nav.enhanced_query,
                            original_query: nav.original_query,
                            focused: nav.focused,
                            is_loading: nav.is_loading,
                        }
                    },
                }
            }
        }
    }
}
