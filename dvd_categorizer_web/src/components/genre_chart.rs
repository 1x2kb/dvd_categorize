use dioxus::prelude::*;
use dioxus_grapher::PieChart;
use models::PieChartData;

#[component]
pub fn GenreChart(data: ReadSignal<PieChartData>) -> Element {
    // Responsive sizing: 500px desktop, scaled on mobile
    let chart_size = web_sys::window()
        .and_then(
            |w| {
                w.inner_width()
                    .ok()
            },
        )
        .and_then(|w| w.as_f64())
        .map(
            |width| {
                if width < 768.0 {
                    (width * 0.9).min(400.0)
                } else {
                    500.0
                }
            },
        )
        .unwrap_or(500.0);

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
                style: "width: 100%; display: flex; justify-content: center;",
                PieChart {
                    data: data().data,
                    size: chart_size,
                }
            }
        }
    }
}
