use dioxus::prelude::*;

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
}

#[component]
pub fn Graph(props: GraphProps) -> Element {
    if props.points.is_empty() {
        return rsx! {
            div {
                class: "graph-container",
                style: "width: {props.width}px; height: {props.height}px; border: 1px solid #ccc;",
                "No data points"
            }
        };
    }

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

    let path_data = props.points
        .iter()
        .enumerate()
        .map(|(i, (x, y))| {
            let svg_x = padding + (x - min_x) * scale_x;
            let svg_y = props.height - padding - (y - min_y) * scale_y;
            if i == 0 {
                format!("M {} {}", svg_x, svg_y)
            } else {
                format!("L {} {}", svg_x, svg_y)
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

    rsx! {
        svg {
            width: "{props.width}",
            height: "{props.height}",
            
            defs {
                linearGradient {
                    id: "lineGradient",
                    x1: "0%",
                    y1: "0%",
                    x2: "0%",
                    y2: "100%",
                    stop {
                        offset: "0%",
                        stop_color: "{props.stroke_color}",
                        stop_opacity: "1",
                    }
                    stop {
                        offset: "50%",
                        stop_color: "{props.stroke_color}",
                        stop_opacity: "0.9",
                    }
                    stop {
                        offset: "100%",
                        stop_color: "{props.stroke_color}",
                        stop_opacity: "0.7",
                    }
                }
                linearGradient {
                    id: "areaGradient",
                    x1: "0%",
                    y1: "0%",
                    x2: "0%",
                    y2: "100%",
                    stop {
                        offset: "0%",
                        stop_color: "{props.stroke_color}",
                        stop_opacity: "0.3",
                    }
                    stop {
                        offset: "100%",
                        stop_color: "{props.stroke_color}",
                        stop_opacity: "0.05",
                    }
                }
            }
            
            for i in 0..5 {
                {
                    let grid_y = padding + (i as f64 * graph_height / 4.0);
                    rsx! {
                        line {
                            x1: "{padding}",
                            y1: "{grid_y}",
                            x2: "{props.width - padding}",
                            y2: "{grid_y}",
                            stroke: "#e5e7eb",
                            stroke_width: "1",
                            stroke_dasharray: "4,4",
                            opacity: "0.5",
                        }
                    }
                }
            }
            
            path {
                d: "{path_data} L {props.width - padding},{props.height - padding} L {padding},{props.height - padding} Z",
                fill: "url(#areaGradient)",
                stroke: "none",
            }
            
            path {
                d: "{path_data}",
                stroke: "url(#lineGradient)",
                stroke_width: "{props.stroke_width + 1.0}",
                fill: "none",
                stroke_linecap: "round",
                stroke_linejoin: "round",
                style: "transition: all 0.3s ease;",
            }
            
            for (x, y) in props.points.iter() {
                {
                    let svg_x = padding + (x - min_x) * scale_x;
                    let svg_y = props.height - padding - (y - min_y) * scale_y;
                    rsx! {
                        g {
                            circle {
                                cx: "{svg_x}",
                                cy: "{svg_y}",
                                r: "6",
                                fill: "{props.stroke_color}",
                                opacity: "0.3",
                            }
                            circle {
                                cx: "{svg_x}",
                                cy: "{svg_y}",
                                r: "4",
                                fill: "{props.stroke_color}",
                                style: "cursor: pointer; transition: all 0.2s ease;",
                            }
                            circle {
                                cx: "{svg_x}",
                                cy: "{svg_y}",
                                r: "2",
                                fill: "white",
                            }
                        }
                    }
                }
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
