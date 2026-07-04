use crate::components::browse_bar::BrowseBar;
use crate::components::movie_grid::MovieGrid;
use crate::components::search_bar::SearchBar;
use crate::components::search_mode_selector::SearchModeSelector;
use crate::components::{ActorChart, GenreChart, YearChart};
use dioxus::prelude::*;
use models::{BarChartData, PieChartData, ScoredMovie, SearchRequest};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct AiAction {
    pub uuid: String,
    pub action: String,
    pub model: Option<String>,
}

async fn send_search_request(
    query: String,
    disable_enhancement: bool,
    search_mode: models::SearchMode,
    selected_model: Option<String>,
) -> Result<models::SearchResponse, reqwest::Error> {
    let window = web_sys::window().unwrap();
    let location = window.location();
    let hostname = location
        .hostname()
        .unwrap_or_else(|_| "127.0.0.1".to_string());

    // TODO: This is incorrect! Front-end does not have access to environment variables
    let server_port = std::env::var("server_port").unwrap_or("3000".to_string());

    let search_request = SearchRequest {
        query,
        disable_enhancement,
        search_mode,
        model: selected_model,
    };

    let client = reqwest::Client::new();
    let response: models::SearchResponse = client
        .post(format!("http://{hostname}:{server_port}/ai/dvd-match"))
        .json(&search_request)
        .send()
        .await?
        .json()
        .await?;

    Ok(response)
}

async fn fetch_recent_movies() -> Result<Vec<ScoredMovie>, reqwest::Error> {
    let window = web_sys::window().unwrap();
    let location = window.location();
    let hostname = location
        .hostname()
        .unwrap_or_else(|_| "127.0.0.1".to_string());

    let server_port = std::env::var("server_port").unwrap_or("3000".to_string());

    let client = reqwest::Client::new();
    let response: Vec<ScoredMovie> = client
        .get(format!("http://{hostname}:{server_port}/ai/recent"))
        .send()
        .await?
        .json()
        .await?;

    Ok(response)
}

async fn fetch_recent_releases() -> Result<Vec<ScoredMovie>, reqwest::Error> {
    let window = web_sys::window().unwrap();
    let location = window.location();
    let hostname = location
        .hostname()
        .unwrap_or_else(|_| "127.0.0.1".to_string());

    let server_port = std::env::var("server_port").unwrap_or("3000".to_string());

    let client = reqwest::Client::new();
    let response: Vec<ScoredMovie> = client
        .get(format!("http://{hostname}:{server_port}/dvd/recent-releases"))
        .send()
        .await?
        .json()
        .await?;

    Ok(response)
}

async fn fetch_random_movies(count: u32) -> Result<Vec<ScoredMovie>, reqwest::Error> {
    let window = web_sys::window().unwrap();
    let location = window.location();
    let hostname = location
        .hostname()
        .unwrap_or_else(|_| "127.0.0.1".to_string());

    let server_port = std::env::var("server_port").unwrap_or("3000".to_string());

    let client = reqwest::Client::new();
    let response: Vec<ScoredMovie> = client
        .get(format!("http://{hostname}:{server_port}/dvd/random?count={count}"))
        .send()
        .await?
        .json()
        .await?;

    Ok(response)
}

async fn fetch_unique_locations() -> Result<Vec<String>, reqwest::Error> {
    let window = web_sys::window().unwrap();
    let location = window.location();
    let hostname = location
        .hostname()
        .unwrap_or_else(|_| "127.0.0.1".to_string());

    let server_port = std::env::var("server_port").unwrap_or("3000".to_string());

    let client = reqwest::Client::new();
    let response: Vec<String> = client
        .get(format!("http://{hostname}:{server_port}/uniqueLocations()"))
        .send()
        .await?
        .json()
        .await?;

    Ok(response)
}

async fn fetch_movies_by_location(
    location_name: String,
) -> Result<Vec<ScoredMovie>, reqwest::Error> {
    let window = web_sys::window().unwrap();
    let location = window.location();
    let hostname = location
        .hostname()
        .unwrap_or_else(|_| "127.0.0.1".to_string());

    let server_port = std::env::var("server_port").unwrap_or("3000".to_string());

    let encoded: String = location_name
        .bytes()
        .map(
            |b| match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    (b as char).to_string()
                }
                _ => format!(
                    "%{:02X}",
                    b
                ),
            },
        )
        .collect();

    let client = reqwest::Client::new();
    let response: Vec<models::FullMovie> = client
        .get(format!("http://{hostname}:{server_port}/location/{encoded}"))
        .send()
        .await?
        .json()
        .await?;

    // Wrap FullMovie into ScoredMovie with 0 score for MovieGrid compatibility
    Ok(
        response
            .into_iter()
            .map(
                |movie| ScoredMovie {
                    movie,
                    vector_score: 0.0,
                },
            )
            .collect(),
    )
}

async fn fetch_unknown_location_movies() -> Result<Vec<ScoredMovie>, reqwest::Error> {
    let window = web_sys::window().unwrap();
    let location = window.location();
    let hostname = location
        .hostname()
        .unwrap_or_else(|_| "127.0.0.1".to_string());

    let server_port = std::env::var("server_port").unwrap_or("3000".to_string());

    let client = reqwest::Client::new();
    let response: Vec<ScoredMovie> = client
        .get(format!("http://{hostname}:{server_port}/dvd/unknown-location"))
        .send()
        .await?
        .json()
        .await?;

    Ok(response)
}

async fn fetch_available_models() -> Result<models::AvailableModelsResponse, reqwest::Error> {
    let window = web_sys::window().unwrap();
    let location = window.location();
    let hostname = location
        .hostname()
        .unwrap_or_else(|_| "127.0.0.1".to_string());

    let server_port = std::env::var("server_port").unwrap_or("3000".to_string());

    let client = reqwest::Client::new();
    let response: models::AvailableModelsResponse = client
        .get(format!("http://{hostname}:{server_port}/ai/models"))
        .send()
        .await?
        .json()
        .await?;

    Ok(response)
}

#[component]
pub fn LiveResults() -> Element {
    let mut movies: Signal<Arc<Vec<ScoredMovie>>> = use_signal(|| Arc::new(Vec::new()));
    let mut input_value = use_signal(String::new);
    let mut is_loading = use_signal(|| false);
    let mut disable_enhancement = use_signal(|| false);
    let mut search_mode = use_signal(|| models::SearchMode::Text);
    let mut enhanced_query = use_signal(String::new);
    let mut original_query = use_signal(String::new);
    let mut selected_model = use_signal(|| None::<String>);
    let mut showing_random = use_signal(|| false);
    let mut showing_recent_movies = use_signal(|| false);
    let mut year_data = use_signal(
        || BarChartData {
            labels: vec![],
            values: vec![],
        },
    );
    let mut genre_data = use_signal(|| PieChartData { data: vec![] });
    let mut actor_data = use_signal(
        || BarChartData {
            labels: vec![],
            values: vec![],
        },
    );
    let mut available_models = use_signal(Vec::<models::AvailableModel>::new);
    let mut available_locations = use_signal(Vec::<String>::new);

    // Fetch available models on component mount
    use_effect(
        move || {
            spawn(
                async move {
                    match fetch_available_models().await {
                        Ok(response) => {
                            available_models.set(response.models);
                        }
                        Err(err) => {
                            log::error!(
                                "Failed to fetch available models: {:#?}",
                                err
                            );
                        }
                    }
                },
            );
        },
    );

    // Fetch unique locations on component mount
    use_effect(
        move || {
            spawn(
                async move {
                    match fetch_unique_locations().await {
                        Ok(mut locations) => {
                            locations.sort();
                            available_locations.set(locations);
                        }
                        Err(err) => {
                            log::error!(
                                "Failed to fetch unique locations: {:#?}",
                                err
                            );
                        }
                    }
                },
            );
        },
    );

    rsx! {
        div {
            class: "ai-live-results-container",

            // Browse section
            BrowseBar {
                is_loading: is_loading(),
                locations: available_locations(),
                on_by_location: move |loc: String| {
                    spawn(async move {
                        if is_loading() {
                            return;
                        }

                        is_loading.set(true);
                        showing_random.set(false);
                        showing_recent_movies.set(false);
                        year_data.set(BarChartData { labels: vec![], values: vec![] });
                        genre_data.set(PieChartData { data: vec![] });
                        actor_data.set(BarChartData { labels: vec![], values: vec![] });
                        movies.set(Arc::new(vec![]));
                        enhanced_query.set(String::new());
                        original_query.set(String::new());

                        let result = fetch_movies_by_location(loc.clone()).await;
                        match result {
                            Ok(by_location) => {
                                movies.set(Arc::new(by_location));
                            }
                            Err(err) => {
                                log::error!("Failed to fetch movies for location {}: {:#?}", loc, err);
                                movies.set(Arc::new(vec![]));
                            }
                        }
                        is_loading.set(false);
                    });
                },
                on_browse: move |_| {
                    spawn(async move {
                        if is_loading() {
                            return;
                        }

                        is_loading.set(true);
                        showing_random.set(false);
                        showing_recent_movies.set(false);
                        year_data.set(BarChartData { labels: vec![], values: vec![] });
                        genre_data.set(PieChartData { data: vec![] });
                        actor_data.set(BarChartData { labels: vec![], values: vec![] });
                        movies.set(Arc::new(vec![]));
                        enhanced_query.set(String::new());
                        original_query.set(String::new());

                        let result = fetch_recent_movies().await;
                        match result {
                            Ok(recent_movies) => {
                                movies.set(Arc::new(recent_movies));
                            }
                            Err(err) => {
                                log::error!("Failed to fetch recent movies {:#?}", err);
                                movies.set(Arc::new(vec![]));
                            }
                        }
                        is_loading.set(false);
                    });
                },
                on_recent_releases: move |_| {
                    spawn(async move {
                        if is_loading() {
                            return;
                        }

                        is_loading.set(true);
                        showing_random.set(false);
                        showing_recent_movies.set(true);
                        genre_data.set(PieChartData { data: vec![] });
                        actor_data.set(BarChartData { labels: vec![], values: vec![] });
                        movies.set(Arc::new(vec![]));
                        enhanced_query.set(String::new());
                        original_query.set(String::new());

                        let result = fetch_recent_releases().await;
                        match result {
                            Ok(recent_releases) => {
                                movies.set(Arc::new(recent_releases));
                            }
                            Err(err) => {
                                log::error!("Failed to fetch recent releases {:#?}", err);
                                movies.set(Arc::new(vec![]));
                            }
                        }

                        // Fetch year stats
                        let window = web_sys::window().unwrap();
                        let location = window.location();
                        let hostname = location.hostname().unwrap_or_else(|_| "127.0.0.1".to_string());
                        let server_port = std::env::var("server_port").unwrap_or("3000".to_string());

                        match reqwest::get(format!("http://{hostname}:{server_port}/stats/movies-by-year")).await {
                            Ok(resp) => {
                                if let Ok(data) = resp.json::<BarChartData>().await {
                                    year_data.set(data);
                                }
                            }
                            Err(err) => {
                                log::error!("Failed to fetch year stats: {:#?}", err);
                            }
                        }

                        is_loading.set(false);
                    });
                },
                on_random: move |_| {
                    spawn(async move {
                        if is_loading() {
                            return;
                        }

                        is_loading.set(true);
                        showing_random.set(true);
                        showing_recent_movies.set(false);
                        year_data.set(BarChartData { labels: vec![], values: vec![] });
                        movies.set(Arc::new(vec![]));
                        enhanced_query.set(String::new());
                        original_query.set(String::new());

                        let result = fetch_random_movies(3).await;
                        match result {
                            Ok(random_movies) => {
                                movies.set(Arc::new(random_movies));
                            }
                            Err(err) => {
                                log::error!("Failed to fetch random movies {:#?}", err);
                                movies.set(Arc::new(vec![]));
                            }
                        }

                        // Fetch genre and actor stats
                        let window = web_sys::window().unwrap();
                        let location = window.location();
                        let hostname = location.hostname().unwrap_or_else(|_| "127.0.0.1".to_string());
                        let server_port = std::env::var("server_port").unwrap_or("3000".to_string());

                        match reqwest::get(format!("http://{hostname}:{server_port}/stats/genres")).await {
                            Ok(resp) => {
                                if let Ok(data) = resp.json::<PieChartData>().await {
                                    genre_data.set(data);
                                }
                            }
                            Err(err) => {
                                log::error!("Failed to fetch genre stats: {:#?}", err);
                            }
                        }

                        match reqwest::get(format!("http://{hostname}:{server_port}/stats/top-actors")).await {
                            Ok(resp) => {
                                if let Ok(data) = resp.json::<BarChartData>().await {
                                    actor_data.set(data);
                                }
                            }
                            Err(err) => {
                                log::error!("Failed to fetch actor stats: {:#?}", err);
                            }
                        }

                        is_loading.set(false);
                    });
                },
                on_unknown_location: move |_| {
                    spawn(async move {
                        if is_loading() {
                            return;
                        }

                        is_loading.set(true);
                        showing_random.set(false);
                        showing_recent_movies.set(false);
                        year_data.set(BarChartData { labels: vec![], values: vec![] });
                        genre_data.set(PieChartData { data: vec![] });
                        actor_data.set(BarChartData { labels: vec![], values: vec![] });
                        movies.set(Arc::new(vec![]));
                        enhanced_query.set(String::new());
                        original_query.set(String::new());

                        let result = fetch_unknown_location_movies().await;
                        match result {
                            Ok(unknown_movies) => {
                                movies.set(Arc::new(unknown_movies));
                            }
                            Err(err) => {
                                log::error!("Failed to fetch unknown location movies {:#?}", err);
                                movies.set(Arc::new(vec![]));
                            }
                        }
                        is_loading.set(false);
                    });
                }
            }

            // Search section
            div {
                class: "search-section",

                // Search mode tabs
                SearchModeSelector {
                    search_mode: search_mode(),
                    on_mode_change: move |mode| search_mode.set(mode),
                    selected_model: selected_model(),
                    on_model_change: move |model| selected_model.set(model),
                    available_models: available_models
                }

                SearchBar {
                    input_value: input_value(),
                    on_input_change: move |value| input_value.set(value),
                    on_search: move |_| {
                        let query = input_value();
                        let disable_enh = disable_enhancement();
                        let mode = search_mode();
                        let model = selected_model();

                        spawn(async move {
                            if is_loading() {
                                return;
                            }

                            is_loading.set(true);
                            showing_random.set(false);
                            showing_recent_movies.set(false);
                            year_data.set(BarChartData { labels: vec![], values: vec![] });
                            genre_data.set(PieChartData { data: vec![] });
                            actor_data.set(BarChartData { labels: vec![], values: vec![] });
                            movies.set(Arc::new(vec![]));

                            let result = send_search_request(query, disable_enh, mode, model).await;
                            match result {
                                Ok(response) => {
                                    movies.set(Arc::new(response.results));
                                    original_query.set(response.original_query);
                                    enhanced_query.set(response.enhanced_query);
                                }
                                Err(err) => {
                                    log::error!("Failed to send search request {:#?}", err);
                                    movies.set(Arc::new(vec![]));
                                }
                            }
                            is_loading.set(false);
                        });
                    },
                    is_loading: is_loading(),
                    search_mode: search_mode(),
                    disable_enhancement: disable_enhancement(),
                    on_enhancement_toggle: move |checked| disable_enhancement.set(checked),
                    enhanced_query: enhanced_query(),
                    original_query: original_query()
                }
            }

            // Visual indicator for random movies
            if showing_random() {
                div {
                    class: "random-indicator",
                    style: "text-align: center; padding: 12px; margin: 16px 0; background: rgba(8, 145, 178, 0.1); border-radius: 8px; border: 1px solid rgba(8, 145, 178, 0.3);",
                    span {
                        style: "color: #0891b2; font-weight: 600; font-size: 14px;",
                        "🎲 Showing 3 random movies"
                    }
                }
            }

            // Year chart for recent movies
            if showing_recent_movies() {
                div {
                    class: "recent-movies-chart-wrapper",
                    YearChart {
                        data: year_data,
                    }
                }
            }

            // Movie grid section
            MovieGrid {
                movies: Arc::clone(&movies()),
                search_mode: search_mode(),
                on_location_updated: move |(movie_id, new_location): (models::MovieId, String)| {
                    log::info!("Location update callback called for movie {} with location: {}", movie_id, new_location);
                    // Update the movie in the list
                    let current_movies = movies();
                    let updated_movies: Vec<ScoredMovie> = current_movies
                        .iter()
                        .map(|scored_movie| {
                            if scored_movie.movie.id == movie_id {
                                let mut updated_movie = scored_movie.movie.clone();
                                updated_movie.location = Some(new_location.clone());
                                log::info!("Updated movie {} location to: {}", movie_id, new_location);
                                ScoredMovie {
                                    movie: updated_movie,
                                    vector_score: scored_movie.vector_score,
                                }
                            } else {
                                scored_movie.clone()
                            }
                        })
                        .collect();
                    let count = updated_movies.len();
                    movies.set(Arc::new(updated_movies));
                    log::info!("Movies signal updated, new count: {}", count);
                },
            }

            // Genre and actor charts for random movies
            if showing_random() {
                div {
                    class: "random-movies-chart-wrapper",
                    GenreChart { data: genre_data }
                    ActorChart { data: actor_data }
                }
            }
        }
    }
}
