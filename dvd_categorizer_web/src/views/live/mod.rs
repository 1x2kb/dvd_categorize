//! One component per live page view. Each owns its own fetch, state and chrome;
//! the page only decides which one is mounted.

pub mod by_location;
pub mod random_movies;
pub mod recent_movies;
pub mod recent_releases;
pub mod search_results;
pub mod unknown_location;

use dioxus::prelude::*;

pub use by_location::ByLocationView;
pub use random_movies::RandomMovies;
pub use recent_movies::RecentMovies;
pub use recent_releases::RecentReleases;
pub use search_results::SearchResults;
pub use unknown_location::UnknownLocation;

/// Which view the page is showing.
#[derive(Clone, PartialEq, Eq)]
pub enum LiveView {
    Idle,
    Recent,
    RecentReleases,
    Random,
    UnknownLocation,
    ByLocation(String),
    Search(String),
}

/// Props shared by the simple browse views.
#[derive(Props, Clone, PartialEq)]
pub struct BrowseViewProps {
    pub focused: Signal<Option<u64>>,
    pub is_loading: Signal<bool>,
}
