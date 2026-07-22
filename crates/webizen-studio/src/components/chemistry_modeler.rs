//! Chemistry Modeler — SMILES → host descriptors.
//!
//! Host: `calculate_chemistry_properties` → organic_chemistry::parse_smiles + compute_descriptors.
//! Honesty: **Partial** — MW + Crippen LogP are real engine numbers; 2D structure view is still a placeholder.

use dioxus::prelude::*;
use serde::Deserialize;

use crate::components::honesty_chip::{HonestyChip, HonestyLevel};
use crate::components::qapp_engine::invoke_json;

#[derive(Deserialize, Default, Clone, PartialEq)]
struct ChemistryProps {
    molecular_weight: f64,
    log_p: f64,
}

#[derive(Clone, PartialEq)]
enum ChemPhase {
    Idle,
    Loading,
    Ready(ChemistryProps),
    Error(String),
}

#[component]
pub fn ChemistryModeler() -> Element {
    let mut smiles = use_signal(|| "CCO".to_string());
    let phase = use_signal(|| ChemPhase::Idle);

    // Recompute when SMILES changes (debounced only by resource identity).
    let smiles_for_resource = smiles;
    let mut phase_for_resource = phase;
    use_effect(move || {
        let current = smiles_for_resource.read().clone();
        phase_for_resource.set(ChemPhase::Loading);
        spawn(async move {
            let args = serde_json::json!({ "smiles": current });
            match invoke_json("calculate_chemistry_properties", args).await {
                Ok(res) => match serde_json::from_value::<ChemistryProps>(res) {
                    Ok(parsed) => phase_for_resource.set(ChemPhase::Ready(parsed)),
                    Err(e) => phase_for_resource
                        .set(ChemPhase::Error(format!("Bad host payload: {e}"))),
                },
                Err(e) => phase_for_resource.set(ChemPhase::Error(e)),
            }
        });
    });

    let snapshot = phase.read().clone();

    rsx! {
        div {
            style: "padding: 20px; background: #1e1e2e; color: #cdd6f4; border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; box-shadow: 0 4px 6px rgba(0,0,0,0.3); height: 100%; overflow-y: auto;",

            div {
                style: "display: flex; justify-content: space-between; align-items: flex-start; gap: 1rem; flex-wrap: wrap; border-bottom: 1px solid #313244; padding-bottom: 8px;",
                div {
                    h2 { style: "margin: 0; color: #f9e2af;", "Chemistry Modeler" }
                    p { style: "margin: 0.35rem 0 0 0; color: #a6adc8; font-size: 0.85rem;",
                        "Native SMILES parse + descriptors (qualia-core-db organic chemistry)."
                    }
                }
                HonestyChip {
                    level: HonestyLevel::Partial,
                    detail: "MW + Crippen LogP host · structure view scaffold".to_string(),
                }
            }

            div {
                label { style: "color: #a6adc8; font-size: 0.85rem;", "SMILES String" }
                input {
                    value: "{smiles}",
                    oninput: move |e| smiles.set(e.value().clone()),
                    style: "width: 100%; padding: 8px; background: #181825; border: 1px solid #45475a; color: #cdd6f4; border-radius: 4px; margin-top: 4px; box-sizing: border-box; font-family: monospace;"
                }
                div { style: "margin-top: 0.5rem; display: flex; flex-wrap: wrap; gap: 0.4rem;",
                    button {
                        style: "padding: 0.25rem 0.6rem; font-size: 0.75rem; background: #313244; border: 1px solid #45475a; color: #cdd6f4; border-radius: 999px; cursor: pointer;",
                        onclick: move |_| smiles.set("CCO".to_string()),
                        "ethanol (CCO)"
                    }
                    button {
                        style: "padding: 0.25rem 0.6rem; font-size: 0.75rem; background: #313244; border: 1px solid #45475a; color: #cdd6f4; border-radius: 999px; cursor: pointer;",
                        onclick: move |_| smiles.set("c1ccccc1".to_string()),
                        "benzene"
                    }
                    button {
                        style: "padding: 0.25rem 0.6rem; font-size: 0.75rem; background: #313244; border: 1px solid #45475a; color: #cdd6f4; border-radius: 999px; cursor: pointer;",
                        onclick: move |_| smiles.set("CC(=O)Oc1ccccc1C(=O)O".to_string()),
                        "aspirin"
                    }
                }
            }

            match &snapshot {
                ChemPhase::Loading | ChemPhase::Idle => {
                    rsx! {
                        div {
                            style: "padding: 1rem; border-radius: 8px; background: #181825; border: 1px solid #313244; color: #89dceb; text-align: center;",
                            "Computing descriptors on host…"
                        }
                    }
                }
                ChemPhase::Error(msg) => {
                    rsx! {
                        div {
                            style: "padding: 1rem; border-radius: 8px; background: rgba(127,29,29,0.35); border: 1px solid rgba(248,113,113,0.4); color: #fecaca;",
                            div { style: "font-weight: 700; margin-bottom: 0.35rem;", "Host chemistry failed" }
                            pre {
                                style: "margin: 0; white-space: pre-wrap; word-break: break-word; font-size: 0.85rem;",
                                "{msg}"
                            }
                            p { style: "margin: 0.5rem 0 0 0; font-size: 0.8rem; color: #fca5a5;",
                                "Zeros are not shown on failure. Fix the SMILES or check desktop host binding."
                            }
                        }
                    }
                }
                ChemPhase::Ready(props) => {
                    rsx! {
                        div {
                            style: "display: grid; grid-template-columns: repeat(2, 1fr); gap: 16px;",
                            div {
                                style: "background: #11111b; padding: 16px; border-radius: 8px; border-left: 4px solid #89dceb;",
                                h4 { style: "margin-top: 0; color: #89dceb;", "Molecular Weight" }
                                div { style: "font-size: 1.4rem; font-weight: 700;", "{props.molecular_weight:.4} g/mol" }
                                div { style: "font-size: 0.7rem; color: #6c7086; margin-top: 0.35rem;", "host · compute_descriptors" }
                            }
                            div {
                                style: "background: #11111b; padding: 16px; border-radius: 8px; border-left: 4px solid #f5c2e7;",
                                h4 { style: "margin-top: 0; color: #f5c2e7;", "LogP (Crippen)" }
                                div { style: "font-size: 1.4rem; font-weight: 700;", "{props.log_p:.4}" }
                                div { style: "font-size: 0.7rem; color: #6c7086; margin-top: 0.35rem;", "host · logp_crippen" }
                            }
                        }
                    }
                }
            }

            div {
                style: "flex: 1; min-height: 120px; border: 1px dashed #45475a; border-radius: 8px; display: flex; align-items: center; justify-content: center; background: #181825; color: #6c7086; text-align: center; padding: 1rem;",
                "2D structure visualizer — Scaffold (not drawn from host). SMILES: {smiles}"
            }
        }
    }
}
