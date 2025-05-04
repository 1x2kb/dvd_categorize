use dioxus::prelude::*;

use crate::components::display_movie::DisplayMovie;

#[component]
pub fn MoviesList() -> Element {
    rsx! {
        DisplayMovie {}
    }
}
