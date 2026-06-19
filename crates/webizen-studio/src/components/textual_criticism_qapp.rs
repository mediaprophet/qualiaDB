use dioxus::prelude::*;

#[component]
pub fn TextualCriticismQapp() -> Element {
    let mut tradition = use_signal(|| "Classical".to_string());
    let mut method = use_signal(|| "Stemmatic".to_string());
    let mut witness_type = use_signal(|| "Manuscript".to_string());
    let mut manuscript_count = use_signal(|| 20u32);
    let mut variant_density = use_signal(|| 0.3f64);
    let mut emendation_confidence = use_signal(|| 65u32);
    let mut notes = use_signal(|| String::new());

    let traditions = [
        "Classical",
        "Biblical",
        "Medieval",
        "Early Printed",
        "Modern Literary",
    ];
    let methods = [
        "Stemmatic",
        "Copy Text",
        "Best Text",
        "Eclectic",
        "Material Philology",
    ];
    let witnesses = ["Manuscript", "Early Print", "Later Print", "Digital"];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",

            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Textual Criticism" }

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
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Method" }
                select {
                    value: "{method}",
                    onchange: move |e| method.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in methods { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Witness Type" }
                select {
                    value: "{witness_type}",
                    onchange: move |e| witness_type.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in witnesses { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Manuscript Count: {manuscript_count}" }
                input {
                    r#type: "range", min: "0", max: "500",
                    value: "{manuscript_count}",
                    oninput: move |e| manuscript_count.set(e.value().parse().unwrap_or(20)),
                    style: "width: 100%; margin-top: 4px;"
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Variant Density: {variant_density:.2}" }
                input {
                    r#type: "range", min: "0", max: "100",
                    value: "{variant_density() * 100.0}",
                    oninput: move |e| variant_density.set(e.value().parse::<f64>().unwrap_or(30.0) / 100.0),
                    style: "width: 100%; margin-top: 4px;"
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Emendation Confidence: {emendation_confidence}%" }
                input {
                    r#type: "range", min: "0", max: "100",
                    value: "{emendation_confidence}",
                    oninput: move |e| emendation_confidence.set(e.value().parse().unwrap_or(65)),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{tradition} | {method} | {witness_type} | MSS: {manuscript_count} | Density: {variant_density:.2} | Conf: {emendation_confidence}%" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → knowledge engine | sieve | anchor" }
            }
        }
    }
}
