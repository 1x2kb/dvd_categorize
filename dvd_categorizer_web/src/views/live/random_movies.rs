// random_movies.rs
use dioxus::prelude::*;
use log::error;
use models::{
    BarChartData, PieChartData, RandomSelectionOddsResponse, ScoredMovie, SearchMode,
};

use crate::api::get_json;
use crate::components::movie_results::MovieResults;
use crate::components::{ActorChart, GenreChart, RandomOdds};
use crate::views::live::BrowseViewProps;

const RANDOM_COUNT: u32 = 3;

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

#[component]
pub fn RandomMovies(props: BrowseViewProps) -> Element {
    let mut movies = use_signal(Vec::<ScoredMovie>::new);
    let mut genre_data = use_signal(empty_pie);
    let mut actor_data = use_signal(empty_bars);
    let mut odds = use_signal(empty_odds);

    let mut is_loading = props.is_loading;
    let focused = props.focused;

    use_effect(move || {
        spawn(async move {
            is_loading.set(true);
            genre_data.set(empty_pie());
            actor_data.set(empty_bars());
            odds.set(empty_odds());

            match get_json::<Vec<ScoredMovie>>(&format!("/dvd/random?count={RANDOM_COUNT}")).await {
                Ok(found) => movies.set(found),
                Err(e) => {
                    error!("Failed to fetch random movies: {e}");
                    movies.set(Vec::new());
                }
            }

            match get_json::<RandomSelectionOddsResponse>(&format!(
                "/stats/random-odds?count={RANDOM_COUNT}"
            ))
            .await
            {
                Ok(stats) => {
                    genre_data.set(PieChartData {
                        data: stats
                            .genres
                            .iter()
                            .map(|item| (item.label.clone(), item.count as f64))
                            .collect(),
                    });
                    actor_data.set(BarChartData {
                        labels: stats
                            .actors
                            .iter()
                            .map(|item| item.label.clone())
                            .collect(),
                        values: stats
                            .actors
                            .iter()
                            .map(|item| item.count as f64)
                            .collect(),
                    });
                    odds.set(stats);
                }
                Err(e) => error!("Failed to fetch random odds: {e}"),
            }

            is_loading.set(false);
        });
    });

    rsx! {
        div {
            class: "random-indicator",
            style: "text-align: center; padding: 12px; margin: 16px 0; background: rgba(8, 145, 178, 0.1); border-radius: 8px; border: 1px solid rgba(8, 145, 178, 0.3);",
            span {
                style: "color: #0891b2; font-weight: 600; font-size: 14px;",
                "🎲 Showing {RANDOM_COUNT} random movies"
            }
        }

        MovieResults {
            movies,
            search_mode: SearchMode::Text,
            focused
        }

        div {
            class: "random-movies-chart-wrapper",
            div {
                class: "random-movies-charts-row",
                GenreChart { data: genre_data }
                ActorChart { data: actor_data }
            }
            RandomOdds { data: odds }
        }
    }
}
