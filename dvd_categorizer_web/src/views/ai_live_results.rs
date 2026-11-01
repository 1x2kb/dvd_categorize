// ai_live_results.rs
use crate::components::browse_bar::BrowseBar;
use crate::components::movie_grid::MovieGrid;
use crate::components::search_bar::SearchBar;
use crate::components::search_mode_selector::SearchModeSelector;
use crate::components::{ActorChart, GenreChart, RandomOdds, YearChart};
use dioxus::prelude::*;
use log::error;
use models::{
    AvailableModel, AvailableModelsResponse, BarChartData, FullMovie, MovieId, PieChartData,
    RandomSelectionOddsResponse, ScoredMovie, SearchMode, SearchRequest, SearchResponse,
};
use serde::de::DeserializeOwned;

// ---------------------------------------------------------------------------
// Transport
// ---------------------------------------------------------------------------

fn api_base() -> String {
    let hostname = web_sys::window()
        .and_then(
            |w| {
                w.location()
                    .hostname()
                    .ok()
            },
        )
        .unwrap_or_else(|| "127.0.0.1".to_string());
    let port = std::env::var("server_port").unwrap_or_else(|_| "3000".to_string());
    format!("http://{hostname}:{port}")
}

async fn get_json<T: DeserializeOwned>(path: &str) -> Result<T, String> {
    let url = format!(
        "{}{}",
        api_base(),
        path
    );
    let response = gloo_net::http::Request::get(&url)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !response.ok() {
        return Err(
            format!(
                "{} {}",
                response.status(),
                response.status_text()
            ),
        );
    }
    response
        .json::<T>()
        .await
        .map_err(|e| e.to_string())
}

/// Percent-encode a path segment.
fn encode_segment(raw: &str) -> String {
    raw.bytes()
        .map(
            |b| match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    (b as char).to_string()
                }
                _ => format!("%{b:02X}"),
            },
        )
        .collect()
}

fn empty_bars() -> BarChartData {
    BarChartData {
        labels: vec![],
        values: vec![],
    }
}

fn empty_pie() -> PieChartData {
    PieChartData { data: vec![] }
}

fn empty_odds() -> RandomSelectionOddsResponse {
    RandomSelectionOddsResponse {
        total_movies: 0,
        random_count: 0,
        genres: vec![],
        actors: vec![],
    }
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct LiveState {
    movies: Signal<Vec<ScoredMovie>>,
    input_value: Signal<String>,
    is_loading: Signal<bool>,
    disable_enhancement: Signal<bool>,
    search_mode: Signal<SearchMode>,
    enhanced_query: Signal<String>,
    original_query: Signal<String>,
    selected_model: Signal<Option<String>>,
    showing_random: Signal<bool>,
    showing_recent_movies: Signal<bool>,
    year_data: Signal<BarChartData>,
    genre_data: Signal<PieChartData>,
    actor_data: Signal<BarChartData>,
    random_odds: Signal<RandomSelectionOddsResponse>,
    available_models: Signal<Vec<AvailableModel>>,
    available_locations: Signal<Vec<String>>,
    focused: Signal<Option<u64>>,
}

impl LiveState {
    fn is_loading(&self) -> bool {
        *self
            .is_loading
            .read()
    }

    fn search_mode(&self) -> SearchMode {
        *self
            .search_mode
            .read()
    }

    fn selected_model(&self) -> Option<String> {
        self.selected_model
            .read()
            .clone()
    }

    /// Shared preamble for every browse action. `clear_year` and `clear_charts`
    /// exist because the original handlers cleared different subsets: recent
    /// releases keeps the year chart until its own fetch lands, and random
    /// keeps the genre/actor/odds panels until its own fetch lands.
    fn reset_view(&mut self, clear_year: bool, clear_charts: bool) {
        self.showing_random
            .set(false);
        self.showing_recent_movies
            .set(false);
        self.movies
            .set(Vec::new());
        self.enhanced_query
            .set(String::new());
        self.original_query
            .set(String::new());
        self.focused
            .set(None);

        if clear_year {
            self.year_data
                .set(empty_bars());
        }
        if clear_charts {
            self.genre_data
                .set(empty_pie());
            self.actor_data
                .set(empty_bars());
            self.random_odds
                .set(empty_odds());
        }
    }
}

// ---------------------------------------------------------------------------
// Loaders — one per browse action
// ---------------------------------------------------------------------------

async fn load_by_location(mut st: LiveState, location: String) {
    if st.is_loading() {
        return;
    }
    st.is_loading
        .set(true);
    st.reset_view(
        true, true,
    );

    let path = format!(
        "/location/{}",
        encode_segment(&location)
    );
    match get_json::<Vec<FullMovie>>(&path).await {
        Ok(movies) => st
            .movies
            .set(
                movies
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
            st.movies
                .set(Vec::new());
        }
    }

    st.is_loading
        .set(false);
}

async fn load_recent(mut st: LiveState) {
    if st.is_loading() {
        return;
    }
    st.is_loading
        .set(true);
    st.reset_view(
        true, true,
    );

    match get_json::<Vec<ScoredMovie>>("/ai/recent").await {
        Ok(movies) => st
            .movies
            .set(movies),
        Err(e) => {
            error!("Failed to fetch recent movies: {e}");
            st.movies
                .set(Vec::new());
        }
    }

    st.is_loading
        .set(false);
}

async fn load_recent_releases(mut st: LiveState) {
    if st.is_loading() {
        return;
    }
    st.is_loading
        .set(true);
    st.reset_view(
        false, true,
    );
    st.showing_recent_movies
        .set(true);

    match get_json::<Vec<ScoredMovie>>("/dvd/recent-releases").await {
        Ok(movies) => st
            .movies
            .set(movies),
        Err(e) => {
            error!("Failed to fetch recent releases: {e}");
            st.movies
                .set(Vec::new());
        }
    }

    match get_json::<BarChartData>("/stats/movies-by-year").await {
        Ok(data) => st
            .year_data
            .set(data),
        Err(e) => error!("Failed to fetch year stats: {e}"),
    }

    st.is_loading
        .set(false);
}

async fn load_random(mut st: LiveState) {
    if st.is_loading() {
        return;
    }
    st.is_loading
        .set(true);
    st.reset_view(
        true, false,
    );
    st.showing_random
        .set(true);

    match get_json::<Vec<ScoredMovie>>("/dvd/random?count=3").await {
        Ok(movies) => st
            .movies
            .set(movies),
        Err(e) => {
            error!("Failed to fetch random movies: {e}");
            st.movies
                .set(Vec::new());
        }
    }

    match get_json::<RandomSelectionOddsResponse>("/stats/random-odds?count=3").await {
        Ok(odds) => {
            st.genre_data
                .set(
                    PieChartData {
                        data: odds
                            .genres
                            .iter()
                            .map(
                                |item| {
                                    (
                                        item.label
                                            .clone(),
                                        item.count as f64,
                                    )
                                },
                            )
                            .collect(),
                    },
                );
            st.actor_data
                .set(
                    BarChartData {
                        labels: odds
                            .actors
                            .iter()
                            .map(
                                |item| {
                                    item.label
                                        .clone()
                                },
                            )
                            .collect(),
                        values: odds
                            .actors
                            .iter()
                            .map(|item| item.count as f64)
                            .collect(),
                    },
                );
            st.random_odds
                .set(odds);
        }
        Err(e) => {
            error!("Failed to fetch random odds: {e}");
            st.genre_data
                .set(empty_pie());
            st.actor_data
                .set(empty_bars());
            st.random_odds
                .set(empty_odds());
        }
    }

    st.is_loading
        .set(false);
}

async fn load_unknown_location(mut st: LiveState) {
    if st.is_loading() {
        return;
    }
    st.is_loading
        .set(true);
    st.reset_view(
        true, true,
    );

    match get_json::<Vec<ScoredMovie>>("/dvd/unknown-location").await {
        Ok(movies) => st
            .movies
            .set(movies),
        Err(e) => {
            error!("Failed to fetch unknown location movies: {e}");
            st.movies
                .set(Vec::new());
        }
    }

    st.is_loading
        .set(false);
}

async fn load_search(mut st: LiveState, query: String) {
    if st.is_loading() {
        return;
    }
    st.is_loading
        .set(true);
    st.reset_view(
        true, true,
    );

    let request = SearchRequest {
        query,
        disable_enhancement: *st
            .disable_enhancement
            .read(),
        search_mode: st.search_mode(),
        model: st.selected_model(),
    };

    let result = async {
        let url = format!(
            "{}/ai/dvd-match",
            api_base()
        );
        let response = gloo_net::http::Request::post(&url)
            .json(&request)
            .map_err(|e| e.to_string())?
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !response.ok() {
            return Err(
                format!(
                    "{} {}",
                    response.status(),
                    response.status_text()
                ),
            );
        }
        response
            .json::<SearchResponse>()
            .await
            .map_err(|e| e.to_string())
    }
    .await;

    match result {
        Ok(response) => {
            st.movies
                .set(response.results);
            st.original_query
                .set(response.original_query);
            st.enhanced_query
                .set(response.enhanced_query);
        }
        Err(e) => {
            error!("Failed to send search request: {e}");
            st.movies
                .set(Vec::new());
        }
    }

    st.is_loading
        .set(false);
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

#[component]
pub fn LiveResults() -> Element {
    let mut st = LiveState {
        movies: use_signal(Vec::new),
        input_value: use_signal(String::new),
        is_loading: use_signal(|| false),
        disable_enhancement: use_signal(|| false),
        search_mode: use_signal(|| SearchMode::Text),
        enhanced_query: use_signal(String::new),
        original_query: use_signal(String::new),
        selected_model: use_signal(|| None::<String>),
        showing_random: use_signal(|| false),
        showing_recent_movies: use_signal(|| false),
        year_data: use_signal(empty_bars),
        genre_data: use_signal(empty_pie),
        actor_data: use_signal(empty_bars),
        random_odds: use_signal(empty_odds),
        available_models: use_signal(Vec::<AvailableModel>::new),
        available_locations: use_signal(Vec::<String>::new),
        focused: use_signal(|| None::<u64>),
    };

    // Fetch available models on mount
    use_effect(
        move || {
            spawn(
                async move {
                    match get_json::<AvailableModelsResponse>("/ai/models").await {
                        Ok(response) => st
                            .available_models
                            .set(response.models),
                        Err(e) => error!("Failed to fetch available models: {e}"),
                    }
                },
            );
        },
    );

    // Fetch unique locations on mount
    use_effect(
        move || {
            spawn(
                async move {
                    match get_json::<Vec<String>>("/uniqueLocations()").await {
                        Ok(mut locations) => {
                            locations.sort();
                            st.available_locations
                                .set(locations);
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
                // Any click that reaches the page clears the card lock. Cards
                // stop propagation, so their own toggle is unaffected.
                onclick: move |_| st.focused.set(None),

                BrowseBar {
                    is_loading: st.is_loading,
                    locations: st.available_locations,
                    on_by_location: move |location: String| {
                        spawn(load_by_location(st, location));
                    },
                    on_browse: move |_| {
                        spawn(load_recent(st));
                    },
                    on_recent_releases: move |_| {
                        spawn(load_recent_releases(st));
                    },
                    on_random: move |_| {
                        spawn(load_random(st));
                    },
                    on_unknown_location: move |_| {
                        spawn(load_unknown_location(st));
                    },
                }

                div {
                    class: "search-section",

                    SearchModeSelector {
                        search_mode: st.search_mode(),
                        on_mode_change: move |mode| st.search_mode.set(mode),
                        selected_model: st.selected_model,
                        on_model_change: move |model| st.selected_model.set(model),
                        available_models: st.available_models,
                    }

                    SearchBar {
                        input_value: st.input_value,
                        on_search: move |_| {
                            let query = st.input_value.read().trim().to_string();
                            spawn(load_search(st, query));
                        },
                        is_loading: st.is_loading,
                        search_mode: st.search_mode,
                        disable_enhancement: st.disable_enhancement,
                        on_enhancement_toggle: move |checked| st.disable_enhancement.set(checked),
                        enhanced_query: st.enhanced_query,
                        original_query: st.original_query,
                    }
                }

                if *st.showing_random.read() {
                    div {
                        class: "random-indicator",
                        style: "text-align: center; padding: 12px; margin: 16px 0; background: rgba(8, 145, 178, 0.1); border-radius: 8px; border: 1px solid rgba(8, 145, 178, 0.3);",
                        span {
                            style: "color: #0891b2; font-weight: 600; font-size: 14px;",
                            "🎲 Showing 3 random movies"
                        }
                    }
                }

                if *st.showing_recent_movies.read() {
                    div {
                        class: "recent-movies-chart-wrapper",
                        YearChart { data: st.year_data }
                    }
                }

                MovieGrid {
                    movies: st.movies,
                    search_mode: st.search_mode(),
                    on_location_updated: move |(movie_id, new_location): (MovieId, String)| {
                        st.movies.with_mut(|movies| {
                            if let Some(scored) = movies.iter_mut().find(|m| m.movie.id == movie_id) {
                                scored.movie.location = Some(new_location);
                            }
                        });
                    },
                    persist_deletes: true,
                    on_movie_deleted: move |(movie_id, _key_hash): (MovieId, u64)| {
                        st.movies.with_mut(|movies| movies.retain(|m| m.movie.id != movie_id));
                    },
                    focused: st.focused,
                }

                if *st.showing_random.read() {
                    div {
                        class: "random-movies-chart-wrapper",
                        div {
                            class: "random-movies-charts-row",
                            GenreChart { data: st.genre_data }
                            ActorChart { data: st.actor_data }
                        }
                        RandomOdds { data: st.random_odds }
                    }
                }
            }
        }
}