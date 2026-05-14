use dioxus::prelude::*;

pub mod components;

pub use components::{Graph, BarGraph, PieChart, XAxisMode, AutoOptions, ExplicitOptions};

#[derive(Clone, PartialEq, Props)]
pub struct GraphData {
    pub points: Vec<(f64, f64)>,
    pub width: f64,
    pub height: f64,
}
