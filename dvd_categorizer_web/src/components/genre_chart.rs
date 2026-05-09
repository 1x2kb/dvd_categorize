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
            PieChart {
                data: data().data,
                size,
            }
        }
    }
}
