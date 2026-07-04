use dioxus::prelude::*;
use dioxus_grapher::BarGraph;
use models::BarChartData;

#[component]
pub fn ActorChart(data: ReadSignal<BarChartData>) -> Element {
    let chart_data: Vec<(
        String,
        f64,
    )> = data()
        .labels
        .into_iter()
        .zip(
            data()
                .values
                .into_iter(),
        )
        .collect();

    rsx! {
        div {
            class: "chart-section actor-chart-responsive",
            style: "width: 100%;",
            h2 { "Top 10 Actors" }
            div {
                style: "width: 100%; height: 400px;",
                BarGraph {
                    data: chart_data,
                    bar_color: "#0891b2".to_string(),
                    x_label: "Actor".to_string(),
                    y_label: "Movies".to_string(),
                    responsive: true,
                }
            }
        }
    }
}
