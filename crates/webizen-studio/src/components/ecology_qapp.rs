use dioxus::prelude::*;

#[component]
pub fn EcologyQapp() -> Element {
    let mut ecosystem_type = use_signal(|| "Tropical Forest".to_string());
    let mut trophic_level = use_signal(|| "Primary Consumer".to_string());
    let mut population_n = use_signal(|| 1000u32);
    let mut carrying_capacity_k = use_signal(|| 5000u32);
    let mut interaction_type = use_signal(|| "Predation".to_string());
    let mut biodiversity_index = use_signal(|| 0.75f64);
    let mut field_notes = use_signal(|| String::new());

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Ecology QApp" }

            div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Ecosystem Type" }
                    select {
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        onchange: move |e| ecosystem_type.set(e.value()),
                        option { "Tropical Forest" }
                        option { "Temperate Forest" }
                        option { "Boreal" }
                        option { "Grassland" }
                        option { "Desert" }
                        option { "Wetland" }
                        option { "Marine" }
                        option { "Freshwater" }
                        option { "Arctic" }
                        option { "Coral Reef" }
                        option { "Savanna" }
                        option { "Chaparral" }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Trophic Level" }
                    select {
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        onchange: move |e| trophic_level.set(e.value()),
                        option { "Producer" }
                        option { "Primary Consumer" }
                        option { "Secondary Consumer" }
                        option { "Tertiary Consumer" }
                        option { "Decomposer" }
                        option { "Apex Predator" }
                        option { "Omnivore" }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Interaction Type" }
                    select {
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        onchange: move |e| interaction_type.set(e.value()),
                        option { "Predation" }
                        option { "Competition" }
                        option { "Mutualism" }
                        option { "Parasitism" }
                        option { "Commensalism" }
                        option { "Amensalism" }
                        option { "Herbivory" }
                        option { "Decomposition" }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Biodiversity Index (Shannon H): {biodiversity_index:.3}" }
                    input {
                        r#type: "range",
                        min: "0.0",
                        max: "1.0",
                        step: "0.001",
                        value: "{biodiversity_index}",
                        style: "width: 100%; margin-top: 8px;",
                        oninput: move |e| biodiversity_index.set(e.value().parse().unwrap_or(0.75)),
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Population N" }
                    input {
                        r#type: "number",
                        value: "{population_n}",
                        min: "1",
                        step: "10",
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        oninput: move |e| population_n.set(e.value().parse().unwrap_or(1000)),
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Carrying Capacity K: {carrying_capacity_k}" }
                    input {
                        r#type: "range",
                        min: "100",
                        max: "1000000",
                        step: "100",
                        value: "{carrying_capacity_k}",
                        style: "width: 100%; margin-top: 8px;",
                        oninput: move |e| carrying_capacity_k.set(e.value().parse().unwrap_or(5000)),
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Field Notes" }
                textarea {
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; min-height: 60px; box-sizing: border-box; resize: vertical;",
                    placeholder: "Enter field observations, sampling methodology, transect data...",
                    oninput: move |e| field_notes.set(e.value()),
                    "{field_notes}"
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent); flex: 1;",
                h3 { style: "margin-top: 0; color: var(--qualia-accent); font-size: 0.9rem;", "Output / Analysis" }
                div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 8px; font-size: 0.8rem;",
                    div { style: "color: var(--qualia-text-muted);", "Ecosystem:" }
                    div { style: "color: var(--qualia-text);", "{ecosystem_type}" }
                    div { style: "color: var(--qualia-text-muted);", "Trophic Level:" }
                    div { style: "color: var(--qualia-text);", "{trophic_level}" }
                    div { style: "color: var(--qualia-text-muted);", "Interaction:" }
                    div { style: "color: var(--qualia-text);", "{interaction_type}" }
                    div { style: "color: var(--qualia-text-muted);", "N / K:" }
                    div { style: "color: var(--qualia-text);", "{population_n} / {carrying_capacity_k}" }
                    div { style: "color: var(--qualia-text-muted);", "Biodiversity H:" }
                    div { style: "color: var(--qualia-text);", "{biodiversity_index:.3}" }
                }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 12px; border-top: 1px solid var(--qualia-border); padding-top: 8px;",
                    "QualiaDB → graph theory food-web | ODE Lotka-Volterra | ecological sieve"
                }
            }
        }
    }
}
