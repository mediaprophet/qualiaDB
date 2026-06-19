use dioxus::prelude::*;

#[component]
pub fn RuralStudiesQapp() -> Element {
    let mut rural_type = use_signal(|| "Agricultural".to_string());
    let mut theoretical_approach = use_signal(|| "Political Economy".to_string());
    let mut issue = use_signal(|| "Depopulation".to_string());
    let mut population_density = use_signal(|| "Low 1-10".to_string());
    let mut economic_diversification = use_signal(|| 50u32);
    let mut notes = use_signal(|| String::new());

    let rural_types = [
        "Agricultural",
        "Extractive",
        "Post-Productivist",
        "Amenity",
        "Digital Rural",
        "Shrinking Rural",
    ];
    let approaches = [
        "Political Economy",
        "New Rural Geography",
        "Postproductivist",
        "Feminist Rural",
        "Decolonial",
    ];
    let issues = [
        "Depopulation",
        "Land Access",
        "Digital Divide",
        "Agriculture Policy",
        "Rural Poverty",
        "In-Migration",
    ];
    let densities = ["Very Low <1/km²", "Low 1-10", "Moderate 10-50"];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Rural Studies" }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Rural Type" }
                select {
                    value: "{rural_type}",
                    onchange: move |e| rural_type.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in rural_types { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Theoretical Approach" }
                select {
                    value: "{theoretical_approach}",
                    onchange: move |e| theoretical_approach.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in approaches { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Issue" }
                select {
                    value: "{issue}",
                    onchange: move |e| issue.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in issues { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Population Density" }
                select {
                    value: "{population_density}",
                    onchange: move |e| population_density.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in densities { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Economic Diversification: {economic_diversification}" }
                input {
                    r#type: "range", min: "0", max: "100", value: "{economic_diversification}",
                    oninput: move |e| economic_diversification.set(e.value().parse().unwrap_or(50)),
                    style: "width: 100%; margin-top: 4px;"
                }
            }
            div {
                style: "flex: 1; display: flex; flex-direction: column;",
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Notes" }
                textarea {
                    value: "{notes}",
                    oninput: move |e| notes.set(e.value()),
                    style: "flex: 1; width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: none; box-sizing: border-box; min-height: 60px;"
                }
            }
            div {
                style: "background: var(--qualia-bg); padding: 12px 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent);",
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{rural_type} | {theoretical_approach} | {issue} | {population_density} | Diversification: {economic_diversification}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → rural studies engine | place sieve | land anchor" }
            }
        }
    }
}
