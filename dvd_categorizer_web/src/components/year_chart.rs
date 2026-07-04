use dioxus::prelude::*;
use dioxus_grapher::{AutoOptions, BarGraph, XAxisMode};
use models::BarChartData;

#[component]
pub fn YearChart(data: ReadSignal<BarChartData>) -> Element {
    let data_value = data();
    let chart_data: Vec<(
        String,
        f64,
    )> = data_value
        .labels
        .iter()
        .cloned()
        .zip(
            data_value
                .values
                .iter()
                .cloned(),
        )
        .collect();

    rsx! {
        div {
            class: "chart-section year-chart-responsive",
            style: "width: 100%;",
            h2 { "Top 15 Release Years by Movie Count" }
            div {
                style: "width: 100%; height: 400px;",
                BarGraph {
                    data: chart_data,
                    bar_color: "#0891b2".to_string(),
                    x_label: "Year".to_string(),
                    y_label: "Movies".to_string(),
                    x_mode: XAxisMode::Auto(AutoOptions { skip_labels: 1 }),
                    responsive: true,
                }
            }
        }
    }
}
