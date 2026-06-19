use dioxus::prelude::*;

#[component]
pub fn QpuOptimizer() -> Element {
    rsx! {
        div {
            style: "padding: 20px; background: #1e1e2e; color: #cdd6f4; border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%;",
            h2 { style: "margin: 0; color: #cba6f7; border-bottom: 1px solid #313244; padding-bottom: 8px;", "QPU Pulse Optimizer" }
            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 16px;",
                div {
                    label { "Target Gate" }
                    select {
                        style: "width: 100%; padding: 8px; background: #181825; border: 1px solid #45475a; color: #cdd6f4; border-radius: 4px; margin-top: 4px;",
                        option { "CNOT" }
                        option { "Hadamard" }
                        option { "Toffoli" }
                    }
                }
                div {
                    label { "Fidelity Target" }
                    input {
                        type: "text",
                        value: "99.99%",
                        style: "width: 100%; padding: 8px; background: #181825; border: 1px solid #45475a; color: #cdd6f4; border-radius: 4px; margin-top: 4px;"
                    }
                }
            }
            div {
                style: "flex: 1; border: 1px solid #313244; border-radius: 8px; background: #181825; position: relative; overflow: hidden;",
                div {
                    style: "position: absolute; bottom: 0; left: 0; width: 100%; height: 50%; background: linear-gradient(0deg, rgba(203,166,247,0.2) 0%, transparent 100%); border-top: 2px solid #cba6f7;"
                }
                div {
                    style: "position: absolute; top: 10px; left: 10px; font-size: 12px; color: #6c7086;",
                    "Optimized Microwave Pulse Envelope"
                }
            }
            button {
                style: "background: #cba6f7; color: #11111b; border: none; padding: 10px; border-radius: 4px; cursor: pointer; font-weight: bold;",
                "Start GRAPE Optimization"
            }
        }
    }
}
