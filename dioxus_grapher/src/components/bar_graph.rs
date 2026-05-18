use dioxus::prelude::*;

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
    pub data: Vec<(
        String,
        f64,
    )>,
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
}

#[component]
pub fn BarGraph(props: BarGraphProps) -> Element {
    let mut hovered_index = use_signal(|| None::<usize>);

    if props
        .data
        .is_empty()
    {
        return rsx! {
            div {
                class: "bar-graph-container",
                style: "width: {props.width}px; height: {props.height}px; border: 1px solid #ccc;",
                "No data"
            }
        };
    }

    let padding = 60.0;
    let graph_width = props.width - 2.0 * padding;
    let graph_height = props.height - 2.0 * padding;

    // Use custom y_max or auto-calculate
    let max_value = props
        .y_max
        .unwrap_or_else(
            || {
                props
                    .data
                    .iter()
                    .map(|(_, v)| *v)
                    .fold(
                        f64::NEG_INFINITY,
                        f64::max,
                    )
            },
        );

    // Use custom y_min or auto-calculate
    let min_value = props
        .y_min
        .unwrap_or_else(
            || {
                props
                    .data
                    .iter()
                    .map(|(_, v)| *v)
                    .fold(
                        f64::INFINITY,
                        f64::min,
                    )
                    .min(0.0)
            },
        );

    let value_range = max_value - min_value;
    let scale_y = if value_range > 0.0 {
        graph_height / value_range
    } else {
        1.0
    };

    let bar_width = graph_width
        / props
            .data
            .len() as f64
        * 0.8;
    let bar_spacing = graph_width
        / props
            .data
            .len() as f64;

    let y_ticks = 5;
    let tick_step = value_range / y_ticks as f64;

    rsx! {
        svg {
            width: "{props.width}",
            height: "{props.height}",
            style: "font-family: sans-serif;",

            defs {
                linearGradient {
                    id: "barGradient",
                    x1: "0%",
                    y1: "0%",
                    x2: "0%",
                    y2: "100%",
                    stop {
                        offset: "0%",
                        stop_color: "{props.bar_color}",
                        stop_opacity: "1",
                    }
                    stop {
                        offset: "100%",
                        stop_color: "{props.bar_color}",
                        stop_opacity: "0.8",
                    }
                }
            }

            // Y-axis line
            line {
                x1: "{padding}",
                y1: "{padding}",
                x2: "{padding}",
                y2: "{props.height - padding}",
                stroke: "#374151",
                stroke_width: "2",
            }

            // X-axis line
            line {
                x1: "{padding}",
                y1: "{props.height - padding}",
                x2: "{props.width - padding}",
                y2: "{props.height - padding}",
                stroke: "#374151",
                stroke_width: "2",
            }

            // Y-axis ticks and labels
            for i in 0..=y_ticks {
                {
                    let value = min_value + (i as f64 * tick_step);
                    let y_pos = props.height - padding - ((value - min_value) * scale_y);
                    rsx! {
                        line {
                            x1: "{padding - 5.0}",
                            y1: "{y_pos}",
                            x2: "{padding}",
                            y2: "{y_pos}",
                            stroke: "#374151",
                            stroke_width: "1",
                        }
                        text {
                            x: "{padding - 10.0}",
                            y: "{y_pos + 4.0}",
                            text_anchor: "end",
                            font_size: "12",
                            fill: "#9ca3af",
                            "{value:.0}"
                        }
                    }
                }
            }

            // Bars and X-axis labels
            for (idx, (label, value)) in props.data.iter().enumerate() {
                {
                    let x_pos = padding + (idx as f64 * bar_spacing) + (bar_spacing - bar_width) / 2.0;
                    let bar_height = (value - min_value) * scale_y;
                    let y_pos = props.height - padding - bar_height;

                    // Get label based on x_mode
                    let display_label = match &props.x_mode {
                        XAxisMode::Explicit(opts) => {
                            opts.labels.get(idx).cloned().unwrap_or_else(|| label.clone())
                        }
                        XAxisMode::Auto(_) => label.clone(),
                    };

                    let is_hovered = hovered_index() == Some(idx);
                    let bar_fill = if is_hovered { "#06b6d4" } else { "url(#barGradient)" };

                    rsx! {
                        g {
                            onmouseenter: move |_| hovered_index.set(Some(idx)),
                            onmouseleave: move |_| hovered_index.set(None),

                            // Vertical line to x-axis when hovered
                            if is_hovered {
                                line {
                                    x1: "{x_pos + bar_width / 2.0}",
                                    y1: "{y_pos}",
                                    x2: "{x_pos + bar_width / 2.0}",
                                    y2: "{props.height - padding}",
                                    stroke: "#06b6d4",
                                    stroke_width: "2",
                                    stroke_dasharray: "4,4",
                                    opacity: "0.6",
                                }
                            }

                            rect {
                                x: "{x_pos}",
                                y: "{y_pos}",
                                width: "{bar_width}",
                                height: "{bar_height}",
                                fill: "{bar_fill}",
                                rx: "6",
                                ry: "6",
                                style: "cursor: pointer; filter: drop-shadow(0 4px 6px rgba(0, 0, 0, 0.1));",
                            }
                            rect {
                                x: "{x_pos}",
                                y: "{y_pos}",
                                width: "{bar_width}",
                                height: "{bar_height.max(8.0)}",
                                fill: "none",
                                stroke: if is_hovered { "#06b6d4" } else { "{props.bar_color}" },
                                stroke_width: if is_hovered { "3" } else { "2" },
                                rx: "6",
                                ry: "6",
                                opacity: if is_hovered { "0.8" } else { "0.3" },
                            }
                        }

                        {
                            let skip = match &props.x_mode {
                                XAxisMode::Auto(opts) => opts.skip_labels,
                                XAxisMode::Explicit(opts) => opts.skip_labels,
                            };

                            if idx % skip == 0 {
                                rsx! {
                                    text {
                                        x: "{x_pos + bar_width / 2.0}",
                                        y: "{props.height - padding + 20.0}",
                                        text_anchor: "middle",
                                        font_size: "12",
                                        fill: "#9ca3af",
                                        "{display_label}"
                                    }
                                }
                            } else {
                                rsx! {}
                            }
                        }

                        text {
                            x: "{x_pos + bar_width / 2.0}",
                            y: "{y_pos - 5.0}",
                            text_anchor: "middle",
                            font_size: "11",
                            fill: "#9ca3af",
                            font_weight: "bold",
                            pointer_events: "none",
                            "{value:.0}"
                        }
                    }
                }
            }

            // X-axis label
            text {
                x: "{props.width / 2.0}",
                y: "{props.height - 10.0}",
                text_anchor: "middle",
                font_size: "14",
                fill: "#374151",
                font_weight: "bold",
                "{props.x_label}"
            }

            // Y-axis label (rotated)
            text {
                x: "{15.0}",
                y: "{props.height / 2.0}",
                text_anchor: "middle",
                font_size: "14",
                fill: "#374151",
                font_weight: "bold",
                transform: "rotate(-90, 15, {props.height / 2.0})",
                "{props.y_label}"
            }

            // Tooltip on hover
            if let Some(idx) = hovered_index() {
                if let Some((label, value)) = props.data.get(idx) {
                    {
                        let display_label = match &props.x_mode {
                            XAxisMode::Explicit(opts) => {
                                opts.labels.get(idx).cloned().unwrap_or_else(|| label.clone())
                            }
                            XAxisMode::Auto(_) => label.clone(),
                        };

                        let tooltip_x = props.width / 2.0;
                        let tooltip_y = 30.0;

                        rsx! {
                            g {
                                rect {
                                    x: "{tooltip_x - 100.0}",
                                    y: "{tooltip_y - 25.0}",
                                    width: "200",
                                    height: "40",
                                    fill: "#1f2937",
                                    rx: "8",
                                    stroke: "{props.bar_color}",
                                    stroke_width: "2",
                                    style: "filter: drop-shadow(0 4px 6px rgba(0, 0, 0, 0.3));",
                                }
                                text {
                                    x: "{tooltip_x}",
                                    y: "{tooltip_y - 5.0}",
                                    text_anchor: "middle",
                                    font_size: "14",
                                    font_weight: "bold",
                                    fill: "#ffffff",
                                    "{display_label}"
                                }
                                text {
                                    x: "{tooltip_x}",
                                    y: "{tooltip_y + 10.0}",
                                    text_anchor: "middle",
                                    font_size: "12",
                                    fill: "#d1d5db",
                                    "Value: {value:.0}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
