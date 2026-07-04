use dioxus::prelude::*;
use dioxus::web::WebEventExt;
use wasm_bindgen::JsCast;
use web_sys::{HtmlCanvasElement, HtmlElement};

use crate::canvas_utils::{setup_canvas, use_container_size, with_alpha};

#[derive(Clone, PartialEq)]
pub struct AutoOptions {
    pub skip_labels: usize,
}

impl Default for AutoOptions {
    fn default() -> Self {
        Self { skip_labels: 1 }
    }
}

#[derive(Clone, PartialEq)]
pub struct ExplicitOptions {
    pub labels: Vec<String>,
    pub skip_labels: usize,
}

impl Default for ExplicitOptions {
    fn default() -> Self {
        Self {
            labels: vec![],
            skip_labels: 1,
        }
    }
}

#[derive(Clone, PartialEq)]
pub enum XAxisMode {
    Auto(AutoOptions),
    Explicit(ExplicitOptions),
}

impl Default for XAxisMode {
    fn default() -> Self {
        Self::Auto(AutoOptions::default())
    }
}

#[derive(Clone, PartialEq, Props)]
pub struct BarGraphProps {
    pub data: Vec<(String, f64)>,
    #[props(default = 600.0)]
    pub width: f64,
    #[props(default = 400.0)]
    pub height: f64,
    #[props(default = String::from("#3b82f6"))]
    pub bar_color: String,
    #[props(default = String::from("X Axis"))]
    pub x_label: String,
    #[props(default = String::from("Y Axis"))]
    pub y_label: String,
    #[props(default = XAxisMode::default())]
    pub x_mode: XAxisMode,
    #[props(default = None)]
    pub y_min: Option<f64>,
    #[props(default = None)]
    pub y_max: Option<f64>,
    #[props(default = true)]
    pub responsive: bool,
}

#[component]
pub fn BarGraph(props: BarGraphProps) -> Element {
    let mut canvas_ref = use_signal(|| None::<HtmlCanvasElement>);
    let mut container_ref = use_signal(|| None::<HtmlElement>);
    let mut hovered_index = use_signal(|| None::<usize>);
    let container_size = use_container_size(container_ref);

    let width = props.width;
    let height = props.height;
    let bar_color = props.bar_color.clone();
    let x_label = props.x_label.clone();
    let y_label = props.y_label.clone();
    let x_mode = props.x_mode.clone();
    let y_min = props.y_min;
    let y_max = props.y_max;
    let data_for_effect = props.data.clone();
    let data_for_handlers = props.data.clone();

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
        let _ = data_for_effect.clone();
        let _ = hovered_index();
        let (current_w, current_h) = container_size();
        let _ = canvas_ref();
        if let Some(canvas) = canvas_ref() {
            let (w, h) = if props.responsive && current_w > 0.0 && current_h > 0.0 {
                (current_w, current_h)
            } else {
                (width, height)
            };
            let props = BarGraphProps {
                data: data_for_effect.clone(),
                width: w,
                height: h,
                bar_color: bar_color.clone(),
                x_label: x_label.clone(),
                y_label: y_label.clone(),
                x_mode: x_mode.clone(),
                y_min,
                y_max,
                responsive: false,
            };
            draw_bar_graph(&canvas, &props, hovered_index());
        }
    });

    if props.data.is_empty() {
        let style = if props.responsive {
            "width: 100%; height: 100%; border: 1px solid #ccc;"
        } else {
            &format!("width: {}px; height: {}px; border: 1px solid #ccc;", width, height)
        };
        return rsx! {
            div {
                class: "bar-graph-container",
                style: "{style}",
                "No data"
            }
        };
    }

    let data_move = data_for_handlers.clone();

    let container_style = if props.responsive {
        "width: 100%; height: 100%;"
    } else {
        &format!("width: {}px; height: {}px;", width, height)
    };

    rsx! {
        div {
            class: "bar-graph-container",
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
                onmousemove: move |e: Event<MouseData>| {
                    if let Some(canvas) = canvas_ref() {
                        update_bar_hover(&canvas, &e.as_web_event(), &data_move, hovered_index);
                    }
                },
                onmouseleave: move |_| {
                    hovered_index.set(None);
                },
            }
        }
    }
}

fn draw_bar_graph(canvas: &HtmlCanvasElement, props: &BarGraphProps, hovered_index: Option<usize>) {
    let Some(ctx) = setup_canvas(canvas, props.width, props.height) else {
        return;
    };

    let padding = 60.0;
    let graph_width = props.width - 2.0 * padding;
    let graph_height = props.height - 2.0 * padding;

    let max_value = props.y_max.unwrap_or_else(|| {
        props.data.iter().map(|(_, v)| *v).fold(f64::NEG_INFINITY, f64::max)
    });
    let min_value = props.y_min.unwrap_or_else(|| {
        props.data.iter().map(|(_, v)| *v).fold(f64::INFINITY, f64::min).min(0.0)
    });

    let value_range = max_value - min_value;
    let scale_y = if value_range > 0.0 {
        graph_height / value_range
    } else {
        1.0
    };

    let bar_count = props.data.len() as f64;
    let bar_spacing = graph_width / bar_count;
    let bar_width = bar_spacing * 0.8;
    let y_ticks = 5;
    let tick_step = value_range / y_ticks as f64;

    // Background
    ctx.set_fill_style_str("transparent");
    ctx.fill_rect(0.0, 0.0, props.width, props.height);

    // Axes
    ctx.set_stroke_style_str("#374151");
    ctx.set_line_width(2.0);
    ctx.begin_path();
    ctx.move_to(padding, padding);
    ctx.line_to(padding, props.height - padding);
    ctx.move_to(padding, props.height - padding);
    ctx.line_to(props.width - padding, props.height - padding);
    ctx.stroke();

    // Y-axis ticks and labels
    ctx.set_font("12px sans-serif");
    ctx.set_fill_style_str("#9ca3af");
    ctx.set_text_align("right");
    ctx.set_text_baseline("middle");
    for i in 0..=y_ticks {
        let value = min_value + (i as f64 * tick_step);
        let y_pos = props.height - padding - ((value - min_value) * scale_y);
        ctx.begin_path();
        ctx.move_to(padding - 5.0, y_pos);
        ctx.line_to(padding, y_pos);
        ctx.stroke();
        ctx.fill_text(&format!("{:.0}", value), padding - 10.0, y_pos + 4.0)
            .ok();
    }

    // Bar gradient
    let bar_gradient = ctx.create_linear_gradient(0.0, padding, 0.0, props.height - padding);
    bar_gradient.add_color_stop(0.0, &props.bar_color).ok();
    bar_gradient.add_color_stop(1.0, &with_alpha(&props.bar_color, 0.8)).ok();

    // Bars and X-axis labels
    let base_skip = match &props.x_mode {
        XAxisMode::Auto(opts) => opts.skip_labels,
        XAxisMode::Explicit(opts) => opts.skip_labels,
    };

    // Auto-skip labels when bars become too narrow for the text to fit.
    // A label needs roughly 60 CSS pixels. Skip so we don't show more than
    // one label per 60px of bar width.
    let auto_skip = (60.0 / bar_width).ceil().max(1.0) as usize;
    let desired_skip = base_skip.max(auto_skip).min(props.data.len().max(1));

    ctx.set_font("12px sans-serif");
    ctx.set_text_align("center");
    ctx.set_text_baseline("top");

    let mut bar_regions: Vec<(usize, f64, f64, f64, f64)> = Vec::new();

    for (idx, (label, value)) in props.data.iter().enumerate() {
        let x_pos = padding + (idx as f64 * bar_spacing) + (bar_spacing - bar_width) / 2.0;
        let bar_height = (value - min_value) * scale_y;
        let y_pos = props.height - padding - bar_height;

        bar_regions.push((idx, x_pos, y_pos, bar_width, bar_height));

        let display_label = match &props.x_mode {
            XAxisMode::Explicit(opts) => opts.labels.get(idx).cloned().unwrap_or_else(|| label.clone()),
            XAxisMode::Auto(_) => label.clone(),
        };

        let is_hovered = hovered_index == Some(idx);
        let fill_color = if is_hovered { "#06b6d4" } else { "" };

        // Hover guide line
        if is_hovered {
            ctx.set_stroke_style_str("#06b6d4");
            ctx.set_line_width(2.0);
            ctx.set_line_dash(&wasm_bindgen::JsValue::from_str("4,4")).ok();
            ctx.begin_path();
            ctx.move_to(x_pos + bar_width / 2.0, y_pos);
            ctx.line_to(x_pos + bar_width / 2.0, props.height - padding);
            ctx.stroke();
            ctx.set_line_dash(&wasm_bindgen::JsValue::from_str("")).ok();
        }

        // Bar fill
        if is_hovered {
            ctx.set_fill_style_str(&fill_color);
        } else {
            ctx.set_fill_style_canvas_gradient(&bar_gradient);
        }
        round_rect(&ctx, x_pos, y_pos, bar_width, bar_height.max(0.0), 6.0);
        ctx.fill();

        // Bar border
        ctx.set_stroke_style_str(if is_hovered { "#06b6d4" } else { &props.bar_color });
        ctx.set_line_width(if is_hovered { 3.0 } else { 2.0 });
        ctx.set_global_alpha(if is_hovered { 0.8 } else { 0.3 });
        round_rect(&ctx, x_pos, y_pos, bar_width, bar_height.max(8.0), 6.0);
        ctx.stroke();
        ctx.set_global_alpha(1.0);

        // Value label
        ctx.set_font("bold 11px sans-serif");
        ctx.set_global_alpha(1.0);
        ctx.set_text_align("center");
        let value_text = format!("{:.0}", value);
        let label_y = if bar_height > 20.0 {
            // Draw inside the bar so it never clips the canvas edge.
            ctx.set_fill_style_str("#ffffff");
            ctx.set_text_baseline("middle");
            y_pos + 12.0
        } else {
            // Short bar: draw above the bar.
            ctx.set_fill_style_str("#9ca3af");
            ctx.set_text_baseline("bottom");
            y_pos - 8.0
        };
        ctx.fill_text(&value_text, x_pos + bar_width / 2.0, label_y)
            .ok();

        // X-axis label
        if idx % desired_skip == 0 {
            ctx.set_fill_style_str("#9ca3af");
            ctx.set_font("12px sans-serif");
            ctx.set_text_baseline("top");
            ctx.fill_text(&display_label, x_pos + bar_width / 2.0, props.height - padding + 20.0)
                .ok();
        }
    }

    // Axis labels
    ctx.set_fill_style_str("#374151");
    ctx.set_font("bold 14px sans-serif");
    ctx.set_text_align("center");
    ctx.set_text_baseline("alphabetic");
    ctx.fill_text(&props.x_label, props.width / 2.0, props.height - 10.0)
        .ok();

    ctx.save();
    ctx.translate(15.0, props.height / 2.0).ok();
    ctx.rotate(-std::f64::consts::PI / 2.0).ok();
    ctx.fill_text(&props.y_label, 0.0, 0.0).ok();
    ctx.restore();

    // Tooltip
    if let Some(idx) = hovered_index {
        if let Some((label, value)) = props.data.get(idx) {
            let display_label = match &props.x_mode {
                XAxisMode::Explicit(opts) => opts.labels.get(idx).cloned().unwrap_or_else(|| label.clone()),
                XAxisMode::Auto(_) => label.clone(),
            };
            draw_tooltip(&ctx, props.width / 2.0, 30.0, &display_label, &format!("Value: {:.0}", value), &props.bar_color);
        }
    }
}

fn round_rect(ctx: &web_sys::CanvasRenderingContext2d, x: f64, y: f64, width: f64, height: f64, radius: f64) {
    let r = radius.min(width / 2.0).min(height / 2.0);
    ctx.begin_path();
    ctx.move_to(x + r, y);
    ctx.line_to(x + width - r, y);
    ctx.quadratic_curve_to(x + width, y, x + width, y + r);
    ctx.line_to(x + width, y + height - r);
    ctx.quadratic_curve_to(x + width, y + height, x + width - r, y + height);
    ctx.line_to(x + r, y + height);
    ctx.quadratic_curve_to(x, y + height, x, y + height - r);
    ctx.line_to(x, y + r);
    ctx.quadratic_curve_to(x, y, x + r, y);
    ctx.close_path();
}

fn draw_tooltip(
    ctx: &web_sys::CanvasRenderingContext2d,
    x: f64,
    y: f64,
    label: &str,
    value: &str,
    stroke_color: &str,
) {
    ctx.set_font("bold 14px sans-serif");
    let label_metrics = ctx.measure_text(label).ok();
    ctx.set_font("12px sans-serif");
    let value_metrics = ctx.measure_text(value).ok();

    let label_width = label_metrics.map(|m| m.width()).unwrap_or(0.0);
    let value_width = value_metrics.map(|m| m.width()).unwrap_or(0.0);
    let content_width = label_width.max(value_width);
    let width = content_width + 40.0;
    let height = 40.0;

    let rx = 8.0;
    ctx.set_fill_style_str("#1f2937");
    round_rect(ctx, x - width / 2.0, y - height / 2.0, width, height, rx);
    ctx.fill();

    ctx.set_stroke_style_str(stroke_color);
    ctx.set_line_width(2.0);
    round_rect(ctx, x - width / 2.0, y - height / 2.0, width, height, rx);
    ctx.stroke();

    ctx.set_fill_style_str("#ffffff");
    ctx.set_font("bold 14px sans-serif");
    ctx.set_text_align("center");
    ctx.set_text_baseline("middle");
    ctx.fill_text(label, x, y - 5.0).ok();

    ctx.set_fill_style_str("#d1d5db");
    ctx.set_font("12px sans-serif");
    ctx.fill_text(value, x, y + 10.0).ok();
}

fn update_bar_hover(
    canvas: &HtmlCanvasElement,
    event: &web_sys::MouseEvent,
    data: &[(String, f64)],
    mut hovered_index: Signal<Option<usize>>,
) {
    let padding = 60.0;
    let width = canvas.width() as f64 / crate::canvas_utils::device_pixel_ratio();
    let graph_width = width - 2.0 * padding;
    let bar_count = data.len() as f64;
    let bar_spacing = graph_width / bar_count;
    let bar_width = bar_spacing * 0.8;

    let rect = canvas.get_bounding_client_rect();
    let x = (event.client_x() as f64 - rect.left()) * (canvas.width() as f64 / rect.width())
        / crate::canvas_utils::device_pixel_ratio();

    let mut found = None;
    for (idx, _) in data.iter().enumerate() {
        let x_pos = padding + (idx as f64 * bar_spacing) + (bar_spacing - bar_width) / 2.0;
        if x >= x_pos && x <= x_pos + bar_width {
            found = Some(idx);
            break;
        }
    }

    hovered_index.set(found);
}
