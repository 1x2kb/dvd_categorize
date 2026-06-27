use app_data::AppData;
use dioxus::prelude::*;
pub mod app_data;
pub mod components;
pub mod views;

pub use views::insert_media::InsertMedia;
#[cfg(feature = "ai-backend")]
pub use views::model_pull::ModelPull;
#[cfg(feature = "ai-backend")]
pub use views::movie_generator::MoviePrompt;
pub use views::stats::Stats;
pub use views::ai_live_results::LiveResults;
#[cfg(feature = "ai-backend")]
pub use views::ai_chat::AiChat;

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(Navbar)]
        #[route("/ai/chat")]
        AiChat {},
        #[route("/ai/live")]
        LiveRedirect {},
        #[redirect("/", || Route::LiveResults {})]
        #[route("/live")]
        LiveResults {},
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
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");
const MOVIE_GRID_CSS: Asset = asset!("/assets/movie_grid.css");
const CHAT_CSS: Asset = asset!("/assets/chat.css");
const AI_LIVE_RESULTS_CSS: Asset = asset!("/assets/ai_live_results.css");
const STATS_CSS: Asset = asset!("/assets/stats.css");

fn main() {
    console_error_panic_hook::set_once();
    wasm_logger::init(wasm_logger::Config::default());

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
fn LiveRedirect() -> Element {
    let nav = navigator();
    use_effect(move || {
        nav.replace(Route::LiveResults {});
    });
    rsx! { div {} }
}

// Stub components when ai-backend feature is disabled
#[cfg(not(feature = "ai-backend"))]
#[component]
fn AiChat() -> Element {
    rsx! {
        div { class: "error-message",
            h1 { "AI Features Not Available" }
            p { "This application was compiled without AI backend support." }
            p { "Please use a build with the 'ai-backend' feature enabled." }
        }
    }
}

#[cfg(not(feature = "ai-backend"))]
#[component]
fn AiLiveResults() -> Element {
    rsx! {
        div { class: "error-message",
            h1 { "AI Features Not Available" }
            p { "This application was compiled without AI backend support." }
        }
    }
}

#[cfg(not(feature = "ai-backend"))]
#[component]
fn ModelPull() -> Element {
    rsx! {
        div { class: "error-message",
            h1 { "AI Features Not Available" }
            p { "This application was compiled without AI backend support." }
        }
    }
}

#[cfg(not(feature = "ai-backend"))]
#[component]
fn MoviePrompt() -> Element {
    rsx! {
        div { class: "error-message",
            h1 { "AI Features Not Available" }
            p { "This application was compiled without AI backend support." }
        }
    }
}

/// Helper function to render AI-specific navigation links
#[cfg(feature = "ai-backend")]
fn render_ai_links() -> Element {
    rsx! {
        Link {
            to: Route::AiChat {},
            "Chat"
        }
        Link {
            to: Route::MoviePrompt {}, "Generator"
        }
        Link {
            to: Route::ModelPull {}, "Models"
        }
    }
}

/// Shared navbar component.
#[component]
fn Navbar() -> Element {
    #[cfg(feature = "ai-backend")]
    let ai_links = render_ai_links();
    
    #[cfg(not(feature = "ai-backend"))]
    let ai_links = rsx! {};
    
    rsx! {
        div {
            id: "navbar",
            
            // Always-available links
            Link {
                to: Route::LiveResults {}, "Live"
            }
            Link {
                to: Route::InsertMedia {}, "Insert"
            }
            Link {
                to: Route::Stats {}, "Stats"
            }
            
            // empty when ai-backend feature is disabled can't include #cfg in rsx!
            { ai_links }
        }

        Outlet::<Route> {}
    }
}
