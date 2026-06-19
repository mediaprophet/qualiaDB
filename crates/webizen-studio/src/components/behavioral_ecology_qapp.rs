use dioxus::prelude::*;

#[component]
pub fn BehavioralEcologyQapp() -> Element {
    let mut behaviour_type = use_signal(|| "Foraging".to_string());
    let mut evolutionary_explanation = use_signal(|| "Natural Selection".to_string());
    let mut study_species = use_signal(|| String::new());
    let mut habitat_type = use_signal(|| "Forest".to_string());
    let mut sample_n = use_signal(|| 50u32);
    let mut observation_method = use_signal(|| "Direct Observation".to_string());
    let mut notes = use_signal(|| String::new());

    let behaviour_types = [
        "Foraging",
        "Mating",
        "Parental Care",
        "Territorial",
        "Migratory",
        "Social",
        "Anti-Predator",
        "Communication",
        "Play",
        "Tool Use",
    ];
    let explanations = [
        "Natural Selection",
        "Sexual Selection",
        "Kin Selection",
        "Group Selection",
        "Reciprocal Altruism",
        "Game Theory ESS",
    ];
    let habitats = [
        "Forest",
        "Grassland",
        "Marine",
        "Desert",
        "Urban",
        "Arctic",
        "Coral Reef",
    ];
    let obs_methods = [
        "Direct Observation",
        "Camera Trap",
        "GPS Tracking",
        "Experiment",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Behavioral Ecology" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Behaviour Type" }
                    select {
                        value: "{behaviour_type}",
                        onchange: move |e| behaviour_type.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in behaviour_types { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Evolutionary Explanation" }
                    select {
                        value: "{evolutionary_explanation}",
                        onchange: move |e| evolutionary_explanation.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in explanations { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Habitat Type" }
                    select {
                        value: "{habitat_type}",
                        onchange: move |e| habitat_type.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in habitats { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Observation Method" }
                    select {
                        value: "{observation_method}",
                        onchange: move |e| observation_method.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in obs_methods { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Study Species" }
                input {
                    r#type: "text",
                    value: "{study_species}",
                    oninput: move |e| study_species.set(e.value()),
                    placeholder: "e.g. Apis mellifera, Parus major, Corvus corax",
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Sample N: {sample_n}" }
                input {
                    r#type: "range",
                    min: "5",
                    max: "500",
                    value: "{sample_n}",
                    oninput: move |e| sample_n.set(e.value().parse().unwrap_or(50)),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{behaviour_type} | {study_species} | {habitat_type} | n={sample_n} | {observation_method}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → ethology engine | evolutionary sieve | habitat anchor" }
            }
        }
    }
}
