use dioxus::prelude::*;

#[component]
pub fn QueerTheoryQapp() -> Element {
    let mut theoretical_orientation = use_signal(|| "Butlerian Performativity".to_string());
    let mut key_theorist = use_signal(|| "Butler".to_string());
    let mut site = use_signal(|| "Identity".to_string());
    let mut subversion_strategy = use_signal(|| "Parody".to_string());
    let mut normativity_critique = use_signal(|| 50u32);
    let mut notes = use_signal(|| String::new());

    let orientations = [
        "Butlerian Performativity",
        "Anti-Normativity",
        "Queer Temporality",
        "Queer of Color Critique",
        "Crip Queer",
        "Trans Studies",
    ];
    let theorists = [
        "Butler",
        "Sedgwick",
        "Muñoz",
        "Puar",
        "Halberstam",
        "Crenshaw",
        "Spade",
    ];
    let sites = [
        "Identity",
        "Body",
        "Temporality",
        "Space",
        "Law",
        "Family",
        "Affect",
    ];
    let strategies = [
        "Parody",
        "Camp",
        "Failure",
        "Futurity",
        "Coalition",
        "Refusal",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Queer Theory" }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Theoretical Orientation" }
                select {
                    value: "{theoretical_orientation}",
                    onchange: move |e| theoretical_orientation.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in orientations { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Key Theorist" }
                select {
                    value: "{key_theorist}",
                    onchange: move |e| key_theorist.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in theorists { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Site" }
                select {
                    value: "{site}",
                    onchange: move |e| site.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in sites { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Subversion Strategy" }
                select {
                    value: "{subversion_strategy}",
                    onchange: move |e| subversion_strategy.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in strategies { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Normativity Critique: {normativity_critique}" }
                input {
                    r#type: "range", min: "0", max: "100", value: "{normativity_critique}",
                    oninput: move |e| normativity_critique.set(e.value().parse().unwrap_or(50)),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{key_theorist} | {theoretical_orientation} | {subversion_strategy} | Critique: {normativity_critique}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → queer theory engine | performativity sieve | normativity anchor" }
            }
        }
    }
}
