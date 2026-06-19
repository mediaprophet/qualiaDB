use dioxus::prelude::*;

#[component]
pub fn EnvironmentalDesignQapp() -> Element {
    let mut design_domain = use_signal(|| "Interior".to_string());
    let mut sustainability_strategy = use_signal(|| "Biophilic Design".to_string());
    let mut user_experience = use_signal(|| "Comfort".to_string());
    let mut climate_zone = use_signal(|| "Temperate".to_string());
    let mut energy_performance = use_signal(|| 70u32);
    let mut notes = use_signal(|| String::new());

    let design_domains = [
        "Interior",
        "Landscape",
        "Urban",
        "Industrial",
        "Exhibition",
        "Wayfinding",
    ];
    let sustainability_strategies = [
        "Passive Solar",
        "Green Roof",
        "Rainwater Harvesting",
        "Biophilic Design",
        "Net Zero",
        "Circular Economy",
    ];
    let user_experiences = [
        "Comfort",
        "Accessibility",
        "Safety",
        "Aesthetics",
        "Wellbeing",
    ];
    let climate_zones = ["Tropical", "Arid", "Temperate", "Continental", "Polar"];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",

            h2 {
                style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;",
                "Environmental Design"
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Design Domain" }
                select {
                    value: "{design_domain}",
                    onchange: move |e| design_domain.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in design_domains { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Sustainability Strategy" }
                select {
                    value: "{sustainability_strategy}",
                    onchange: move |e| sustainability_strategy.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in sustainability_strategies { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "User Experience" }
                select {
                    value: "{user_experience}",
                    onchange: move |e| user_experience.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in user_experiences { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Climate Zone" }
                select {
                    value: "{climate_zone}",
                    onchange: move |e| climate_zone.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in climate_zones { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Energy Performance: {energy_performance}%" }
                input {
                    r#type: "range", min: "0", max: "100",
                    value: "{energy_performance}",
                    oninput: move |e| energy_performance.set(e.value().parse().unwrap_or(70)),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{design_domain} | {sustainability_strategy} | {user_experience} | {climate_zone} | energy {energy_performance}%" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → knowledge engine | sieve | anchor" }
            }
        }
    }
}
