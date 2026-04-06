use dioxus::prelude::*;
use models::SearchMode;

#[derive(Props, Clone, PartialEq)]
pub struct SearchBarProps {
    pub input_value: String,
    pub on_input_change: EventHandler<String>,
    pub on_search: EventHandler<()>,
    pub is_loading: bool,
    pub search_mode: SearchMode,
    pub disable_enhancement: bool,
    pub on_enhancement_toggle: EventHandler<bool>,
    pub enhanced_query: String,
    pub original_query: String,
}

#[component]
pub fn SearchBar(props: SearchBarProps) -> Element {
    rsx! {
        div {
            class: "search-input-area",
            div {
                class: "search-input-group",

                input {
                    class: "search-input",
                    r#type: "text",
                    placeholder: "Enter your search query...",
                    value: "{props.input_value}",
                    oninput: move |evt| props.on_input_change.call(evt.value()),
                    onkeypress: move |evt| {
                        if evt.key() == Key::Enter && !props.is_loading {
                            props.on_search.call(());
                        }
                    }
                }

                button {
                    class: "search-button",
                    onclick: move |_| {
                        if !props.is_loading {
                            props.on_search.call(());
                        }
                    },
                    disabled: props.is_loading,

                    if props.is_loading {
                        "Searching..."
                    } else {
                        "Search"
                    }
                }
            }

            // Checkbox for disabling AI enhancement (only show for Vector/Both modes)
            if !matches!(props.search_mode, SearchMode::Text) {
                div {
                    class: "enhancement-checkbox-container",
                    input {
                        r#type: "checkbox",
                        id: "disable-enhancement",
                        checked: props.disable_enhancement,
                        onchange: move |evt| {
                            props.on_enhancement_toggle.call(evt.checked());
                        }
                    }
                    label {
                        r#for: "disable-enhancement",
                        "Disable AI Enhancement"
                    }
                }
            }

            // Display enhanced query if different from original
            if !props.enhanced_query.is_empty() && props.enhanced_query != props.original_query {
                div {
                    class: "enhanced-query-display",
                    div {
                        class: "enhanced-query-title",
                        "AI Enhanced Query:"
                    }
                    div {
                        class: "enhanced-query-text",
                        "{props.enhanced_query}"
                    }
                }
            }
        }
    }
}
