use dioxus::prelude::*;

#[component]
pub fn QuantumDft() -> Element {
    rsx! {
        div {
            style: "padding: 20px; background: #1e1e2e; color: #cdd6f4; border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%;",
            h2 { style: "margin: 0; color: #89b4fa; border-bottom: 1px solid #313244; padding-bottom: 8px;", "Quantum DFT Engine" }
            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 16px;",
                div {
                    label { "Functional" }
                    select {
                        style: "width: 100%; padding: 8px; background: #181825; border: 1px solid #45475a; color: #cdd6f4; border-radius: 4px; margin-top: 4px;",
                        option { "B3LYP" }
                        option { "PBE" }
                        option { "M06-2X" }
                    }
                }
                div {
                    label { "Basis Set" }
                    select {
                        style: "width: 100%; padding: 8px; background: #181825; border: 1px solid #45475a; color: #cdd6f4; border-radius: 4px; margin-top: 4px;",
                        option { "6-31G(d)" }
                        option { "def2-SVP" }
                        option { "cc-pVDZ" }
                    }
                }
            }
            div {
                label { "Geometry (XYZ format)" }
                textarea {
                    style: "width: 100%; height: 100px; padding: 8px; background: #181825; border: 1px solid #45475a; color: #cdd6f4; border-radius: 4px; margin-top: 4px; font-family: monospace;",
                    "O 0.000 0.000 0.000\nH 0.757 0.586 0.000\nH -0.757 0.586 0.000"
                }
            }
            button {
                style: "background: #89b4fa; color: #11111b; border: none; padding: 10px; border-radius: 4px; cursor: pointer; font-weight: bold; margin-top: auto;",
                "Run Ground State Calculation"
            }
            div {
                style: "background: #11111b; padding: 16px; border-radius: 8px; border-left: 4px solid #89b4fa;",
                "Energy: -76.4089 Hartree (Mocked)"
            }
        }
    }
}
