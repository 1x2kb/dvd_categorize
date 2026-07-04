# Dioxus Grapher

Graph component library for Dioxus WASM apps.

## Features

- Canvas line graphs with device-pixel-ratio scaling
- Canvas bar graphs with axis labels and hover tooltips
- Canvas pie charts with hover tooltips + DOM legend
- Auto-scaling axes
- Customizable colors + stroke width
- HiDPI / Retina / 4K crisp rendering

## Usage

### Line Graph

```rust
use dioxus::prelude::*;
use dioxus_grapher::Graph;

#[component]
fn App() -> Element {
    rsx! {
        Graph {
            points: vec![(0.0, 0.0), (1.0, 2.0), (2.0, 1.5)],
            width: 600.0,
            height: 400.0,
            stroke_color: "#3b82f6".to_string(),
            stroke_width: 2.0,
        }
    }
}
```

### Bar Graph

```rust
use dioxus::prelude::*;
use dioxus_grapher::BarGraph;

#[component]
fn App() -> Element {
    rsx! {
        BarGraph {
            data: vec![
                ("Jan".to_string(), 45.0),
                ("Feb".to_string(), 62.0),
                ("Mar".to_string(), 58.0),
            ],
            width: 700.0,
            height: 450.0,
            bar_color: "#3b82f6".to_string(),
            x_label: "Month".to_string(),
            y_label: "Sales".to_string(),
        }
    }
}
```

### Pie Chart

```rust
use dioxus::prelude::*;
use dioxus_grapher::PieChart;

#[component]
fn App() -> Element {
    rsx! {
        PieChart {
            data: vec![
                ("Chrome".to_string(), 65.0),
                ("Safari".to_string(), 18.0),
                ("Firefox".to_string(), 8.0),
            ],
            size: 400.0,
            show_legend: true,
        }
    }
}
```

## Run Examples

```bash
dx serve --example simple --platform web
dx serve --example bar_chart --platform web
dx serve --example pie_demo --platform web
```

## Graph Props

- `points: Vec<(f64, f64)>` - Data points (x, y)
- `width: f64` - Graph width (default: 600)
- `height: f64` - Graph height (default: 400)
- `stroke_color: String` - Line color (default: "#3b82f6")
- `stroke_width: f64` - Line thickness (default: 2.0)
- `responsive: bool` - Fill parent container size (default: true)

## BarGraph Props

- `data: Vec<(String, f64)>` - Labels + values
- `width: f64` - Graph width (default: 600)
- `height: f64` - Graph height (default: 400)
- `bar_color: String` - Bar color (default: "#3b82f6")
- `x_label: String` - X-axis label (default: "X Axis")
- `y_label: String` - Y-axis label (default: "Y Axis")
- `responsive: bool` - Fill parent container size and auto-skip X labels (default: true)

## PieChart Props

- `data: Vec<(String, f64)>` - Labels + values
- `size: f64` - Chart size (default: 400)
- `colors: Vec<String>` - Slice colors (default: 8 color palette)
- `show_legend: bool` - Show legend (default: true)
- `responsive: bool` - Fill parent container size (default: true)
