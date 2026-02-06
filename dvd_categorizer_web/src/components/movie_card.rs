use chrono::{DateTime, Local, NaiveDateTime};
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

                // Added On timestamp
                if let Some(added_on) = &movie().added_on {
                    p {
                        style: "font-size: 0.9em; color: #888; margin-top: 8px;",
                        "Added: "
                        {
                            // Parse the timestamp and format it to local time
                            if let Ok(naive_dt) = NaiveDateTime::parse_from_str(added_on, "%Y-%m-%d %H:%M:%S%.f") {
                                let utc_dt = DateTime::<chrono::Utc>::from_naive_utc_and_offset(naive_dt, chrono::Utc);
                                let local_dt: DateTime<Local> = DateTime::from(utc_dt);
                                local_dt.format("%B %d, %Y at %I:%M %p").to_string()
                            } else {
                                // Fallback to raw string if parsing fails
                                added_on.clone()
                            }
                        }
                    }
                }

                // Location
                if let Some(location) = &movie().location {
                    p {
                        style: "font-size: 0.9em; font-weight: 600; color: #0891b2; margin-top: 4px;",
                        "Location: {location}"
                    }
                }
            }
        }
    }
}
