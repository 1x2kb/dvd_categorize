use dioxus::prelude::*;
use dioxus::web::WebEventExt;
use wasm_bindgen::JsCast;
use web_sys::{HtmlCanvasElement, HtmlElement};

use crate::canvas_utils::{setup_canvas, use_container_size, with_alpha};

#[derive(Clone, PartialEq)]
struct SliceData {
    label: String,
    value: f64,
    percentage: f64,
    start_angle: f64,
    end_angle: f64,
    color: String,
    index: usize,
}

#[derive(Clone, PartialEq, Props)]
pub struct PieChartProps {
    pub data: Vec<(String, f64)>,
    #[props(default = 400.0)]
    pub size: f64,
    #[props(default = vec![
        "#1e40af".to_string(),
        "#0f766e".to_string(),
        "#b91c1c".to_string(),
        "#0369a1".to_string(),
        "#1e3a8a".to_string(),
        "#ca8a04".to_string(),
        "#164e63".to_string(),
        "#92400e".to_string(),
    ])]
    pub colors: Vec<String>,
    #[props(default = true)]
    pub show_legend: bool,
    #[props(default = None)]
    pub top_n: Option<usize>,
    #[props(default = None)]
    pub min_percentage: Option<f64>,
    #[props(default = true)]
    pub responsive: bool,
}

#[component]
pub fn PieChart(props: PieChartProps) -> Element {
    let mut canvas_ref = use_signal(|| None::<HtmlCanvasElement>);
    let mut container_ref = use_signal(|| None::<HtmlElement>);
    let mut hovered_index = use_signal(|| None::<usize>);
    let container_size = use_container_size(container_ref);

    let size = props.size;
    let colors = props.colors.clone();
    let show_legend = props.show_legend;
    let data_for_effect = props.data.clone();

    let canvas_size = if props.responsive {
        let (w, h) = container_size();
        if w > 0.0 && h > 0.0 {
            w.min(h)
        } else {
            size
        }
    } else {
        size
    };

    let filtered_data = use_memo(move || {
        let mut data = data_for_effect.clone();
        data.sort_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
        if let Some(n) = props.top_n {
            data.truncate(n);
        }
        let total: f64 = data.iter().map(|(_, v)| v).sum();
        if let Some(min_pct) = props.min_percentage {
            data.retain(|(_, v)| (*v / total * 100.0) >= min_pct);
        }
        data
    });

    let slices = use_memo(move || {
        let data = filtered_data.read();
        let total: f64 = data.iter().map(|(_, v)| v).sum();
        if total <= 0.0 {
            return vec![];
        }

        let mut current_angle = -90.0_f64;
        let mut result = Vec::new();
        for (idx, (label, value)) in data.iter().enumerate() {
            let percentage = (value / total) * 100.0;
            let angle = (value / total) * 360.0;
            let end_angle = current_angle + angle;
            let color = colors
                .get(idx % colors.len())
                .cloned()
                .unwrap_or_else(|| "#999999".to_string());

            result.push(SliceData {
                label: label.clone(),
                value: *value,
                percentage,
                start_angle: current_angle,
                end_angle,
                color,
                index: idx,
            });
            current_angle = end_angle;
        }
        result
    });

    use_effect(move || {
        let _ = hovered_index();
        let (current_w, current_h) = container_size();
        let _ = canvas_ref();
        if let Some(canvas) = canvas_ref() {
            let s = if props.responsive && current_w > 0.0 && current_h > 0.0 {
                current_w.min(current_h)
            } else {
                size
            };
            draw_pie_chart(&canvas, s, &slices.read(), hovered_index());
        }
    });

    if props.data.is_empty() {
        let style = if props.responsive {
            "width: 100%; height: 100%; border: 1px solid #ccc;"
        } else {
            &format!("width: {}px; height: {}px; border: 1px solid #ccc;", size, size)
        };
        return rsx! {
            div {
                class: "pie-chart-container",
                style: "{style}",
                "No data"
            }
        };
    }

    let total: f64 = filtered_data.read().iter().map(|(_, v)| v).sum();
    if total <= 0.0 {
        let style = if props.responsive {
            "width: 100%; height: 100%; border: 1px solid #ccc;"
        } else {
            &format!("width: {}px; height: {}px; border: 1px solid #ccc;", size, size)
        };
        return rsx! {
            div {
                class: "pie-chart-container",
                style: "{style}",
                "Invalid data (total must be > 0)"
            }
        };
    }

    let slices_legend = slices.read().clone();
    let slices_enter = slices_legend.clone();
    let slices_move = slices_legend.clone();

    let container_style = if props.responsive {
        "width: 100%; height: 100%; display: flex; align-items: center; gap: 30px; font-family: sans-serif;"
    } else {
        &format!("width: {}px; height: {}px; display: flex; align-items: center; gap: 30px; font-family: sans-serif;", size, size)
    };

    rsx! {
        div {
            style: "{container_style}",
            onmounted: move |e: Event<MountedData>| {
                let element = e.as_web_event().dyn_into::<HtmlElement>().ok();
                container_ref.set(element);
            },

            canvas {
                width: "{canvas_size}",
                height: "{canvas_size}",
                style: "overflow: visible;",
                onmounted: move |e: Event<MountedData>| {
                    let element = e.as_web_event().dyn_into::<HtmlCanvasElement>().ok();
                    canvas_ref.set(element);
                },
                onmouseenter: move |e: Event<MouseData>| {
                    if let Some(canvas) = canvas_ref() {
                        update_pie_hover(&canvas, &e.as_web_event(), &slices_enter, hovered_index);
                    }
                },
                onmousemove: move |e: Event<MouseData>| {
                    if let Some(canvas) = canvas_ref() {
                        update_pie_hover(&canvas, &e.as_web_event(), &slices_move, hovered_index);
                    }
                },
                onmouseleave: move |_| {
                    hovered_index.set(None);
                },
            }

            if show_legend {
                div {
                    style: "display: flex; flex-direction: column; gap: 12px;",

                    for slice in slices_legend.iter() {
                        {
                            let is_hovered = hovered_index() == Some(slice.index);
                            let bg_color = if is_hovered { "#f3f4f6" } else { "transparent" };
                            let transform_val = if is_hovered { "translateX(4px)" } else { "translateX(0)" };
                            let scale_val = if is_hovered { "scale(1.1)" } else { "scale(1)" };
                            let idx = slice.index;
                            let label = &slice.label;
                            let value = slice.value;
                            let percentage = slice.percentage;
                            let color = &slice.color;

                            rsx! {
                                div {
                                    key: "{idx}",
                                    style: "display: flex; align-items: center; gap: 12px; cursor: pointer; padding: 10px; border-radius: 8px; background: {bg_color}; transition: all 0.2s ease; transform: {transform_val};",
                                    onmouseenter: move |_| hovered_index.set(Some(idx)),
                                    onmouseleave: move |_| hovered_index.set(None),

                                    div {
                                        style: "width: 20px; height: 20px; background: {color}; border-radius: 6px; box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1); transition: all 0.2s ease; transform: {scale_val};",
                                    }
                                    div {
                                        style: "font-size: 14px;",
                                        span {
                                            style: "font-weight: bold;",
                                            "{label}"
                                        }
                                        span {
                                            style: "color: #6b7280; margin-left: 5px;",
                                            "{value:.2} ({percentage:.1}%)"
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

fn draw_pie_chart(canvas: &HtmlCanvasElement, size: f64, slices: &[SliceData], hovered_index: Option<usize>) {
    let Some(ctx) = setup_canvas(canvas, size, size) else {
        return;
    };

    let center_x = size / 2.0;
    let center_y = size / 2.0;
    let radius = (size / 2.0) * 0.7;

    ctx.clear_rect(0.0, 0.0, size, size);

    for slice in slices.iter() {
        let is_hovered = hovered_index == Some(slice.index);
        let offset = if is_hovered { 10.0 } else { 0.0 };
        let mid_angle = (slice.start_angle + slice.end_angle) / 2.0_f64;
        let offset_x = offset * mid_angle.to_radians().cos();
        let offset_y = offset * mid_angle.to_radians().sin();
        let cx = center_x + offset_x;
        let cy = center_y + offset_y;

        let (start_x, start_y) = angle_to_point(cx, cy, radius, slice.start_angle);

        // Gradient fill
        let gradient = ctx.create_radial_gradient(cx - radius * 0.3, cy - radius * 0.3, radius * 0.1, cx, cy, radius)
            .unwrap_or_else(|_| ctx.create_radial_gradient(cx, cy, 0.0, cx, cy, radius).unwrap_or_else(|_| panic!("radial gradient")));
        gradient.add_color_stop(0.0, &with_alpha(&slice.color, 0.95)).ok();
        gradient.add_color_stop(0.5, &slice.color).ok();
        gradient.add_color_stop(1.0, &with_alpha(&slice.color, 0.85)).ok();

        ctx.begin_path();
        ctx.move_to(cx, cy);
        ctx.line_to(start_x, start_y);
        ctx.arc(cx, cy, radius, slice.start_angle.to_radians(), slice.end_angle.to_radians()).ok();
        ctx.line_to(cx, cy);
        ctx.close_path();
        ctx.set_fill_style_canvas_gradient(&gradient);
        ctx.set_stroke_style_str("white");
        ctx.set_line_width(3.0);
        ctx.set_global_alpha(if is_hovered { 1.0 } else { 0.95 });
        ctx.fill();
        ctx.stroke();

        // Shine overlay
        let shine = ctx.create_radial_gradient(cx - radius * 0.3, cy - radius * 0.3, 0.0, cx, cy, radius)
            .unwrap_or_else(|_| ctx.create_radial_gradient(cx, cy, 0.0, cx, cy, radius).unwrap_or_else(|_| panic!("radial gradient")));
        shine.add_color_stop(0.0, "rgba(255, 255, 255, 0.4)").ok();
        shine.add_color_stop(1.0, "rgba(255, 255, 255, 0)").ok();

        ctx.begin_path();
        ctx.move_to(cx, cy);
        ctx.line_to(start_x, start_y);
        ctx.arc(cx, cy, radius, slice.start_angle.to_radians(), slice.end_angle.to_radians()).ok();
        ctx.line_to(cx, cy);
        ctx.close_path();
        ctx.set_fill_style_canvas_gradient(&shine);
        ctx.set_global_alpha(1.0);
        ctx.fill();

        // Percentage label
        let label_radius = radius * 0.65;
        let label_x = cx + label_radius * mid_angle.to_radians().cos();
        let label_y = cy + label_radius * mid_angle.to_radians().sin();
        ctx.set_fill_style_str("white");
        ctx.set_font("bold 14px sans-serif");
        ctx.set_text_align("center");
        ctx.set_text_baseline("middle");
        ctx.fill_text(&format!("{:.1}%", slice.percentage), label_x, label_y).ok();
    }

    // Tooltip
    if let Some(idx) = hovered_index {
        if let Some(slice) = slices.get(idx) {
            draw_tooltip(
                &ctx,
                center_x,
                30.0,
                &slice.label,
                &format!("Value: {:.2} ({:.1}%)", slice.value, slice.percentage),
                &slice.color,
            );
        }
    }
}

fn angle_to_point(cx: f64, cy: f64, radius: f64, angle_deg: f64) -> (f64, f64) {
    let rad = angle_deg.to_radians();
    (cx + radius * rad.cos(), cy + radius * rad.sin())
}

fn draw_tooltip(
    ctx: &web_sys::CanvasRenderingContext2d,
    x: f64,
    y: f64,
    label: &str,
    value: &str,
    _stroke_color: &str,
) {
    ctx.set_font("bold 13px sans-serif");
    let label_width = ctx.measure_text(label).ok().map(|m| m.width()).unwrap_or(0.0);
    ctx.set_font("11px sans-serif");
    let value_width = ctx.measure_text(value).ok().map(|m| m.width()).unwrap_or(0.0);
    let content_width = label_width.max(value_width);
    let width = content_width + 40.0;
    let height = 40.0;
    let rx = 5.0;

    ctx.set_fill_style_str("rgba(0, 0, 0, 0.9)");
    round_rect(ctx, x - width / 2.0, y - height / 2.0, width, height, rx);
    ctx.fill();

    ctx.set_stroke_style_str("white");
    ctx.set_line_width(1.0);
    round_rect(ctx, x - width / 2.0, y - height / 2.0, width, height, rx);
    ctx.stroke();

    ctx.set_fill_style_str("white");
    ctx.set_font("bold 13px sans-serif");
    ctx.set_text_align("center");
    ctx.set_text_baseline("middle");
    ctx.fill_text(label, x, y - 8.0).ok();

    ctx.set_font("11px sans-serif");
    ctx.fill_text(value, x, y + 8.0).ok();
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

fn update_pie_hover(
    canvas: &HtmlCanvasElement,
    event: &web_sys::MouseEvent,
    slices: &[SliceData],
    mut hovered_index: Signal<Option<usize>>,
) {
    let size = canvas.width() as f64 / crate::canvas_utils::device_pixel_ratio();
    let center_x = size / 2.0;
    let center_y = size / 2.0;
    let radius = (size / 2.0) * 0.7;

    let rect = canvas.get_bounding_client_rect();
    let x = (event.client_x() as f64 - rect.left()) * (canvas.width() as f64 / rect.width())
        / crate::canvas_utils::device_pixel_ratio();
    let y = (event.client_y() as f64 - rect.top()) * (canvas.height() as f64 / rect.height())
        / crate::canvas_utils::device_pixel_ratio();

    let dx = x - center_x;
    let dy = y - center_y;
    let distance = (dx * dx + dy * dy).sqrt();
    let angle = dy.atan2(dx).to_degrees();
    let normalized_angle = ((angle + 90.0) % 360.0 + 360.0) % 360.0;

    let mut found = None;
    if distance <= radius {
        for slice in slices.iter() {
            let start = ((slice.start_angle + 90.0) % 360.0 + 360.0) % 360.0;
            let end = ((slice.end_angle + 90.0) % 360.0 + 360.0) % 360.0;
            if if start < end {
                normalized_angle >= start && normalized_angle < end
            } else {
                normalized_angle >= start || normalized_angle < end
            } {
                found = Some(slice.index);
                break;
            }
        }
    }

    hovered_index.set(found);
}
