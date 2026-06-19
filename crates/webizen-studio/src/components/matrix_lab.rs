use dioxus::prelude::*;

#[component]
pub fn MatrixLab() -> Element {
    rsx! {
        div {
            style: "padding: 20px; background: #1e1e2e; color: #cdd6f4; border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%;",
            h2 { style: "margin: 0; color: #94e2d5; border-bottom: 1px solid #313244; padding-bottom: 8px;", "Matrix Lab" }
            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 16px;",
                div {
                    h4 { style: "color: #94e2d5; margin: 0 0 8px 0;", "Matrix A" }
                    textarea {
                        style: "width: 100%; height: 100px; padding: 8px; background: #181825; border: 1px solid #45475a; color: #cdd6f4; border-radius: 4px; font-family: monospace;",
                        "1 2\n3 4"
                    }
                }
                div {
                    h4 { style: "color: #94e2d5; margin: 0 0 8px 0;", "Matrix B" }
                    textarea {
                        style: "width: 100%; height: 100px; padding: 8px; background: #181825; border: 1px solid #45475a; color: #cdd6f4; border-radius: 4px; font-family: monospace;",
                        "5 6\n7 8"
                    }
                }
            }
            div {
                style: "display: flex; gap: 8px;",
                button { style: "background: #45475a; color: #cdd6f4; border: none; padding: 6px 12px; border-radius: 4px; cursor: pointer;", "A × B" }
                button { style: "background: #45475a; color: #cdd6f4; border: none; padding: 6px 12px; border-radius: 4px; cursor: pointer;", "A + B" }
                button { style: "background: #45475a; color: #cdd6f4; border: none; padding: 6px 12px; border-radius: 4px; cursor: pointer;", "det(A)" }
                button { style: "background: #45475a; color: #cdd6f4; border: none; padding: 6px 12px; border-radius: 4px; cursor: pointer;", "inv(A)" }
            }
            div {
                style: "flex: 1; background: #11111b; padding: 16px; border-radius: 8px; border: 1px solid #313244;",
                h4 { style: "margin: 0 0 8px 0; color: #a6adc8;", "Result" }
                pre { style: "margin: 0;", "19 22\n43 50" }
            }
        }
    }
}
