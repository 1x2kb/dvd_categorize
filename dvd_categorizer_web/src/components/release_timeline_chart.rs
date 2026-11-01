// release_timeline_chart.rs
use dioxus::prelude::*;
use dioxus_grapher::{AutoOptions, BarGraph, XAxisMode};
use models::ScoredMovie;

fn current_year() -> i32 {
    js_sys::Date::new_0().get_full_year() as i32
}

#[derive(Props, Clone, PartialEq)]
pub struct ReleaseTimelineChartProps {
    pub movies: ReadSignal<Vec<ScoredMovie>>,
    #[props(default = 15)]
    pub years_back: usize,
}

/// Release counts per year, most recent first. Counts come from the movies the
/// caller already loaded, so the chart always matches the grid beside it.
///
/// Bars are ordered by relevance: the leftmost is the current year and later
/// pages are progressively less relevant, so paging degrades sensibly.
#[component]
pub fn ReleaseTimelineChart(props: ReleaseTimelineChartProps) -> Element {
    let current = current_year();
    let years_back = props
        .years_back
        .max(1);
    let span = years_back as i32;

    // Contiguous descending years ending at the current one, so a gap shows as
    // an empty bar instead of the axis jumping between years that happen to exist.
    let chart_data = use_memo(
        move || {
            let movies = props
                .movies
                .read();
            (0..span)
                .map(
                    |offset| {
                        let year = current - offset;
                        let count = movies
                            .iter()
                            .filter(
                                |scored| {
                                    scored
                                        .movie
                                        .release_year
                                        == year
                                },
                            )
                            .count();
                        (
                            year.to_string(),
                            count as f64,
                        )
                    },
                )
                .collect::<Vec<_>>()
        },
    );

    let data = chart_data();
    let total: usize = data
        .iter()
        .map(|(_, count)| *count as usize)
        .sum();

    rsx! {
        div {
            class: "chart-section year-chart-responsive",
            style: "width: 100%;",
            h2 { "Releases by Year" }
            p {
                style: "font-size: 0.875rem; color: #9ca3af; margin-bottom: 0.5rem;",
                "Most recent first, back to {current - span + 1}. {total} in this view."
            }
            div {
                style: "width: 100%; height: 400px;",
                BarGraph {
                    data,
                    bar_color: "#0891b2".to_string(),
                    x_label: "Year".to_string(),
                    y_label: "Movies".to_string(),
                    x_mode: XAxisMode::Auto(AutoOptions { skip_labels: 1 }),
                    responsive: true,
                    mobile_max_items: Some(6),
                }
            }
        }
    }
}
