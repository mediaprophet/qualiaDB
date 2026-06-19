use dioxus::prelude::*;

#[component]
pub fn StructuralismQapp() -> Element {
    let mut discipline = use_signal(|| "Linguistics".to_string());
    let mut theorist = use_signal(|| "Saussure".to_string());
    let mut structural_element = use_signal(|| "Sign".to_string());
    let mut method = use_signal(|| "Synchronic Analysis".to_string());
    let mut system_closure = use_signal(|| 50u32);
    let mut notes = use_signal(|| String::new());

    let disciplines = [
        "Linguistics",
        "Anthropology",
        "Literary Theory",
        "Psychology",
        "Semiotics",
        "Marxist Structuralism",
    ];
    let theorists = [
        "Saussure",
        "Lévi-Strauss",
        "Barthes",
        "Althusser",
        "Lacan",
        "Jakobson",
        "Greimas",
    ];
    let elements = [
        "Sign",
        "Signifier/Signified",
        "Binary Opposition",
        "Paradigm/Syntagm",
        "Code",
        "Myth",
        "Actant",
    ];
    let methods = [
        "Synchronic Analysis",
        "Binary Analysis",
        "Transformational Grammar",
        "Actantial Model",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Structuralism" }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Discipline" }
                select {
                    value: "{discipline}",
                    onchange: move |e| discipline.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in disciplines { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Theorist" }
                select {
                    value: "{theorist}",
                    onchange: move |e| theorist.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in theorists { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Structural Element" }
                select {
                    value: "{structural_element}",
                    onchange: move |e| structural_element.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in elements { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Method" }
                select {
                    value: "{method}",
                    onchange: move |e| method.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in methods { option { value: "{x}", "{x}" } }
                }
            }
            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "System Closure: {system_closure}" }
                input {
                    r#type: "range", min: "0", max: "100", value: "{system_closure}",
                    oninput: move |e| system_closure.set(e.value().parse().unwrap_or(50)),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{theorist} | {discipline} | {structural_element} | Closure: {system_closure}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → structuralism engine | sign sieve | system anchor" }
            }
        }
    }
}
