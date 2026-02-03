pub use dioxus::prelude::*;
use models::FullMovie;

#[component]
pub fn MovieCard(movie: ReadSignal<FullMovie>) -> Element {
    let actors = movie()
        .actors
        .iter()
        .map(
            |actor| {
                actor
                    .name
                    .as_str()
            },
        )
        .collect::<Vec<_>>()
        .join(", ");

    let director_display = movie()
        .director
        .as_ref()
        .map(
            |d| {
                format!(
                    "Directed by: {}",
                    d.name
                )
            },
        )
        .unwrap_or_default();

    let genres = movie()
        .genres
        .join(",");

    rsx! {
        div {
            key: "{movie().id}",

            span {
                h2 {
                    "{movie().name}"
                }

                if let Some(description) = movie().description {
                    p {
                        "{description}"
                    }
                }

                p {
                    "Starring: {actors}"
                }

                p {
                    "Genres: {genres}"
                }

                // Conditional director section
                if !director_display.is_empty() {
                    p { "{director_display}" }
                }
            }
        }
    }
}
