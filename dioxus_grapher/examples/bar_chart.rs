use dioxus::prelude::*;
use dioxus_grapher::BarGraph;

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let sales_data = vec![
        (
            "Jan".to_string(),
            45.0,
        ),
        (
            "Feb".to_string(),
            62.0,
        ),
        (
            "Mar".to_string(),
            58.0,
        ),
        (
            "Apr".to_string(),
            73.0,
        ),
        (
            "May".to_string(),
            81.0,
        ),
        (
            "Jun".to_string(),
            69.0,
        ),
    ];

    let temperature_data = vec![
        (
            "Mon".to_string(),
            22.5,
        ),
        (
            "Tue".to_string(),
            24.0,
        ),
        (
            "Wed".to_string(),
            19.5,
        ),
        (
            "Thu".to_string(),
            21.0,
        ),
        (
            "Fri".to_string(),
            23.5,
        ),
    ];

    rsx! {
        div {
            style: "padding: 20px; font-family: sans-serif;",
            h1 { "Bar Graph Examples" }

            div {
                style: "margin: 30px 0; height: 450px;",
                h2 { "Monthly Sales" }
                BarGraph {
                    data: sales_data,
                    width: 700.0,
                    height: 450.0,
                    bar_color: "#1e40af".to_string(),
                    x_label: "Month".to_string(),
                    y_label: "Sales (thousands)".to_string(),
                }
            }

            div {
                style: "margin: 30px 0; height: 400px;",
                h2 { "Weekly Temperature" }
                BarGraph {
                    data: temperature_data,
                    width: 600.0,
                    height: 400.0,
                    bar_color: "#0f766e".to_string(),
                    x_label: "Day of Week".to_string(),
                    y_label: "Temperature (°C)".to_string(),
                }
            }
        }
    }
}
