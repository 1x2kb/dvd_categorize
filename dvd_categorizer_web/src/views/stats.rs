use dioxus::prelude::*;
use crate::components::{StatsOverviewCard, YearChart, GenreChart, ActorChart};
use models::{StatsOverview, BarChartData, PieChartData};

#[component]
pub fn Stats() -> Element {
    let mut overview = use_signal(|| StatsOverview {
        total_movies: 0,
        total_directors: 0,
        total_actors: 0,
    });
    let mut year_data = use_signal(|| BarChartData {
        labels: vec![],
        values: vec![],
    });
    let mut genre_data = use_signal(|| PieChartData {
        data: vec![],
    });
    let mut actor_data = use_signal(|| BarChartData {
        labels: vec![],
        values: vec![],
    });

    // Fetch stats on mount
    use_effect(move || {
        spawn(async move {
            // Fetch overview
            if let Ok(resp) = reqwest::get("http://localhost:3000/stats/overview").await {
                if let Ok(data) = resp.json::<StatsOverview>().await {
                    overview.set(data);
                }
            }
            
            // Fetch year data
            if let Ok(resp) = reqwest::get("http://localhost:3000/stats/movies-by-year").await {
                if let Ok(data) = resp.json::<BarChartData>().await {
                    year_data.set(data);
                }
            }
            
            // Fetch genre data
            if let Ok(resp) = reqwest::get("http://localhost:3000/stats/genres").await {
                if let Ok(data) = resp.json::<PieChartData>().await {
                    genre_data.set(data);
                }
            }
            
            // Fetch actor data
            if let Ok(resp) = reqwest::get("http://localhost:3000/stats/top-actors").await {
                if let Ok(data) = resp.json::<BarChartData>().await {
                    actor_data.set(data);
                }
            }
        });
    });

    rsx! {
        div {
            class: "stats-container",
            h1 { "DVD Collection Statistics" }
            
            StatsOverviewCard { overview }
            
            div {
                class: "charts-container",
                YearChart { data: year_data }
                GenreChart { data: genre_data }
                ActorChart { data: actor_data }
            }
        }
    }
}
