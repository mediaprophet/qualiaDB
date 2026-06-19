use dioxus::prelude::*;

#[component]
pub fn PoststructuralismQapp() -> Element {
    let mut thinker = use_signal(|| "Derrida".to_string());
    let mut key_operation = use_signal(|| "Deconstruction".to_string());
    let mut target_structure = use_signal(|| "Language".to_string());
    let mut methodological_move = use_signal(|| "Différance".to_string());
    let mut radicality = use_signal(|| 50u32);
    let mut notes = use_signal(|| String::new());

    let thinkers = [
        "Derrida",
        "Foucault",
        "Deleuze",
        "Lacan",
        "Kristeva",
        "Cixous",
        "Lyotard",
        "Baudrillard",
    ];
    let operations = [
        "Deconstruction",
        "Genealogy",
        "Rhizome",
        "Desire",
        "Abjection",
        "Écriture Féminine",
        "Simulacrum",
    ];
    let structures = [
        "Language",
        "Power",
        "Subject",
        "Meaning",
        "History",
        "Sexuality",
        "Capital",
    ];
    let moves = [
        "Différance",
        "Archive",
        "Multiplicity",
        "Pleasure/Jouissance",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Poststructuralism" }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Thinker" }
                select {
                    value: "{thinker}",
                    onchange: move |e| thinker.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in thinkers { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Key Operation" }
                select {
                    value: "{key_operation}",
                    onchange: move |e| key_operation.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in operations { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Target Structure" }
                select {
                    value: "{target_structure}",
                    onchange: move |e| target_structure.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in structures { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Methodological Move" }
                select {
                    value: "{methodological_move}",
                    onchange: move |e| methodological_move.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in moves { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Radicality: {radicality}" }
                input {
                    r#type: "range", min: "0", max: "100", value: "{radicality}",
                    oninput: move |e| radicality.set(e.value().parse().unwrap_or(50)),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{thinker} | {key_operation} | {target_structure} | Radicality: {radicality}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → poststructuralist engine | différance sieve | structure anchor" }
            }
        }
    }
}
