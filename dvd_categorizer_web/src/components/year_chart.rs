use dioxus::prelude::*;
use dioxus_grapher::{AutoOptions, BarGraph, XAxisMode};
use models::BarChartData;

#[component]
pub fn YearChart(
    data: ReadSignal<BarChartData>,
    #[props(default = 8)] mobile_max_items: usize,
    #[props(default = 8)] page_size: usize,
) -> Element {
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

    let mut current_page_size = use_signal(|| page_size);

    rsx! {
        div {
            class: "chart-section year-chart-responsive",
            style: "width: 100%;",
            div {
                class: "year-chart-header",
                h2 { "Top 15 Release Years by Movie Count" }
                label {
                    class: "year-chart-page-size",
                    "Items per page: "
                    select {
                        value: "{current_page_size()}",
                        onchange: move |e: Event<FormData>| {
                            if let Ok(size) = e.value().parse::<usize>() {
                                if size > 0 {
                                    current_page_size.set(size);
                                }
                            }
                        },
                        option { value: "2", "2" }
                        option { value: "5", "5" }
                        option { value: "8", "8" }
                        option { value: "10", "10" }
                        option { value: "15", "15" }
                        option { value: "20", "20" }
                    }
                }
            }
            div {
                style: "width: 100%; height: 400px;",
                BarGraph {
                    data: chart_data,
                    bar_color: "#0891b2".to_string(),
                    x_label: "Year".to_string(),
                    y_label: "Movies".to_string(),
                    x_mode: XAxisMode::Auto(AutoOptions { skip_labels: 1 }),
                    responsive: true,
                    mobile_max_items: Some(mobile_max_items),
                    page_size: Some(current_page_size()),
                }
            }
        }
    }
}
