use dioxus::prelude::*;

#[component]
pub fn OdeSolver() -> Element {
    let mut equation = use_signal(|| "dy/dt = -k * y".to_string());

    rsx! {
        div {
            style: "padding: 20px; background: #1e1e2e; color: #cdd6f4; border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%;",
            h2 { style: "margin: 0; color: #cba6f7; border-bottom: 1px solid #313244; padding-bottom: 8px;", "RK4 ODE Solver" }
            div {
                label { "Differential Equation" }
                input {
                    value: "{equation}",
                    oninput: move |e| equation.set(e.value().clone()),
                    style: "width: 100%; padding: 8px; background: #181825; border: 1px solid #45475a; color: #cdd6f4; border-radius: 4px; margin-top: 4px; font-family: monospace;"
                }
            }
            div {
                style: "display: flex; gap: 16px;",
                button {
                    style: "background: #cba6f7; color: #11111b; border: none; padding: 8px 16px; border-radius: 4px; cursor: pointer; font-weight: bold;",
                    "Solve"
                }
            }
            div {
                style: "flex: 1; border: 1px dashed #45475a; border-radius: 8px; display: flex; align-items: center; justify-content: center; color: #6c7086; background: #181825;",
                "Solution Plot Area for {equation}"
            }
        }
    }
}
