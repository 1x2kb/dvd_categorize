use dioxus::prelude::*;
use dioxus_grapher::{BarGraph, PieChart};
use crate::app_data::AppData;
use models::FullMovie;
use std::collections::HashMap;

#[component]
pub fn Stats() -> Element {
    let app_data = use_context::<Signal<AppData>>();
    let movies = app_data.read().movies.read().clone();

    let stats = use_memo(move || {
        if let Some(movies_data) = movies.as_ref() {
            calculate_stats(&movies_data)
        } else {
            MovieStats::default()
        }
    });

    rsx! {
        div {
            class: "stats-container",
            h1 { "DVD Collection Statistics" }
            
            div {
                class: "stats-grid",
                
                div {
                    class: "stat-card",
                    h2 { "Total Movies" }
                    p { class: "stat-value", "{stats.read().total_movies}" }
                }
                
                div {
                    class: "stat-card",
                    h2 { "Total Directors" }
                    p { class: "stat-value", "{stats.read().total_directors}" }
                }
                
                div {
                    class: "stat-card",
                    h2 { "Total Actors" }
                    p { class: "stat-value", "{stats.read().total_actors}" }
                }
            }
            
            div {
                class: "charts-container",
                
                div {
                    class: "chart-section",
                    h2 { "Movies by Year" }
                    BarGraph {
                        data: stats.read().year_data.clone(),
                        width: 1200.0,
                        height: 400.0,
                        bar_color: "#0891b2".to_string(),
                        x_label: "Year".to_string(),
                        y_label: "Movies".to_string(),
                        skip_labels: 5,
                    }
                }
                
                div {
                    class: "chart-section",
                    h2 { "Top 10 Genres" }
                    PieChart {
                        data: stats.read().genre_data.clone(),
                        size: 500.0,
                        top_n: Some(10),
                    }
                }
                
                div {
                    class: "chart-section",
                    h2 { "Top 10 Actors" }
                    BarGraph {
                        data: stats.read().actor_data.clone(),
                        width: 1200.0,
                        height: 400.0,
                        bar_color: "#0e7490".to_string(),
                        x_label: "Top Actors".to_string(),
                        y_label: "Movies".to_string(),
                    }
                }
            }
        }
    }
}

#[derive(Clone, Default, PartialEq)]
struct MovieStats {
    total_movies: usize,
    total_directors: usize,
    total_actors: usize,
    year_data: Vec<(String, f64)>,
    genre_data: Vec<(String, f64)>,
    actor_data: Vec<(String, f64)>,
}

fn calculate_stats(movies: &[FullMovie]) -> MovieStats {
    let total_movies = movies.len();
    
    let total_directors = movies.iter()
        .filter_map(|m| m.director.as_ref())
        .map(|d| d.name.clone())
        .collect::<std::collections::HashSet<_>>()
        .len();
    
    let total_actors = movies.iter()
        .flat_map(|m| &m.actors)
        .map(|a| a.name.clone())
        .collect::<std::collections::HashSet<_>>()
        .len();
    
    let mut year_counts: HashMap<i32, usize> = HashMap::new();
    for movie in movies {
        if movie.release_year > 0 {
            *year_counts.entry(movie.release_year).or_insert(0) += 1;
        }
    }
    let mut year_data: Vec<(String, f64)> = year_counts
        .into_iter()
        .map(|(year, count)| (year.to_string(), count as f64))
        .collect();
    year_data.sort_by_key(|(year, _)| year.parse::<i32>().unwrap_or(0));
    
    let mut genre_counts: HashMap<String, usize> = HashMap::new();
    for movie in movies {
        for genre in &movie.genres {
            *genre_counts.entry(genre.clone()).or_insert(0) += 1;
        }
    }
    let genre_data: Vec<(String, f64)> = genre_counts
        .into_iter()
        .map(|(genre, count)| (genre, count as f64))
        .collect();
    
    let mut actor_counts: HashMap<String, usize> = HashMap::new();
    for movie in movies {
        for actor in &movie.actors {
            *actor_counts.entry(actor.name.clone()).or_insert(0) += 1;
        }
    }
    let mut actor_data: Vec<(String, f64)> = actor_counts
        .into_iter()
        .map(|(actor, count)| (actor, count as f64))
        .collect();
    actor_data.sort_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap());
    actor_data.truncate(10);
    
    MovieStats {
        total_movies,
        total_directors,
        total_actors,
        year_data,
        genre_data,
        actor_data,
    }
}
