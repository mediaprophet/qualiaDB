use dioxus::prelude::*;

#[component]
pub fn TraumaStudiesQapp() -> Element {
    let mut trauma_type = use_signal(|| "Individual".to_string());
    let mut theoretical_model = use_signal(|| "Caruth Trauma Theory".to_string());
    let mut medium = use_signal(|| "Literature".to_string());
    let mut population = use_signal(|| "Survivors".to_string());
    let mut trauma_persistence = use_signal(|| 50u32);
    let mut healing_index = use_signal(|| 50u32);
    let mut notes = use_signal(|| String::new());

    let trauma_types = [
        "Individual",
        "Collective",
        "Historical",
        "Intergenerational",
        "Cultural",
        "Ecological",
        "Vicarious",
    ];
    let models = [
        "Caruth Trauma Theory",
        "Testimony",
        "Postmemory",
        "Resilience",
        "Somatic",
        "Political Trauma",
    ];
    let mediums = [
        "Literature",
        "Film",
        "Testimony",
        "Memoir",
        "Visual Art",
        "Performance",
    ];
    let populations = [
        "Survivors",
        "Descendants",
        "Bystanders",
        "Perpetrators",
        "Communities",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Trauma Studies" }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Trauma Type" }
                select {
                    value: "{trauma_type}",
                    onchange: move |e| trauma_type.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in trauma_types { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Theoretical Model" }
                select {
                    value: "{theoretical_model}",
                    onchange: move |e| theoretical_model.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in models { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Medium" }
                select {
                    value: "{medium}",
                    onchange: move |e| medium.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in mediums { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Population" }
                select {
                    value: "{population}",
                    onchange: move |e| population.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in populations { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Trauma Persistence: {trauma_persistence}" }
                input {
                    r#type: "range", min: "0", max: "100", value: "{trauma_persistence}",
                    oninput: move |e| trauma_persistence.set(e.value().parse().unwrap_or(50)),
                    style: "width: 100%; margin-top: 4px;"
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Healing Index: {healing_index}" }
                input {
                    r#type: "range", min: "0", max: "100", value: "{healing_index}",
                    oninput: move |e| healing_index.set(e.value().parse().unwrap_or(50)),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{trauma_type} | {theoretical_model} | {population} | Persistence: {trauma_persistence} | Healing: {healing_index}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → trauma studies engine | memory sieve | healing anchor" }
            }
        }
    }
}
