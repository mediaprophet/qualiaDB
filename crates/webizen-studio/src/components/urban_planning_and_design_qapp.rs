use dioxus::prelude::*;

#[component]
pub fn UrbanPlanningAndDesignQapp() -> Element {
    let mut planning_model = use_signal(|| "Strategic".to_string());
    let mut land_use = use_signal(|| "Mixed-Use".to_string());
    let mut density = use_signal(|| "Medium 15–75".to_string());
    let mut transport_mode = use_signal(|| "Multi-Modal".to_string());
    let mut housing_affordability = use_signal(|| 50u32);
    let mut walkability = use_signal(|| 60u32);
    let mut notes = use_signal(|| String::new());

    let planning_models = [
        "Master Plan",
        "Strategic",
        "Participatory",
        "Smart City",
        "Transit-Oriented",
        "Incremental",
    ];
    let land_uses = [
        "Residential",
        "Mixed-Use",
        "Commercial",
        "Industrial",
        "Open Space",
        "Institutional",
    ];
    let densities = [
        "Low <15",
        "Medium 15–75",
        "High 75–200",
        "Very High 200+ dwellings/ha",
    ];
    let transport_modes = [
        "Walking",
        "Cycling",
        "Transit",
        "Car-Dependent",
        "Multi-Modal",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",

            h2 {
                style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;",
                "Urban Planning and Design"
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Planning Model" }
                select {
                    value: "{planning_model}",
                    onchange: move |e| planning_model.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in planning_models { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Land Use" }
                select {
                    value: "{land_use}",
                    onchange: move |e| land_use.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in land_uses { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Density" }
                select {
                    value: "{density}",
                    onchange: move |e| density.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in densities { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Transport Mode" }
                select {
                    value: "{transport_mode}",
                    onchange: move |e| transport_mode.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in transport_modes { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Housing Affordability Index: {housing_affordability}" }
                input {
                    r#type: "range", min: "0", max: "100",
                    value: "{housing_affordability}",
                    oninput: move |e| housing_affordability.set(e.value().parse().unwrap_or(50)),
                    style: "width: 100%; margin-top: 4px;"
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Walkability: {walkability}" }
                input {
                    r#type: "range", min: "0", max: "100",
                    value: "{walkability}",
                    oninput: move |e| walkability.set(e.value().parse().unwrap_or(60)),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{planning_model} | {land_use} | {density} | {transport_mode} | afford {housing_affordability} | walk {walkability}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → knowledge engine | sieve | anchor" }
            }
        }
    }
}
