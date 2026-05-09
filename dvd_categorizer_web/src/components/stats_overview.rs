use dioxus::prelude::*;
use models::StatsOverview;

#[component]
pub fn StatsOverviewCard(overview: ReadSignal<StatsOverview>) -> Element {
    rsx! {
        div {
            class: "stats-grid",
            
            div {
                class: "stat-card",
                h2 { "Total Movies" }
                p { class: "stat-value", "{overview().total_movies}" }
            }
            
            div {
                class: "stat-card",
                h2 { "Total Directors" }
                p { class: "stat-value", "{overview().total_directors}" }
            }
            
            div {
                class: "stat-card",
                h2 { "Total Actors" }
                p { class: "stat-value", "{overview().total_actors}" }
            }
        }
    }
}
