use app_data::AppData;
use dioxus::prelude::*;
pub mod app_data;
pub mod components;
pub mod views;

use dotenvy::dotenv;
use log::error;
pub use views::insert_media::InsertMedia;
pub use views::model_pull::ModelPull;
pub use views::movie_generator::MoviePrompt;
pub use views::stats::Stats;
pub use views::{ai_chat::AiChat, ai_live_results::AiLiveResults};

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(Navbar)]
    #[route("/")]
    Home {},
    #[route("/ai/chat")]
    AiChat {},
    #[route("/ai/live")]
    AiLiveResults {},
    #[route("/movies/new")]
    InsertMedia {},
    #[route("/ai/models")]
    ModelPull {},
    #[route("/generator")]
    MoviePrompt {},
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
    let app_data = use_signal(
        || AppData {
            ai_chat: Signal::new(None),
            chat_session_id: Signal::new(None),
            chat_sessions: Signal::new(Vec::new()),
        },
    );

    // Provide context to children
    use_context_provider(|| app_data);

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
                    to: Route::AiChat {  }, "Chat"
                }
                Link {
                    to: Route::AiLiveResults {}, "Live"
                }
                Link {
                    to: Route::InsertMedia {}, "Insert"
                }
                Link {
                    to: Route::MoviePrompt {}, "Generator"
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
                to: Route::AiChat {  }, "Chat"
            }
            Link {
                to: Route::AiLiveResults {}, "Live"
            }
            Link {
                    to: Route::InsertMedia {}, "Insert"
            }
            Link {
                    to: Route::MoviePrompt {}, "Generator"
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
