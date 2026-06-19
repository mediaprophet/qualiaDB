use dioxus::prelude::*;

#[component]
pub fn RegionalismQapp() -> Element {
    let mut theoretical_tradition = use_signal(|| "Literary Regionalism".to_string());
    let mut region_type = use_signal(|| "Literary Region".to_string());
    let mut relationship_to_centre = use_signal(|| "Subordinate".to_string());
    let mut regional_identity = use_signal(|| 50u32);
    let mut economic_distinctiveness = use_signal(|| 50u32);
    let mut notes = use_signal(|| String::new());

    let traditions = [
        "Literary Regionalism",
        "Political Regionalism",
        "Economic Geography",
        "Cultural Geography",
        "Place-Based Theory",
    ];
    let region_types = [
        "Literary Region",
        "Political Region",
        "Economic Zone",
        "Cultural Heartland",
        "Transborder Region",
        "Periphery",
    ];
    let relationships = [
        "Subordinate",
        "Autonomous",
        "Separatist",
        "Federal",
        "Competitive",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Regionalism" }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Theoretical Tradition" }
                select {
                    value: "{theoretical_tradition}",
                    onchange: move |e| theoretical_tradition.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in traditions { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Region Type" }
                select {
                    value: "{region_type}",
                    onchange: move |e| region_type.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in region_types { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Relationship to Centre" }
                select {
                    value: "{relationship_to_centre}",
                    onchange: move |e| relationship_to_centre.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in relationships { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Regional Identity: {regional_identity}" }
                input {
                    r#type: "range", min: "0", max: "100", value: "{regional_identity}",
                    oninput: move |e| regional_identity.set(e.value().parse().unwrap_or(50)),
                    style: "width: 100%; margin-top: 4px;"
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Economic Distinctiveness: {economic_distinctiveness}" }
                input {
                    r#type: "range", min: "0", max: "100", value: "{economic_distinctiveness}",
                    oninput: move |e| economic_distinctiveness.set(e.value().parse().unwrap_or(50)),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{theoretical_tradition} | {region_type} | {relationship_to_centre} | Identity: {regional_identity}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → regionalism engine | place sieve | identity anchor" }
            }
        }
    }
}
