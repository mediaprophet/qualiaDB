use dioxus::prelude::*;

#[component]
pub fn LiteratureAndLawQapp() -> Element {
    let mut analytical_mode = use_signal(|| "Law in Literature".to_string());
    let mut genre = use_signal(|| "Trial Narrative".to_string());
    let mut legal_concept = use_signal(|| "Justice".to_string());
    let mut narrative_justice = use_signal(|| 50u32);
    let mut rhetorical_power = use_signal(|| 50u32);
    let mut notes = use_signal(|| String::new());

    let analytical_modes = [
        "Law in Literature",
        "Law as Literature",
        "Law and Narrative",
        "Legal Semiotics",
        "Rhetoric of Law",
        "Justice Aesthetics",
    ];
    let genres = [
        "Trial Narrative",
        "Legal Drama",
        "Prison Writing",
        "Human Rights Fiction",
        "Constitutional Text",
        "Courtroom Poetry",
    ];
    let legal_concepts = [
        "Justice",
        "Rights",
        "Punishment",
        "Sovereignty",
        "Property",
        "Testimony",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Literature and Law" }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Analytical Mode" }
                select {
                    value: "{analytical_mode}", onchange: move |e| analytical_mode.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in analytical_modes { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Genre" }
                select {
                    value: "{genre}", onchange: move |e| genre.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in genres { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Legal Concept" }
                select {
                    value: "{legal_concept}", onchange: move |e| legal_concept.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in legal_concepts { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Narrative Justice: {narrative_justice}" }
                input { r#type: "range", min: "0", max: "100", value: "{narrative_justice}",
                    oninput: move |e| narrative_justice.set(e.value().parse().unwrap_or(50)),
                    style: "width: 100%; margin-top: 4px;" }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Rhetorical Power: {rhetorical_power}" }
                input { r#type: "range", min: "0", max: "100", value: "{rhetorical_power}",
                    oninput: move |e| rhetorical_power.set(e.value().parse().unwrap_or(50)),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{analytical_mode} | {genre} | {legal_concept} | justice: {narrative_justice} | rhetoric: {rhetorical_power}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → literature and law engine | discourse sieve | anchor" }
            }
        }
    }
}
