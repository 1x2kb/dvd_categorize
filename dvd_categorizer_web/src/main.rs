use dioxus::prelude::*;
pub mod components;
pub mod views;

use dotenvy::dotenv;
// use models::FullMovie;
pub use views::movies_list::MoviesList;

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(Navbar)]
    #[route("/")]
    Home {},
    #[route("/blog/:id")]
    Blog { id: i32 },
    #[route("/movies/list")]
    MoviesList {}
}

// Global state structure
#[derive(Clone, Copy)]
struct AppData {
    movies: Signal<Option<Vec<String>>>,
}

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");
const HEADER_SVG: Asset = asset!("/assets/header.svg");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

fn main() {
    dotenv()
        .ok()
        .expect("Failed to run env reader");

    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    // let movies = use_signal(|| None);
    let server_host = std::env::var("server_host").unwrap_or_else(|_| "127.0.0.1".to_string());
    let server_port = std::env::var("server_port").unwrap_or_else(|_| "3000".to_string());

    // use_effect(
    //     move || async move {
    //         let fetched = reqwest::get(format!("http://{server_host}/{server_port}"))
    //             .await
    //             .unwrap()
    //             .json::<Vec<FullMovie>>()
    //             .await
    //             .unwrap();

    //         movies.set(Some(fetched));
    //     },
    // );

    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS } document::Link { rel: "stylesheet", href: TAILWIND_CSS }
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
                a { href: "https://dioxuslabs.com/learn/0.6/", "📚 Learn Dioxus" }
                Link {
                    to: Route::MoviesList {}, "List"
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

/// Blog page
#[component]
pub fn Blog(id: i32) -> Element {
    rsx! {
        div {
            id: "blog",

            // Content
            h1 { "This is blog #{id}!" }
            p { "In blog #{id}, we show how the Dioxus router works and how URL parameters can be passed as props to our route components." }

            // Navigation links
            Link {
                to: Route::Blog { id: id - 1 },
                "Previous"
            }
            span { " <---> " }
            Link {
                to: Route::Blog { id: id + 1 },
                "Next"
            }
        }
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
        }

        Outlet::<Route> {}
    }
}
