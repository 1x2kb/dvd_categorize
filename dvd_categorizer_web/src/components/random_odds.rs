use dioxus::prelude::*;
use models::{RandomOddsItem, RandomSelectionOddsResponse};

#[component]
pub fn RandomOdds(data: ReadSignal<RandomSelectionOddsResponse>) -> Element {
    let odds = data();

    let format_pct = |probability: f64| {
        if odds.total_movies == 0 {
            "0%".to_string()
        } else {
            format!("{:.1}%", probability * 100.0)
        }
    };

    let format_one_in = |item: &RandomOddsItem| {
        if odds.total_movies == 0 || item.count == 0 {
            "—".to_string()
        } else {
            let per_movie_odds = item.per_movie_odds;
            if per_movie_odds < 1.0 {
                format!("1 in {:.2}", per_movie_odds)
            } else {
                format!("1 in {:.0}", per_movie_odds)
            }
        }
    };

    rsx! {
        if odds.total_movies > 0 {
            div {
                class: "random-odds-panel",
                h3 { "Your odds for this random pick" }
                p {
                    class: "random-odds-note",
                    "Total movies in catalog: {odds.total_movies} (random sample of {odds.random_count})"
                }

                div {
                    class: "random-odds-columns",
                    div {
                        class: "random-odds-column",
                        h4 { "Genres" }
                        ul {
                            for item in odds.genres {
                                li {
                                    key: "{item.label}",
                                    span { class: "random-odds-label", "{item.label}" }
                                    span { class: "random-odds-value", "{format_pct(item.per_movie_probability)}" }
                                }
                            }
                        }
                    }

                    div {
                        class: "random-odds-column",
                        h4 { "Actors" }
                        ul {
                            for item in odds.actors {
                                li {
                                    key: "{item.label}",
                                    span { class: "random-odds-label", "{item.label}" }
                                    span { class: "random-odds-value", "{format_one_in(&item)}" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
