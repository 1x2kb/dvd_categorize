use dioxus::prelude::*;

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
    pub data: Vec<(
        String,
        f64,
    )>,
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
}

#[component]
pub fn PieChart(props: PieChartProps) -> Element {
    let mut hovered_index = use_signal(|| None::<usize>);

    if props
        .data
        .is_empty()
    {
        return rsx! {
            div {
                class: "pie-chart-container",
                style: "width: {props.size}px; height: {props.size}px; border: 1px solid #ccc;",
                "No data"
            }
        };
    }

    // Apply filters: sort by value descending, then apply top_n and min_percentage
    let mut filtered_data = props
        .data
        .clone();
    filtered_data.sort_by(
        |(_, a), (_, b)| {
            b.partial_cmp(a)
                .unwrap_or(std::cmp::Ordering::Equal)
        },
    );

    // Apply top_n filter if specified
    if let Some(n) = props.top_n {
        filtered_data.truncate(n);
    }

    let total: f64 = filtered_data
        .iter()
        .map(|(_, v)| v)
        .sum();

    // Apply min_percentage filter if specified
    if let Some(min_pct) = props.min_percentage {
        filtered_data.retain(|(_, v)| (*v / total * 100.0) >= min_pct);
    }

    if total <= 0.0 {
        return rsx! {
            div {
                class: "pie-chart-container",
                style: "width: {props.size}px; height: {props.size}px; border: 1px solid #ccc;",
                "Invalid data (total must be > 0)"
            }
        };
    }

    let center_x = props.size / 2.0;
    let center_y = props.size / 2.0;
    let radius = (props.size / 2.0) * 0.7;

    let slices = use_memo(
        move || {
            let mut current_angle = -90.0;
            let mut result = Vec::new();

            for (idx, (label, value)) in filtered_data
                .iter()
                .enumerate()
            {
                let percentage = (value / total) * 100.0;
                let angle = (value / total) * 360.0;
                let end_angle = current_angle + angle;

                let color = props
                    .colors
                    .get(
                        idx % props
                            .colors
                            .len(),
                    )
                    .cloned()
                    .unwrap_or_else(|| "#999999".to_string());

                result.push(
                    SliceData {
                        label: label.clone(),
                        value: *value,
                        percentage,
                        start_angle: current_angle,
                        end_angle,
                        color,
                        index: idx,
                    },
                );

                current_angle = end_angle;
            }
            result
        },
    );

    rsx! {
        div {
            style: "display: flex; align-items: center; gap: 30px; font-family: sans-serif;",

            svg {
                width: "{props.size}",
                height: "{props.size}",
                style: "overflow: visible;",

                defs {
                    for (idx, slice) in slices.read().iter().enumerate() {
                        {
                            let color = &slice.color;
                            rsx! {
                                linearGradient {
                                    id: "sliceGradient{idx}",
                                    x1: "0%",
                                    y1: "0%",
                                    x2: "100%",
                                    y2: "100%",
                                    stop {
                                        offset: "0%",
                                        stop_color: "{color}",
                                        stop_opacity: "1",
                                    }
                                    stop {
                                        offset: "50%",
                                        stop_color: "{color}",
                                        stop_opacity: "0.95",
                                    }
                                    stop {
                                        offset: "100%",
                                        stop_color: "{color}",
                                        stop_opacity: "0.85",
                                    }
                                }
                                radialGradient {
                                    id: "sliceShine{idx}",
                                    cx: "30%",
                                    cy: "30%",
                                    r: "70%",
                                    stop {
                                        offset: "0%",
                                        stop_color: "white",
                                        stop_opacity: "0.4",
                                    }
                                    stop {
                                        offset: "100%",
                                        stop_color: "white",
                                        stop_opacity: "0",
                                    }
                                }
                            }
                        }
                    }
                }

                for slice in slices.read().iter() {
                    {
                        let path_data = create_pie_slice(
                            center_x,
                            center_y,
                            radius,
                            slice.start_angle,
                            slice.end_angle,
                        );

                        let mid_angle = (slice.start_angle + slice.end_angle) / 2.0;
                        let label_radius = radius * 0.65;
                        let label_x = center_x + label_radius * mid_angle.to_radians().cos();
                        let label_y = center_y + label_radius * mid_angle.to_radians().sin();

                        let is_hovered = hovered_index() == Some(slice.index);
                        let transform = if is_hovered {
                            let offset = 10.0;
                            let offset_x = offset * mid_angle.to_radians().cos();
                            let offset_y = offset * mid_angle.to_radians().sin();
                            format!("translate({}, {})", offset_x, offset_y)
                        } else {
                            String::new()
                        };

                        let idx = slice.index;
                        let percentage = slice.percentage;

                        rsx! {
                            g {
                                transform: "{transform}",
                                onmouseenter: move |_| hovered_index.set(Some(idx)),
                                onmouseleave: move |_| hovered_index.set(None),
                                style: "transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);",

                                path {
                                    d: "{path_data}",
                                    fill: "url(#sliceGradient{idx})",
                                    stroke: "white",
                                    stroke_width: "3",
                                    cursor: "pointer",
                                    style: "filter: drop-shadow(0 4px 6px rgba(0, 0, 0, 0.2)); transition: all 0.3s ease;",
                                    opacity: if is_hovered { "1" } else { "0.95" },
                                }
                                path {
                                    d: "{path_data}",
                                    fill: "url(#sliceShine{idx})",
                                    stroke: "none",
                                    pointer_events: "none",
                                }

                                text {
                                    x: "{label_x}",
                                    y: "{label_y}",
                                    text_anchor: "middle",
                                    font_size: "14",
                                    font_weight: "bold",
                                    fill: "white",
                                    pointer_events: "none",
                                    "{percentage:.1}%"
                                }
                            }
                        }
                    }
                }

                if let Some(idx) = hovered_index() {
                    if let Some(slice) = slices.read().get(idx) {
                        {
                            let tooltip_x = center_x;
                            let tooltip_y = 30.0;

                            let label = &slice.label;
                            let value = slice.value;
                            let percentage = slice.percentage;

                            rsx! {
                                g {
                                    rect {
                                        x: "{tooltip_x - 100.0}",
                                        y: "{tooltip_y - 25.0}",
                                        width: "200",
                                        height: "40",
                                        fill: "rgba(0, 0, 0, 0.9)",
                                        rx: "5",
                                        stroke: "white",
                                        stroke_width: "1",
                                    }
                                    text {
                                        x: "{tooltip_x}",
                                        y: "{tooltip_y - 8.0}",
                                        text_anchor: "middle",
                                        font_size: "13",
                                        fill: "white",
                                        font_weight: "bold",
                                        "{label}"
                                    }
                                    text {
                                        x: "{tooltip_x}",
                                        y: "{tooltip_y + 8.0}",
                                        text_anchor: "middle",
                                        font_size: "11",
                                        fill: "white",
                                        "Value: {value:.2} ({percentage:.1}%)"
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if props.show_legend {
                div {
                    style: "display: flex; flex-direction: column; gap: 12px;",

                    for slice in slices.read().iter() {
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

fn create_pie_slice(cx: f64, cy: f64, radius: f64, start_angle: f64, end_angle: f64) -> String {
    let start_rad = start_angle.to_radians();
    let end_rad = end_angle.to_radians();

    let x1 = cx + radius * start_rad.cos();
    let y1 = cy + radius * start_rad.sin();
    let x2 = cx + radius * end_rad.cos();
    let y2 = cy + radius * end_rad.sin();

    let large_arc = if (end_angle - start_angle) > 180.0 {
        1
    } else {
        0
    };

    format!(
        "M {},{} L {},{} A {},{} 0 {},{} {},{} Z",
        cx, cy, x1, y1, radius, radius, large_arc, 1, x2, y2
    )
}
