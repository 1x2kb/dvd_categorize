use dioxus::signals::Signal;
use models::{FullMovie, RoledMessage};

// Global state structure
#[derive(Clone)]
pub struct AppData {
    pub movies: Signal<Option<Vec<FullMovie>>>, // Changed to match your API response
    pub ai_chat: Signal<Option<Vec<RoledMessage>>>,
}
