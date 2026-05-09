use std::env::var;
use std::sync::Arc;

use app_data::AppData;
use dioxus::prelude::*;
pub mod app_data;
pub mod components;
pub mod views;

use dotenvy::dotenv;
use log::error;
pub use views::insert_media::InsertMedia;
pub use views::model_pull::ModelPull;
pub use views::stats::Stats;
pub use views::{ai_chat::AiChat, ai_live_results::AiLiveResults, movies_list::MoviesList};

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(Navbar)]
    #[route("/")]
    Home {},
    #[route("/movies/list")]
    MoviesList {},
    #[route("/ai/chat")]
    AiChat {},
    #[route("/ai/live")]
    AiLiveResults {},
    #[route("/moives/new")]
    InsertMedia {},
    #[route("/ai/models")]
    ModelPull {},
    #[route("/stats")]
    Stats {},
}

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");
const HEADER_SVG: Asset = asset!("/assets/header.svg");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");
const MOVIE_GRID_CSS: Asset = asset!("/assets/movie_grid.css");
const CHAT_CSS: Asset = asset!("/assets/chat.css");
const AI_LIVE_RESULTS_CSS: Asset = asset!("/assets/ai_live_results.css");
const STATS_CSS: Asset = asset!("/assets/stats.css");

fn main() {
    console_error_panic_hook::set_once();
    wasm_logger::init(wasm_logger::Config::default());
    
    if let Err(e) = dotenv() {
        error!(
            "Error loading .env file: {}",
            e
        );
        // Handle error gracefully
    }

    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    // Create state
    let mut app_data = use_signal(
        || AppData {
            movies: Signal::new(None),
            ai_chat: Signal::new(None),
        },
    );

    // Provide context to children
    use_context_provider(|| app_data);

    // Async data fetching (non-blocking)
    use_future(
        move || async move {
            // Use the current page's hostname instead of hardcoded localhost
            let window = web_sys::window().unwrap();
            let location = window.location();
            let hostname = location
                .hostname()
                .unwrap_or_else(|_| "127.0.0.1".to_string());
            let server_port = var("server_port").unwrap_or_else(|_| "3000".to_string());
            let api_url = format!("http://{hostname}:{server_port}/dvd");

            if let Ok(response) = reqwest::get(&api_url).await {
                if let Ok(movies) = response
                    .json()
                    .await
                {
                    app_data
                        .write()
                        .movies
                        .set(Some(Arc::new(movies)));
                }
            }
        },
    );

    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }
        document::Link { rel: "stylesheet", href: MOVIE_GRID_CSS }
        document::Link { rel: "stylesheet", href: CHAT_CSS }
        document::Link { rel: "stylesheet", href: AI_LIVE_RESULTS_CSS }
        document::Link { rel: "stylesheet", href: STATS_CSS }
        Router::<Route> {}
    }
}

#[component]
pub fn Hero() -> Element {
    rsx! {
        div {
            id: "hero",
            img { src: HEADER_SVG, id: "header" }
            div { id: "links",
                Link {
                    to: Route::MoviesList {}, "List"
                }
                Link {
                    to: Route::AiChat {  }, "Chat"
                }
                Link {
                    to: Route::AiLiveResults {}, "Live"
                }
                Link {
                    to: Route::InsertMedia {}, "Insert"
                }
                Link {
                    to: Route::ModelPull {}, "Models"
                }
                Link {
                    to: Route::Stats {}, "Stats"
                }
            }
        }
    }
}

/// Home page
#[component]
fn Home() -> Element {
    rsx! {
        Hero {}
    }
}
/// Shared navbar component.
#[component]
fn Navbar() -> Element {
    rsx! {
        div {
            id: "navbar",
            Link {
                to: Route::Home {},
                "Home"
            }
            Link {
                to: Route::MoviesList {  },
                "List"
            }
            Link {
                to: Route::AiChat {  }, "Chat"
            }
            Link {
                to: Route::AiLiveResults {}, "Live"
            }
            Link {
                    to: Route::InsertMedia {}, "Insert"
            }
            Link {
                    to: Route::ModelPull {}, "Models"
            }
            Link {
                    to: Route::Stats {}, "Stats"
            }
        }

        Outlet::<Route> {}
    }
}
