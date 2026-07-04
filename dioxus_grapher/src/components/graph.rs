use dioxus::prelude::*;
use dioxus::web::WebEventExt;
use wasm_bindgen::JsCast;
use web_sys::{HtmlCanvasElement, HtmlElement};

use crate::canvas_utils::{setup_canvas, use_container_size, with_alpha};

#[derive(Clone, PartialEq, Props)]
pub struct GraphProps {
    pub points: Vec<(f64, f64)>,
    #[props(default = 600.0)]
    pub width: f64,
    #[props(default = 400.0)]
    pub height: f64,
    #[props(default = String::from("#3b82f6"))]
    pub stroke_color: String,
    #[props(default = 2.0)]
    pub stroke_width: f64,
    #[props(default = true)]
    pub responsive: bool,
}

#[component]
pub fn Graph(props: GraphProps) -> Element {
    let mut canvas_ref = use_signal(|| None::<HtmlCanvasElement>);
    let mut container_ref = use_signal(|| None::<HtmlElement>);
    let mut hovered_index = use_signal(|| None::<usize>);
    let container_size = use_container_size(container_ref);

    let width = props.width;
    let height = props.height;
    let stroke_color = props.stroke_color.clone();
    let stroke_width = props.stroke_width;
    let points_for_effect = props.points.clone();
    let points_for_handlers = props.points.clone();

    let (canvas_width, canvas_height) = if props.responsive {
        let (w, h) = container_size();
        if w > 0.0 && h > 0.0 {
            (w, h)
        } else {
            (width, height)
        }
    } else {
        (width, height)
    };

    use_effect(move || {
        let _ = points_for_effect.clone();
        let _ = hovered_index();
        let (current_w, current_h) = container_size();
        let _ = canvas_ref();
        if let Some(canvas) = canvas_ref() {
            let (w, h) = if props.responsive && current_w > 0.0 && current_h > 0.0 {
                (current_w, current_h)
            } else {
                (width, height)
            };
            let props = GraphProps {
                points: points_for_effect.clone(),
                width: w,
                height: h,
                stroke_color: stroke_color.clone(),
                stroke_width,
                responsive: false,
            };
            draw_graph(&canvas, &props, hovered_index());
        }
    });

    if props.points.is_empty() {
        let style = if props.responsive {
            "width: 100%; height: 100%; border: 1px solid #ccc;"
        } else {
            &format!("width: {}px; height: {}px; border: 1px solid #ccc;", width, height)
        };
        return rsx! {
            div {
                class: "graph-container",
                style: "{style}",
                "No data points"
            }
        };
    }

    let points_enter = points_for_handlers.clone();
    let points_move = points_for_handlers.clone();

    let container_style = if props.responsive {
        "width: 100%; height: 100%;"
    } else {
        &format!("width: {}px; height: {}px;", width, height)
    };

    rsx! {
        div {
            class: "graph-container",
            style: "{container_style}",
            onmounted: move |e: Event<MountedData>| {
                let element = e.as_web_event().dyn_into::<HtmlElement>().ok();
                container_ref.set(element);
            },
            canvas {
                width: "{canvas_width}",
                height: "{canvas_height}",
                onmounted: move |e: Event<MountedData>| {
                    let element = e.as_web_event().dyn_into::<HtmlCanvasElement>().ok();
                    canvas_ref.set(element);
                },
                onmouseenter: move |e: Event<MouseData>| {
                    if let Some(canvas) = canvas_ref() {
                        update_hover(&canvas, &e.as_web_event(), &points_enter, hovered_index);
                    }
                },
                onmousemove: move |e: Event<MouseData>| {
                    if let Some(canvas) = canvas_ref() {
                        update_hover(&canvas, &e.as_web_event(), &points_move, hovered_index);
                    }
                },
                onmouseleave: move |_| {
                    hovered_index.set(None);
                },
            }
        }
    }
}

fn calculate_bounds(points: &[(f64, f64)]) -> (f64, f64, f64, f64) {
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for (x, y) in points {
        min_x = min_x.min(*x);
        max_x = max_x.max(*x);
        min_y = min_y.min(*y);
        max_y = max_y.max(*y);
    }

    (min_x, max_x, min_y, max_y)
}

fn project_point(
    point: (f64, f64),
    min_x: f64,
    min_y: f64,
    scale_x: f64,
    scale_y: f64,
    padding: f64,
    height: f64,
) -> (f64, f64) {
    (
        padding + (point.0 - min_x) * scale_x,
        height - padding - (point.1 - min_y) * scale_y,
    )
}

fn draw_graph(canvas: &HtmlCanvasElement, props: &GraphProps, hovered_index: Option<usize>) {
    let Some(ctx) = setup_canvas(canvas, props.width, props.height) else {
        return;
    };

    let (min_x, max_x, min_y, max_y) = calculate_bounds(&props.points);
    let padding = 20.0;
    let graph_width = props.width - 2.0 * padding;
    let graph_height = props.height - 2.0 * padding;

    let scale_x = if max_x != min_x {
        graph_width / (max_x - min_x)
    } else {
        1.0
    };
    let scale_y = if max_y != min_y {
        graph_height / (max_y - min_y)
    } else {
        1.0
    };

    // Grid lines
    ctx.set_stroke_style_str("#e5e7eb");
    ctx.set_line_dash(&wasm_bindgen::JsValue::from_str("4,4")).ok();
    for i in 0..5 {
        let grid_y = padding + (i as f64 * graph_height / 4.0);
        ctx.begin_path();
        ctx.move_to(padding, grid_y);
        ctx.line_to(props.width - padding, grid_y);
        ctx.stroke();
    }
    ctx.set_line_dash(&wasm_bindgen::JsValue::from_str("")).ok();

    let projected: Vec<(f64, f64)> = props
        .points
        .iter()
        .map(|p| project_point(*p, min_x, min_y, scale_x, scale_y, padding, props.height))
        .collect();

    if projected.len() < 2 {
        return;
    }

    // Area gradient
    let area_gradient = ctx.create_linear_gradient(0.0, padding, 0.0, props.height - padding);
    area_gradient.add_color_stop(0.0, &with_alpha(&props.stroke_color, 0.3)).ok();
    area_gradient.add_color_stop(1.0, &with_alpha(&props.stroke_color, 0.05)).ok();

    ctx.begin_path();
    ctx.move_to(projected[0].0, projected[0].1);
    for (x, y) in projected.iter().skip(1) {
        ctx.line_to(*x, *y);
    }
    ctx.line_to(props.width - padding, props.height - padding);
    ctx.line_to(padding, props.height - padding);
    ctx.close_path();
    ctx.set_fill_style_canvas_gradient(&area_gradient);
    ctx.fill();

    // Line gradient
    let line_gradient = ctx.create_linear_gradient(0.0, padding, 0.0, props.height - padding);
    line_gradient.add_color_stop(0.0, &props.stroke_color).ok();
    line_gradient.add_color_stop(0.5, &with_alpha(&props.stroke_color, 0.9)).ok();
    line_gradient.add_color_stop(1.0, &with_alpha(&props.stroke_color, 0.7)).ok();

    ctx.begin_path();
    ctx.move_to(projected[0].0, projected[0].1);
    for (x, y) in projected.iter().skip(1) {
        ctx.line_to(*x, *y);
    }
    ctx.set_stroke_style_canvas_gradient(&line_gradient);
    ctx.set_line_width(props.stroke_width + 1.0);
    ctx.set_line_cap("round");
    ctx.set_line_join("round");
    ctx.stroke();

    // Data points
    for (i, (x, y)) in projected.iter().enumerate() {
        let is_hovered = hovered_index == Some(i);
        let radius = if is_hovered { 8.0 } else { 6.0 };

        ctx.begin_path();
        ctx.arc(*x, *y, radius, 0.0, std::f64::consts::PI * 2.0)
            .ok();
        ctx.set_fill_style_str(&with_alpha(&props.stroke_color, 0.3));
        ctx.fill();

        ctx.begin_path();
        ctx.arc(*x, *y, radius - 2.0, 0.0, std::f64::consts::PI * 2.0)
            .ok();
        ctx.set_fill_style_str(&props.stroke_color);
        ctx.fill();

        ctx.begin_path();
        ctx.arc(*x, *y, 2.0, 0.0, std::f64::consts::PI * 2.0)
            .ok();
        ctx.set_fill_style_str("white");
        ctx.fill();
    }
}

fn update_hover(
    canvas: &HtmlCanvasElement,
    event: &web_sys::MouseEvent,
    points: &[(f64, f64)],
    mut hovered_index: Signal<Option<usize>>,
) {
    let (min_x, max_x, min_y, max_y) = calculate_bounds(points);
    let padding = 20.0;
    let graph_width = canvas.width() as f64 / crate::canvas_utils::device_pixel_ratio() - 2.0 * padding;
    let graph_height = canvas.height() as f64 / crate::canvas_utils::device_pixel_ratio() - 2.0 * padding;
    let scale_x = if max_x != min_x {
        graph_width / (max_x - min_x)
    } else {
        1.0
    };
    let scale_y = if max_y != min_y {
        graph_height / (max_y - min_y)
    } else {
        1.0
    };
    let height = canvas.height() as f64 / crate::canvas_utils::device_pixel_ratio();

    let rect = canvas.get_bounding_client_rect();
    let x = (event.client_x() as f64 - rect.left()) * (canvas.width() as f64 / rect.width()) / crate::canvas_utils::device_pixel_ratio();
    let y = (event.client_y() as f64 - rect.top()) * (canvas.height() as f64 / rect.height()) / crate::canvas_utils::device_pixel_ratio();

    let mut closest = None;
    let mut best_dist = 64.0; // 8px hit radius

    for (i, (px, py)) in points.iter().enumerate() {
        let (sx, sy) = project_point((*px, *py), min_x, min_y, scale_x, scale_y, padding, height);
        let dx = sx - x;
        let dy = sy - y;
        let dist = dx * dx + dy * dy;
        if dist < best_dist {
            best_dist = dist;
            closest = Some(i);
        }
    }

    hovered_index.set(closest);
}
