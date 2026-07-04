use dioxus::prelude::*;
use gloo_timers::future::sleep;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, HtmlElement, MouseEvent, Window};

pub fn get_window() -> Option<Window> {
    web_sys::window()
}

/// Poll the bounding client rect of the given element every 100ms and return
/// its current size as a reactive signal. The future is cancelled when the
/// component is dropped.
pub fn use_container_size(element: Signal<Option<HtmlElement>>) -> Signal<(f64, f64)> {
    let mut size = use_signal(|| (0.0, 0.0));

    use_future(move || async move {
        loop {
            if let Some(el) = element() {
                let rect = el.get_bounding_client_rect();
                let width = rect.width();
                let height = rect.height();
                if width > 0.0 && height > 0.0 {
                    let current = size();
                    if (current.0 - width).abs() > 0.5 || (current.1 - height).abs() > 0.5 {
                        size.set((width, height));
                    }
                }
            }
            sleep(std::time::Duration::from_millis(100)).await;
        }
    });

    size
}

pub fn device_pixel_ratio() -> f64 {
    get_window().map(|w| w.device_pixel_ratio()).unwrap_or(1.0)
}

pub fn get_canvas_context(canvas: &HtmlCanvasElement) -> Option<CanvasRenderingContext2d> {
    canvas
        .get_context("2d")
        .ok()
        .flatten()
        .and_then(|c| c.dyn_into::<CanvasRenderingContext2d>().ok())
}

/// Resize the canvas backing store to match the requested CSS size at the
/// current device pixel ratio, then scale the 2D context so all drawing calls
/// can use CSS-pixel coordinates.
pub fn setup_canvas(canvas: &HtmlCanvasElement, css_width: f64, css_height: f64) -> Option<CanvasRenderingContext2d> {
    let dpr = device_pixel_ratio().max(1.0);
    let physical_width = (css_width * dpr).round() as u32;
    let physical_height = (css_height * dpr).round() as u32;

    canvas.set_width(physical_width);
    canvas.set_height(physical_height);

    let style = canvas.style();
    style.set_property("width", &format!("{}px", css_width)).ok()?;
    style.set_property("height", &format!("{}px", css_height)).ok()?;
    style.set_property("display", "block").ok()?;

    let ctx = get_canvas_context(canvas)?;
    ctx.set_transform(1.0, 0.0, 0.0, 1.0, 0.0, 0.0).ok()?;
    ctx.scale(dpr, dpr).ok()?;
    Some(ctx)
}

/// Convert a MouseEvent client coordinate to CSS-pixel coordinates relative to
/// the canvas element.
pub fn mouse_pos(canvas: &HtmlCanvasElement, event: &MouseEvent) -> (f64, f64) {
    let rect = canvas.get_bounding_client_rect();
    let x = (event.client_x() as f64 - rect.left()) * (canvas.width() as f64 / rect.width());
    let y = (event.client_y() as f64 - rect.top()) * (canvas.height() as f64 / rect.height());
    // After the DPR scaling, logical coordinates are what we want.
    (x / device_pixel_ratio(), y / device_pixel_ratio())
}

pub fn clear_canvas(ctx: &CanvasRenderingContext2d, css_width: f64, css_height: f64) {
    let dpr = device_pixel_ratio().max(1.0);
    ctx.set_transform(1.0, 0.0, 0.0, 1.0, 0.0, 0.0).ok();
    ctx.clear_rect(0.0, 0.0, css_width * dpr, css_height * dpr);
    ctx.scale(dpr, dpr).ok();
}

pub fn hex_to_rgb(hex: &str) -> (u8, u8, u8) {
    let hex = hex.trim_start_matches('#');
    let hex = if hex.len() == 3 {
        hex.chars()
            .map(|c| format!("{c}{c}"))
            .collect::<String>()
    } else {
        hex.to_string()
    };
    if hex.len() == 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
        (r, g, b)
    } else {
        (0, 0, 0)
    }
}

pub fn with_alpha(hex: &str, alpha: f64) -> String {
    let (r, g, b) = hex_to_rgb(hex);
    format!("rgba({}, {}, {}, {})", r, g, b, alpha)
}
