use dioxus::prelude::*;
use dioxus_grapher::PieChart;
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
                style: "width: 100%; height: 500px; display: flex; justify-content: center;",
                PieChart {
                    data: data().data,
                    size: 500.0,
                    responsive: true,
                }
            }
        }
    }
}
