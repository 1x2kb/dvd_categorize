use dioxus::prelude::*;
use dioxus::document::eval;
use models::FullMovie;
use serde_json::Value;
use std::sync::Arc;

const ITEM_HEIGHT: f64 = 400.0; // Approximate height of each movie card in pixels
const ITEMS_PER_ROW: usize = 3; // Number of items per row in the grid
const BUFFER_ROWS: usize = 2; // Number of extra rows to render above/below viewport

#[derive(Props, Clone, PartialEq)]
pub struct VirtualMovieGridProps {
    pub movies: Arc<Vec<FullMovie>>,
}

#[component]
pub fn VirtualMovieGrid(props: VirtualMovieGridProps) -> Element {
    let mut scroll_top = use_signal(|| 0.0);
    let mut viewport_height = use_signal(|| 800.0); // Default viewport height
    
    let movies = &props.movies;
    let total_items = movies.len();
    
    // Calculate total number of rows
    let total_rows = (total_items + ITEMS_PER_ROW - 1) / ITEMS_PER_ROW;
    let total_height = total_rows as f64 * ITEM_HEIGHT;
    
    // Calculate which rows should be visible
    let scroll_pos = scroll_top();
    let viewport_h = viewport_height();
    
    let start_row = ((scroll_pos / ITEM_HEIGHT).floor() as usize).saturating_sub(BUFFER_ROWS);
    let visible_rows = ((viewport_h / ITEM_HEIGHT).ceil() as usize) + (BUFFER_ROWS * 2);
    let end_row = (start_row + visible_rows).min(total_rows);
    
    // Calculate which items to render
    let start_index = start_row * ITEMS_PER_ROW;
    let end_index = (end_row * ITEMS_PER_ROW).min(total_items);
    
    // Height of the spacer before visible items
    let offset_y = start_row as f64 * ITEM_HEIGHT;
    
    // Get the visible slice of movies
    let visible_movies: Vec<FullMovie> = movies[start_index..end_index].to_vec();
    let visible_movies_arc = Arc::new(visible_movies);
    
    // Use effect to setup scroll listener and calculate available height
    use_effect(move || {
        spawn(async move {
            let script = r#"
                const container = document.getElementById('virtual-scroll-container');
                if (container) {
                    // Calculate available height: viewport height - navbar - margins
                    const navbar = document.getElementById('navbar');
                    const navbarHeight = navbar ? navbar.offsetHeight : 0;
                    const viewportHeight = window.innerHeight;
                    const availableHeight = viewportHeight - navbarHeight - 40; // 40px for margins (20px * 2)
                    
                    // Set the container height
                    container.style.height = availableHeight + 'px';
                    
                    dioxus.send({ viewport_height: availableHeight });
                    
                    container.addEventListener('scroll', () => {
                        dioxus.send({ scroll_top: container.scrollTop });
                    });
                    
                    // Update height on window resize
                    window.addEventListener('resize', () => {
                        const newViewportHeight = window.innerHeight;
                        const newAvailableHeight = newViewportHeight - navbarHeight - 40;
                        container.style.height = newAvailableHeight + 'px';
                        dioxus.send({ viewport_height: newAvailableHeight });
                    });
                }
            "#;
            
            let mut eval = eval(script);
            
            loop {
                if let Ok(data) = eval.recv::<Value>().await {
                    if let Some(height) = data.get("viewport_height") {
                        if let Some(h) = height.as_f64() {
                            viewport_height.set(h);
                        }
                    }
                    if let Some(st) = data.get("scroll_top") {
                        if let Some(s) = st.as_f64() {
                            scroll_top.set(s);
                        }
                    }
                }
            }
        });
    });
    
    rsx! {
        div {
            id: "virtual-scroll-container",
            class: "virtual-scroll-container",
            style: "overflow-y: auto; position: relative;",
            
            // Total height container to maintain scroll area
            div {
                style: "height: {total_height}px; position: relative;",
                
                // Offset container for visible items
                div {
                    style: "position: absolute; top: {offset_y}px; left: 0; right: 0;",
                    
                    // Render visible movies using the same grid structure as MovieGrid
                    div {
                        class: "movie-grid movie-grid-cols-3",
                        for movie in visible_movies_arc.iter() {
                            div {
                                class: "movie-card",

                                // Movie poster placeholder
                                div {
                                    class: "movie-poster",
                                    "{movie.name}"
                                }

                                // Movie details
                                div {
                                    class: "movie-details",

                                    // Title
                                    h3 {
                                        class: "movie-title",
                                        "{movie.name}"
                                    }

                                    // Description
                                    if let Some(description) = &movie.description {
                                        p {
                                            class: "movie-description",
                                            "{description}"
                                        }
                                    }

                                    // Actors
                                    if !movie.actors.is_empty() {
                                        div {
                                            class: "movie-info-row",
                                            span {
                                                class: "movie-info-label",
                                                "Cast: "
                                            }
                                            span {
                                                class: "movie-info-value",
                                                {
                                                    movie.actors.iter().map(|a| a.name.as_str()).collect::<Vec<_>>().join(", ")
                                                }
                                            }
                                        }
                                    }

                                    // Director
                                    if let Some(director) = &movie.director {
                                        div {
                                            class: "movie-info-row",
                                            span {
                                                class: "movie-info-label",
                                                "Director: "
                                            }
                                            span {
                                                class: "movie-info-value",
                                                "{director.name}"
                                            }
                                        }
                                    }

                                    // Genres
                                    if !movie.genres.is_empty() {
                                        div {
                                            class: "movie-genres-container",
                                            div {
                                                class: "movie-genres",
                                                // Show first 4 genres normally
                                                for genre in movie.genres.iter().take(4) {
                                                    span {
                                                        key: "{genre}",
                                                        class: "movie-genre-tag",
                                                        "{genre}"
                                                    }
                                                }
                                                // Show additional genres (hidden by default, shown on hover)
                                                if movie.genres.len() > 4 {
                                                    for genre in movie.genres.iter().skip(4) {
                                                        span {
                                                            key: "{genre}",
                                                            class: "movie-genre-tag movie-genre-extra",
                                                            "{genre}"
                                                        }
                                                    }
                                                    span {
                                                        class: "movie-genre-tag movie-genre-more",
                                                        "+{movie.genres.len() - 4}"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
