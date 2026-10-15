use dioxus::prelude::*;
use dioxus_grapher::{AutoOptions, BarGraph, XAxisMode};
use models::PieChartData;

#[component]
pub fn GenreChart(data: ReadSignal<PieChartData>) -> Element {
    rsx! {
        div {
            class: "chart-section genre-chart-responsive",
            style: "width: 100%;",
            h2 { "Top 10 Genres" }
            p {
                style: "font-size: 0.875rem; color: #9ca3af; margin-bottom: 0.5rem;",
                "Note: Movies can have multiple genres"
            }
            div {
                style: "width: 100%; height: 400px;",
                BarGraph {
                    data: data().data,
                    bar_color: "#0891b2".to_string(),
                    x_label: "Genre".to_string(),
                    y_label: "Movies".to_string(),
                    x_mode: XAxisMode::Auto(AutoOptions { skip_labels: 1 }),
                    responsive: true,
                    mobile_max_items: Some(8),
                }
            }
        }
    }
}
