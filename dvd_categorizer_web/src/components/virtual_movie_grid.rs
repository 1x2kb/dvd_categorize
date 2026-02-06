use dioxus::document::eval;
use dioxus::prelude::*;
use models::FullMovie;
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use web_sys::wasm_bindgen::JsCast;
use web_sys::window;

const ITEM_HEIGHT: f64 = 400.0; // Approximate height of each movie card in pixels
const ITEMS_PER_ROW: usize = 3; // Number of items per row in the grid
const BUFFER_ROWS: usize = 4; // Number of extra rows to render above/below viewport for smooth scrolling

#[derive(Props, Clone, PartialEq)]
pub struct VirtualMovieGridProps {
    pub movies: Arc<Vec<FullMovie>>,
}

// TODO: AI Coded. REVIEW!!!
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

    // Initialize container height on mount and handle window resize
    use_effect(
        move || {
            spawn(
                async move {
                    // Function to calculate and set height
                    let calculate_height = || async move {
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
                        
                        return availableHeight;
                    }
                    return 800;
                "#;

                        if let Ok(result) = eval(script).await {
                            if let Some(height) = result.as_f64() {
                                viewport_height.set(height);
                            }
                        }
                    };

                    // Set initial height
                    calculate_height().await;

                    // Add resize listener using web_sys for proper state updates
                    if let Some(win) = window() {
                        // Create synchronous resize handler to ensure immediate updates
                        let closure = Closure::wrap(
                            Box::new(
                                move || {
                                    if let Some(resize_window) = window() {
                                        if let Some(document) = resize_window.document() {
                                            if let Some(container) = document
                                                .get_element_by_id("virtual-scroll-container")
                                            {
                                                if let Some(html_container) =
                                                    container.dyn_ref::<web_sys::HtmlElement>()
                                                {
                                                    // Get navbar height
                                                    let navbar_height = document
                                                        .get_element_by_id("navbar")
                                                        .and_then(
                                                            |nav| {
                                                                nav.dyn_ref::<web_sys::HtmlElement>(
                                                                )
                                                                .map(|e| e.offset_height())
                                                            },
                                                        )
                                                        .unwrap_or(0);

                                                    // Calculate new height
                                                    let viewport_height_val = resize_window
                                                        .inner_height()
                                                        .ok()
                                                        .and_then(|h| h.as_f64())
                                                        .unwrap_or(800.0);
                                                    let available_height = viewport_height_val
                                                        - navbar_height as f64
                                                        - 40.0;

                                                    // Update CSS immediately
                                                    let height_style = format!(
                                                        "{}px",
                                                        available_height
                                                    );
                                                    let _ = html_container
                                                        .style()
                                                        .set_property(
                                                            "height",
                                                            &height_style,
                                                        );

                                                    // Update Rust state immediately
                                                    viewport_height.set(available_height);
                                                }
                                            }
                                        }
                                    }
                                },
                            ) as Box<dyn FnMut()>,
                        );

                        let _ = win.add_event_listener_with_callback(
                            "resize",
                            closure
                                .as_ref()
                                .unchecked_ref(),
                        );
                        closure.forget(); // Keep the closure alive for the component lifetime
                    }
                },
            );
        },
    );

    // Throttle scroll updates using requestAnimationFrame
    let mut scroll_pending = use_signal(|| false);

    let handle_scroll = move |_evt: Event<ScrollData>| {
        // Only schedule an update if one isn't already pending
        if !scroll_pending() {
            scroll_pending.set(true);

            if let Some(win) = window() {
                let closure = Closure::once(
                    Box::new(
                        move || {
                            // Update scroll position on next animation frame
                            if let Some(window) = window() {
                                if let Some(document) = window.document() {
                                    if let Some(element) =
                                        document.get_element_by_id("virtual-scroll-container")
                                    {
                                        if let Some(html_element) =
                                            element.dyn_ref::<web_sys::HtmlElement>()
                                        {
                                            let scroll_val = html_element.scroll_top() as f64;
                                            scroll_top.set(scroll_val);
                                        }
                                    }
                                }
                            }
                            scroll_pending.set(false);
                        },
                    ) as Box<dyn FnOnce()>,
                );

                let _ = win.request_animation_frame(
                    closure
                        .as_ref()
                        .unchecked_ref(),
                );
                closure.forget();
            }
        }
    };

    rsx! {
        div {
            id: "virtual-scroll-container",
            class: "virtual-scroll-container",
            style: "overflow-y: auto; position: relative;",
            onscroll: handle_scroll,

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
