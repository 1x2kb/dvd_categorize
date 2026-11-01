// browse_bar.rs
use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct BrowseBarProps {
    pub on_browse: EventHandler<()>,
    pub on_recent_releases: EventHandler<()>,
    pub on_random: EventHandler<()>,
    pub on_unknown_location: EventHandler<()>,
    pub on_by_location: EventHandler<String>,
    pub locations: Signal<Vec<String>>,
    pub is_loading: Signal<bool>,
}

#[component]
pub fn BrowseBar(props: BrowseBarProps) -> Element {
    let mut selected_location = use_signal(String::new);
    let loading = *props.is_loading.read();

    rsx! {
        div {
            class: "browse-actions-bar",
            span {
                class: "browse-label",
                "Browse:"
            }
            button {
                class: "browse-button",
                onclick: move |_| props.on_browse.call(()),
                disabled: loading,

                if loading {
                    "Loading..."
                } else {
                    "Recently Added"
                }
            }
            button {
                class: "browse-button",
                onclick: move |_| props.on_recent_releases.call(()),
                disabled: loading,

                if loading {
                    "Loading..."
                } else {
                    "Recent Releases"
                }
            }
            button {
                class: "browse-button",
                onclick: move |_| props.on_random.call(()),
                disabled: loading,

                if loading {
                    "Loading..."
                } else {
                    "Random Movies"
                }
            }
            button {
                class: "browse-button",
                onclick: move |_| props.on_unknown_location.call(()),
                disabled: loading,

                if loading {
                    "Loading..."
                } else {
                    "Unknown Location"
                }
            }
            div {
                class: "location-browse-group",
                select {
                    class: "location-select",
                    disabled: loading || props.locations.read().is_empty(),
                    value: "{selected_location}",
                    onchange: move |evt| selected_location.set(evt.value()),
                    option {
                        value: "",
                        "Select location..."
                    }
                    for loc in props.locations.read().iter() {
                        option {
                            key: "{loc}",
                            value: "{loc}",
                            "{loc}"
                        }
                    }
                }
                button {
                    class: "browse-button",
                    onclick: move |_| {
                        let value = selected_location.read().clone();
                        if !value.is_empty() {
                            props.on_by_location.call(value);
                        }
                    },
                    disabled: loading || selected_location.read().is_empty(),

                    if loading {
                        "Loading..."
                    } else {
                        "Movies by Location"
                    }
                }
            }
        }
    }
}
