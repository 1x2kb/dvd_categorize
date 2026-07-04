use dioxus::prelude::*;
use dioxus_grapher::Graph;

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let points = vec![
        (
            0.0, 0.0,
        ),
        (
            1.0, 2.0,
        ),
        (
            2.0, 1.5,
        ),
        (
            3.0, 4.0,
        ),
        (
            4.0, 3.5,
        ),
        (
            5.0, 5.0,
        ),
    ];

    rsx! {
        div {
            style: "padding: 20px; font-family: sans-serif;",
            h1 { "Dioxus Grapher Example" }

            div {
                style: "margin: 20px 0; height: 400px;",
                h2 { "Simple Line Graph" }
                Graph {
                    points: points.clone(),
                    width: 600.0,
                    height: 400.0,
                    stroke_color: "#1e40af".to_string(),
                    stroke_width: 3.0,
                }
            }

            div {
                style: "margin: 20px 0; height: 300px;",
                h2 { "Custom Styled Graph" }
                Graph {
                    points: vec![
                        (0.0, 10.0),
                        (1.0, 15.0),
                        (2.0, 13.0),
                        (3.0, 17.0),
                        (4.0, 20.0),
                    ],
                    width: 800.0,
                    height: 300.0,
                    stroke_color: "#0f766e".to_string(),
                    stroke_width: 3.0,
                }
            }
        }
    }
}
