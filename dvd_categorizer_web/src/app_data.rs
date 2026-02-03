use dioxus::signals::Signal;
use models::{FullMovie, RoledMessage};
use std::sync::Arc;

// Global state structure
#[derive(Clone)]
pub struct AppData {
    pub movies: Signal<Option<Arc<Vec<FullMovie>>>>,
    pub ai_chat: Signal<Option<Vec<RoledMessage>>>,
}
