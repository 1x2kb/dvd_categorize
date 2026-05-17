use dioxus::prelude::*;
use dioxus_grapher::PieChart;

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let market_share = vec![
        (
            "Chrome".to_string(),
            65.0,
        ),
        (
            "Safari".to_string(),
            18.0,
        ),
        (
            "Firefox".to_string(),
            8.0,
        ),
        (
            "Edge".to_string(),
            5.0,
        ),
        (
            "Other".to_string(),
            4.0,
        ),
    ];

    let expenses = vec![
        (
            "Rent".to_string(),
            1200.0,
        ),
        (
            "Food".to_string(),
            450.0,
        ),
        (
            "Transport".to_string(),
            200.0,
        ),
        (
            "Entertainment".to_string(),
            150.0,
        ),
        (
            "Utilities".to_string(),
            180.0,
        ),
        (
            "Savings".to_string(),
            500.0,
        ),
    ];

    rsx! {
        div {
            style: "padding: 20px; font-family: sans-serif;",
            h1 { "Pie Chart Examples" }
            p { "Hover over slices or legend items to see details" }

            div {
                style: "margin: 30px 0;",
                h2 { "Browser Market Share" }
                PieChart {
                    data: market_share,
                    size: 400.0,
                    show_legend: true,
                }
            }

            div {
                style: "margin: 30px 0;",
                h2 { "Monthly Expenses" }
                PieChart {
                    data: expenses,
                    size: 450.0,
                    show_legend: true,
                }
            }
        }
    }
}
