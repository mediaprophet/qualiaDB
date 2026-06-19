use dioxus::prelude::*;

#[component]
pub fn EnvironmentalJusticeQapp() -> Element {
    let mut justice_framework = use_signal(|| "Distributive".to_string());
    let mut pollutant_burden = use_signal(|| "Air Quality".to_string());
    let mut affected_community = use_signal(|| "Low-Income".to_string());
    let mut regulatory_gap = use_signal(|| 50u32);
    let mut health_disparity = use_signal(|| 50u32);
    let mut notes = use_signal(|| String::new());

    let justice_frameworks = [
        "Distributive",
        "Procedural",
        "Corrective",
        "Recognition",
        "Transformative",
    ];
    let pollutant_burdens = [
        "Air Quality",
        "Water Contamination",
        "Toxic Waste",
        "Noise",
        "Heat Island",
    ];
    let affected_communities = [
        "Low-Income",
        "Indigenous",
        "BIPOC",
        "Rural",
        "Urban",
        "Worker",
        "Global South",
    ];

    let gap_color = if regulatory_gap() > 50 {
        "var(--qualia-accent)"
    } else {
        "var(--qualia-accent)"
    };

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Environmental Justice" }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Justice Framework" }
                select {
                    value: "{justice_framework}", onchange: move |e| justice_framework.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in justice_frameworks { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Pollutant Burden" }
                select {
                    value: "{pollutant_burden}", onchange: move |e| pollutant_burden.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in pollutant_burdens { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Affected Community" }
                select {
                    value: "{affected_community}", onchange: move |e| affected_community.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in affected_communities { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Regulatory Gap: {regulatory_gap}" }
                input { r#type: "range", min: "0", max: "100", value: "{regulatory_gap}",
                    oninput: move |e| regulatory_gap.set(e.value().parse().unwrap_or(50)),
                    style: "width: 100%; margin-top: 4px;" }
                span { style: "font-size: 0.75rem; color: {gap_color};", "Gap level: {regulatory_gap}" }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Health Disparity: {health_disparity}" }
                input { r#type: "range", min: "0", max: "100", value: "{health_disparity}",
                    oninput: move |e| health_disparity.set(e.value().parse().unwrap_or(50)),
                    style: "width: 100%; margin-top: 4px;" }
            }
            div {
                style: "flex: 1; display: flex; flex-direction: column;",
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Notes" }
                textarea { value: "{notes}", oninput: move |e| notes.set(e.value()),
                    style: "flex: 1; width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: none; box-sizing: border-box; min-height: 60px;" }
            }
            div {
                style: "background: var(--qualia-bg); padding: 12px 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent);",
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{justice_framework} | {pollutant_burden} | {affected_community} | gap: {regulatory_gap} | health: {health_disparity}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → environmental justice engine | discourse sieve | anchor" }
            }
        }
    }
}
