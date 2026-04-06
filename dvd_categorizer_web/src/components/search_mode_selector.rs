use dioxus::prelude::*;
use models::SearchMode;

#[derive(Props, Clone, PartialEq)]
pub struct SearchModeSelectorProps {
    pub search_mode: SearchMode,
    pub on_mode_change: EventHandler<SearchMode>,
    pub selected_model: Option<String>,
    pub on_model_change: EventHandler<Option<String>>,
    pub available_models: Signal<Vec<models::AvailableModel>>,
}

#[component]
pub fn SearchModeSelector(props: SearchModeSelectorProps) -> Element {
    rsx! {
        div {
            class: "search-mode-tabs",

            span {
                class: "mode-label",
                "Mode:"
            }

            button {
                class: if matches!(props.search_mode, SearchMode::Text) { "mode-button active" } else { "mode-button" },
                onclick: move |_| props.on_mode_change.call(SearchMode::Text),
                "Text"
            }

            button {
                class: if matches!(props.search_mode, SearchMode::Both) { "mode-button active" } else { "mode-button" },
                onclick: move |_| props.on_mode_change.call(SearchMode::Both),
                "Both"
            }

            button {
                class: if matches!(props.search_mode, SearchMode::Vector) { "mode-button active" } else { "mode-button" },
                onclick: move |_| props.on_mode_change.call(SearchMode::Vector),
                "Vector"
            }

            button {
                class: if matches!(props.search_mode, SearchMode::Structured) { "mode-button active" } else { "mode-button" },
                onclick: move |_| props.on_mode_change.call(SearchMode::Structured),
                "Structured"
            }

            // Model selector (hidden for Text mode)
            if !matches!(props.search_mode, SearchMode::Text) {
                div {
                    class: "model-selector-container",

                    span {
                        class: "model-label",
                        "Model:"
                    }

                    select {
                        class: "model-select",
                        value: match props.selected_model {
                            Some(ref model) => model.clone(),
                            None => "default".to_string(),
                        },
                        onchange: move |evt| {
                            let value = evt.value();
                            if value == "default" {
                                props.on_model_change.call(None);
                            } else {
                                props.on_model_change.call(Some(value));
                            }
                        },

                        option { value: "default", "Default" }
                        for model in props.available_models.read().iter() {
                            option {
                                value: "{model.name}",
                                "{model.name}"
                            }
                        }
                    }
                }
            }
        }
    }
}
