use dioxus::prelude::*;
use serde::Deserialize;

use crate::components::qapp_engine::invoke_json;

#[derive(Deserialize, Default, Clone)]
struct QuantumDftProps {
    energy: f64,
}

#[component]
pub fn QuantumDft() -> Element {
    let mut geometry =
        use_signal(|| "O 0.000 0.000 0.000\nH 0.757 0.586 0.000\nH -0.757 0.586 0.000".to_string());

    let energy_resource = use_resource(move || {
        let current_geometry = geometry.read().clone();
        async move {
            let args = serde_json::json!({ "molecule": current_geometry });
            if let Ok(res) = invoke_json("calculate_quantum_dft", args).await {
                if let Ok(parsed) = serde_json::from_value::<QuantumDftProps>(res) {
                    return parsed;
                }
            }
            QuantumDftProps::default()
        }
    });

    let energy_props = energy_resource.read().clone().unwrap_or_default();

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
                    value: "{geometry}",
                    oninput: move |e| geometry.set(e.value().clone()),
                    style: "width: 100%; height: 100px; padding: 8px; background: #181825; border: 1px solid #45475a; color: #cdd6f4; border-radius: 4px; margin-top: 4px; font-family: monospace;"
                }
            }
            button {
                style: "background: #89b4fa; color: #11111b; border: none; padding: 10px; border-radius: 4px; cursor: pointer; font-weight: bold; margin-top: auto;",
                "Run Ground State Calculation"
            }
            div {
                style: "background: #11111b; padding: 16px; border-radius: 8px; border-left: 4px solid #89b4fa;",
                "Energy: {energy_props.energy:.4} Hartree"
            }
        }
    }
}
