use dioxus::prelude::*;

#[component]
pub fn CulturalEcologyQapp() -> Element {
    let mut theoretical_tradition = use_signal(|| "Steward's Cultural Ecology".to_string());
    let mut environment_type = use_signal(|| "Tropical".to_string());
    let mut adaptation_strategy = use_signal(|| "Technological".to_string());
    let mut resource = use_signal(|| "Water".to_string());
    let mut carrying_capacity = use_signal(|| 10000u32);
    let mut notes = use_signal(|| String::new());

    let theoretical_traditions = [
        "Steward's Cultural Ecology",
        "Political Ecology",
        "Ethnoecology",
        "Symbolic Ecology",
        "Resilience Theory",
    ];
    let environment_types = [
        "Tropical",
        "Arid",
        "Temperate",
        "Alpine",
        "Coastal",
        "Arctic",
    ];
    let adaptation_strategies = [
        "Technological",
        "Social",
        "Cognitive",
        "Physiological",
        "Symbolic",
    ];
    let resources = ["Water", "Land", "Forest", "Wildlife", "Marine", "Energy"];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Cultural Ecology" }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Theoretical Tradition" }
                select {
                    value: "{theoretical_tradition}", onchange: move |e| theoretical_tradition.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in theoretical_traditions { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Environment Type" }
                select {
                    value: "{environment_type}", onchange: move |e| environment_type.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in environment_types { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Adaptation Strategy" }
                select {
                    value: "{adaptation_strategy}", onchange: move |e| adaptation_strategy.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in adaptation_strategies { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Resource" }
                select {
                    value: "{resource}", onchange: move |e| resource.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in resources { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Carrying Capacity: {carrying_capacity}" }
                input { r#type: "range", min: "0", max: "100000", value: "{carrying_capacity}",
                    oninput: move |e| carrying_capacity.set(e.value().parse().unwrap_or(10000)),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{theoretical_tradition} | {environment_type} | {adaptation_strategy} | {resource} | cap: {carrying_capacity}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → cultural ecology engine | discourse sieve | anchor" }
            }
        }
    }
}
