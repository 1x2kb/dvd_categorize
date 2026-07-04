use dioxus::prelude::*;
use models::{BarChartData, PieChartData};

#[component]
pub fn RandomOdds(
    genre_data: ReadSignal<PieChartData>,
    actor_data: ReadSignal<BarChartData>,
    total_movies: usize,
) -> Element {
    let total = total_movies;

    let genres = genre_data().data;
    let actors = actor_data();
    let actor_entries: Vec<(String, f64)> = actors
        .labels
        .into_iter()
        .zip(actors.values.into_iter())
        .collect();

    let format_pct = |count: f64| {
        if total == 0 {
            "0%".to_string()
        } else {
            format!("{:.1}%", (count / total as f64) * 100.0)
        }
    };

    let format_one_in = |count: f64| {
        if total == 0 || count <= 0.0 {
            "—".to_string()
        } else {
            let odds = total as f64 / count;
            if odds < 1.0 {
                format!("1 in {:.2}", odds)
            } else {
                format!("1 in {:.0}", odds)
            }
        }
    };

    rsx! {
        div {
            class: "random-odds-panel",
            h3 { "Your odds for this random pick" }
            p {
                class: "random-odds-note",
                "Total movies in catalog: {total}"
            }

            div {
                class: "random-odds-columns",
                div {
                    class: "random-odds-column",
                    h4 { "Genres" }
                    ul {
                        for (genre , count) in genres {
                            li {
                                key: "{genre}",
                                span { class: "random-odds-label", "{genre}" }
                                span { class: "random-odds-value", "{format_pct(count)}" }
                            }
                        }
                    }
                }

                div {
                    class: "random-odds-column",
                    h4 { "Actors" }
                    ul {
                        for (label , count) in actor_entries {
                            li {
                                key: "{label}",
                                span { class: "random-odds-label", "{label}" }
                                span { class: "random-odds-value", "{format_one_in(count)}" }
                            }
                        }
                    }
                }
            }
        }
    }
}
