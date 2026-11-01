// search_bar.rs
use dioxus::prelude::*;
use models::SearchMode;

#[derive(Props, Clone, PartialEq)]
pub struct SearchBarProps {
    pub input_value: Signal<String>,
    pub on_search: EventHandler<()>,
    pub is_loading: Signal<bool>,
    pub search_mode: Signal<SearchMode>,
    pub disable_enhancement: Signal<bool>,
    pub on_enhancement_toggle: EventHandler<bool>,
    pub enhanced_query: Signal<String>,
    pub original_query: Signal<String>,
}

#[component]
pub fn SearchBar(mut props: SearchBarProps) -> Element {
    let loading = *props.is_loading.read();

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
                    oninput: move |evt| props.input_value.set(evt.value()),
                    onkeypress: move |evt| {
                        if evt.key() == Key::Enter && !loading {
                            props.on_search.call(());
                        }
                    }
                }

                button {
                    class: "search-button",
                    onclick: move |_| {
                        if !loading {
                            props.on_search.call(());
                        }
                    },
                    disabled: loading,

                    if loading {
                        "Searching..."
                    } else {
                        "Search"
                    }
                }
            }

            // Checkbox for disabling AI enhancement (only show for Vector/Both modes)
            if !matches!(*props.search_mode.read(), SearchMode::Text) {
                div {
                    class: "enhancement-checkbox-container",
                    input {
                        r#type: "checkbox",
                        id: "disable-enhancement",
                        checked: *props.disable_enhancement.read(),
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
            if !props.enhanced_query.read().is_empty()
                && *props.enhanced_query.read() != *props.original_query.read()
            {
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
