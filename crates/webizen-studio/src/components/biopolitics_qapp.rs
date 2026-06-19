use dioxus::prelude::*;

#[component]
pub fn BiopoliticsQapp() -> Element {
    let mut theorist = use_signal(|| "Foucault".to_string());
    let mut concept = use_signal(|| "Biopower".to_string());
    let mut site = use_signal(|| "State".to_string());
    let mut subject_position = use_signal(|| "Citizen".to_string());
    let mut population = use_signal(|| 100000u32);
    let mut notes = use_signal(|| String::new());

    let theorists = [
        "Foucault",
        "Agamben",
        "Mbembe",
        "Esposito",
        "Hardt/Negri",
        "Butler",
    ];
    let concepts = [
        "Biopower",
        "Bare Life",
        "Necropolitics",
        "Biopolitical Community",
        "Immunisation",
        "Population Management",
    ];
    let sites = [
        "State",
        "Medicine",
        "Prison",
        "Border",
        "Military",
        "Digital Platform",
    ];
    let subject_positions = [
        "Citizen",
        "Refugee",
        "Detainee",
        "Patient",
        "Labourer",
        "Digital Subject",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Biopolitics" }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Theorist" }
                select {
                    value: "{theorist}", onchange: move |e| theorist.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in theorists { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Concept" }
                select {
                    value: "{concept}", onchange: move |e| concept.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in concepts { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Site" }
                select {
                    value: "{site}", onchange: move |e| site.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in sites { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Subject Position" }
                select {
                    value: "{subject_position}", onchange: move |e| subject_position.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in subject_positions { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Population: {population}" }
                input { r#type: "range", min: "0", max: "10000000", value: "{population}",
                    oninput: move |e| population.set(e.value().parse().unwrap_or(100000)),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{theorist} | {concept} | {site} | {subject_position} | pop: {population}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → biopolitics engine | discourse sieve | anchor" }
            }
        }
    }
}
