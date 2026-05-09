use dioxus::prelude::*;
use dioxus_grapher::BarGraph;
use models::BarChartData;

#[component]
pub fn ActorChart(
    data: ReadSignal<BarChartData>,
    #[props(default = 1200.0)] width: f64,
    #[props(default = 400.0)] height: f64,
) -> Element {
    let chart_data: Vec<(String, f64)> = data()
        .labels
        .into_iter()
        .zip(data().values.into_iter())
        .collect();
    
    rsx! {
        div {
            class: "chart-section",
            h2 { "Top 10 Actors" }
            BarGraph {
                data: chart_data,
                width,
                height,
                bar_color: "#0e7490".to_string(),
                x_label: "Top Actors".to_string(),
                y_label: "Movies".to_string(),
            }
        }
    }
}
