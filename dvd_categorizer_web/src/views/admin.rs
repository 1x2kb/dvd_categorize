use dioxus::prelude::*;

#[component]
pub fn AdminPage() -> Element {
    rsx! {
        div { class: "admin-container",
            h1 { class: "page-title", "Admin Panel" }

            div { class: "admin-section",
                h2 { "Ollama Model Management" }

                p { "Model management features will appear here." }

                button {
                    class: "button button-primary",
                    "Unload All Models"
                }

                div {
                    class: "status-message",
                    p { "Ready" }
                }
            }
        }
    }
}
