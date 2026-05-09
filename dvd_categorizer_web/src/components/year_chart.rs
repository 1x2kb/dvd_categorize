use dioxus::prelude::*;
use dioxus_grapher::BarGraph;
use models::BarChartData;

#[component]
pub fn YearChart(
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
            h2 { "Top 15 Years by Movie Count" }
            BarGraph {
                data: chart_data,
                width,
                height,
                bar_color: "#0891b2".to_string(),
                x_label: "Year".to_string(),
                y_label: "Movies".to_string(),
                skip_labels: 1,
            }
        }
    }
}
