use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct BrowseBarProps {
    pub on_browse: EventHandler<()>,
    pub is_loading: bool,
}

#[component]
pub fn BrowseBar(props: BrowseBarProps) -> Element {
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
                disabled: props.is_loading,

                if props.is_loading {
                    "Loading..."
                } else {
                    "Recent Movies"
                }
            }
        }
    }
}
