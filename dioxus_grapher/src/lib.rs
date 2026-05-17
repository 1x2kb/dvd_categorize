use dioxus::prelude::*;

pub mod components;

pub use components::{AutoOptions, BarGraph, ExplicitOptions, Graph, PieChart, XAxisMode};

#[derive(Clone, PartialEq, Props)]
pub struct GraphData {
    pub points: Vec<(
        f64,
        f64,
    )>,
    pub width: f64,
    pub height: f64,
}
