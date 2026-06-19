use dioxus::prelude::*;

#[component]
pub fn IndigenousLanguageRevitalizationQapp() -> Element {
    let mut revitalization_strategy = use_signal(|| "Immersion School".to_string());
    let mut language_family = use_signal(|| "Algonquian".to_string());
    let mut vitality_status = use_signal(|| "Endangered".to_string());
    let mut speaker_count = use_signal(|| 1000u32);
    let mut intergenerational_transmission = use_signal(|| 30u32);
    let mut notes = use_signal(|| String::new());

    let strategies = [
        "Master-Apprentice",
        "Immersion School",
        "Language Nest",
        "Digital/Media",
        "Documentation Only",
        "Community Reclamation",
    ];
    let families = [
        "Algonquian",
        "Athabaskan",
        "Siouan",
        "Polynesian",
        "Papuan",
        "Australian",
        "Dravidian",
        "Other",
    ];
    let statuses = [
        "Critical",
        "Severely Endangered",
        "Endangered",
        "Vulnerable",
        "Safe",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",

            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Indigenous Language Revitalization" }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Revitalization Strategy" }
                select {
                    value: "{revitalization_strategy}",
                    onchange: move |e| revitalization_strategy.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in strategies { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Language Family" }
                select {
                    value: "{language_family}",
                    onchange: move |e| language_family.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in families { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Vitality Status" }
                select {
                    value: "{vitality_status}",
                    onchange: move |e| vitality_status.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in statuses { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Speaker Count: {speaker_count}" }
                input {
                    r#type: "range", min: "0", max: "100000",
                    value: "{speaker_count}",
                    oninput: move |e| speaker_count.set(e.value().parse().unwrap_or(1000)),
                    style: "width: 100%; margin-top: 4px;"
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Intergenerational Transmission: {intergenerational_transmission}%" }
                input {
                    r#type: "range", min: "0", max: "100",
                    value: "{intergenerational_transmission}",
                    oninput: move |e| intergenerational_transmission.set(e.value().parse().unwrap_or(30)),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{revitalization_strategy} | {language_family} | {vitality_status} | Speakers: {speaker_count} | Trans: {intergenerational_transmission}%" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → knowledge engine | sieve | anchor" }
            }
        }
    }
}
