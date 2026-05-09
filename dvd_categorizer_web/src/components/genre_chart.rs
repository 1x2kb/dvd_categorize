use dioxus::prelude::*;
use dioxus_grapher::PieChart;
use models::PieChartData;

#[component]
pub fn GenreChart(
    data: ReadSignal<PieChartData>,
    #[props(default = 500.0)] size: f64,
) -> Element {
    rsx! {
        div {
            class: "chart-section",
            h2 { "Top 10 Genres" }
            p {
                style: "font-size: 0.875rem; color: #9ca3af; margin-bottom: 0.5rem;",
                "Note: Movies can have multiple genres"
            }
            PieChart {
                data: data().data,
                size,
            }
        }
    }
}
