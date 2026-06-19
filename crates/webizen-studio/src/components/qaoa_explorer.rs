use dioxus::prelude::*;

#[component]
pub fn QaoaExplorer() -> Element {
    rsx! {
        div {
            style: "padding: 20px; background: #1e1e2e; color: #cdd6f4; border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%;",
            h2 { style: "margin: 0; color: #f38ba8; border-bottom: 1px solid #313244; padding-bottom: 8px;", "QAOA Explorer" }
            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 16px;",
                div {
                    label { "Problem Formulation" }
                    select {
                        style: "width: 100%; padding: 8px; background: #181825; border: 1px solid #45475a; color: #cdd6f4; border-radius: 4px; margin-top: 4px;",
                        option { "MaxCut" }
                        option { "Graph Coloring" }
                        option { "TSP" }
                    }
                }
                div {
                    label { "Depth (p)" }
                    input {
                        type: "number",
                        value: "3",
                        style: "width: 100%; padding: 8px; background: #181825; border: 1px solid #45475a; color: #cdd6f4; border-radius: 4px; margin-top: 4px;"
                    }
                }
            }
            div {
                style: "flex: 1; border: 1px solid #313244; border-radius: 8px; background: #181825; padding: 16px;",
                h4 { style: "margin: 0 0 8px 0; color: #bac2de;", "Energy Landscape (γ, β)" }
                div {
                    style: "width: 100%; height: calc(100% - 30px); background: radial-gradient(circle at 30% 30%, #f38ba8 0%, #181825 60%); border-radius: 4px;"
                }
            }
            div {
                style: "display: flex; gap: 16px; align-items: center;",
                button {
                    style: "background: #f38ba8; color: #11111b; border: none; padding: 10px 20px; border-radius: 4px; cursor: pointer; font-weight: bold;",
                    "Optimize Parameters"
                }
                div {
                    style: "color: #a6adc8; font-size: 14px;",
                    "Approximation Ratio: 0.87 (Mocked)"
                }
            }
        }
    }
}
