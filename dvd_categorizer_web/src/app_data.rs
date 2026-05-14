use dioxus::signals::Signal;
use models::RoledMessage;

// Global state structure
#[derive(Clone)]
pub struct AppData {
    pub ai_chat: Signal<Option<Vec<RoledMessage>>>,
}
