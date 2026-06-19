use dioxus::prelude::*;

#[component]
pub fn IntellectualHistoryQapp() -> Element {
    let mut tradition = use_signal(|| "Analytic".to_string());
    let mut period = use_signal(|| "Enlightenment".to_string());
    let mut key_concept = use_signal(|| "Reason".to_string());
    let mut methodology = use_signal(|| "Textual Analysis".to_string());
    let mut influence = use_signal(|| 60u32);
    let mut notes = use_signal(|| String::new());

    let traditions = [
        "Analytic",
        "Continental",
        "Eastern",
        "Islamic",
        "Indigenous",
        "African",
        "Latin American",
    ];
    let periods = [
        "Ancient",
        "Medieval",
        "Early Modern",
        "Enlightenment",
        "19th Century",
        "20th Century",
        "Contemporary",
    ];
    let concepts = [
        "Reason", "Power", "Language", "Justice", "Nature", "Spirit", "Progress",
    ];
    let methodologies = [
        "Textual Analysis",
        "Contextual",
        "Reception History",
        "Conceptual",
        "Genealogical",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",

            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Intellectual History" }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Tradition" }
                select {
                    value: "{tradition}",
                    onchange: move |e| tradition.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in traditions { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Period" }
                select {
                    value: "{period}",
                    onchange: move |e| period.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in periods { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Key Concept" }
                select {
                    value: "{key_concept}",
                    onchange: move |e| key_concept.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in concepts { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Methodology" }
                select {
                    value: "{methodology}",
                    onchange: move |e| methodology.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in methodologies { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Influence: {influence}%" }
                input {
                    r#type: "range", min: "0", max: "100",
                    value: "{influence}",
                    oninput: move |e| influence.set(e.value().parse().unwrap_or(60)),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{tradition} | {period} | Concept: {key_concept} | {methodology} | Influence: {influence}%" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → knowledge engine | sieve | anchor" }
            }
        }
    }
}
