use dioxus::prelude::*;

#[component]
pub fn DisplayMovie() -> Element {
    rsx! {
        div {
            "Movie title:"
        }
    }
}
