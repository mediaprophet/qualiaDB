use dioxus::prelude::*;

#[component]
pub fn ChemistryModeler() -> Element {
    let mut smiles = use_signal(|| "CCO".to_string());

    rsx! {
        div {
            style: "padding: 20px; background: #1e1e2e; color: #cdd6f4; border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; box-shadow: 0 4px 6px rgba(0,0,0,0.3); height: 100%;",
            h2 { style: "margin: 0; color: #f9e2af; border-bottom: 1px solid #313244; padding-bottom: 8px;", "Chemistry Modeler" }
            div {
                label { "SMILES String" }
                input {
                    value: "{smiles}",
                    oninput: move |e| smiles.set(e.value().clone()),
                    style: "width: 100%; padding: 8px; background: #181825; border: 1px solid #45475a; color: #cdd6f4; border-radius: 4px; margin-top: 4px;"
                }
            }
            div {
                style: "display: grid; grid-template-columns: repeat(2, 1fr); gap: 16px;",
                div {
                    style: "background: #11111b; padding: 16px; border-radius: 8px; border-left: 4px solid #89dceb;",
                    h4 { style: "margin-top: 0; color: #89dceb;", "Molecular Weight" }
                    div { "46.07 g/mol (Mocked)" }
                }
                div {
                    style: "background: #11111b; padding: 16px; border-radius: 8px; border-left: 4px solid #f5c2e7;",
                    h4 { style: "margin-top: 0; color: #f5c2e7;", "LogP" }
                    div { "-0.00 (Mocked)" }
                }
            }
            div {
                style: "flex: 1; border: 1px solid #313244; border-radius: 8px; display: flex; align-items: center; justify-content: center; background: #181825;",
                "2D Molecule Structure Visualizer Placeholder for {smiles}"
            }
        }
    }
}
