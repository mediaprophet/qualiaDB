use dioxus::prelude::*;

#[component]
pub fn BodyStudiesQapp() -> Element {
    let mut theoretical_framework = use_signal(|| "Phenomenology".to_string());
    let mut body_aspect = use_signal(|| "Embodiment".to_string());
    let mut methodology = use_signal(|| "Ethnographic".to_string());
    let mut cultural_context = use_signal(|| "Western".to_string());
    let mut notes = use_signal(|| String::new());

    let frameworks = [
        "Phenomenology",
        "Feminist Theory",
        "Disability Studies",
        "Critical Race Theory",
        "Performance Studies",
        "Medical Humanities",
    ];
    let aspects = [
        "Embodiment",
        "Medicalization",
        "Racialization",
        "Gendering",
        "Aging",
        "Disability",
        "Beauty/Normality",
    ];
    let methodologies = [
        "Ethnographic",
        "Autoethnographic",
        "Textual",
        "Visual",
        "Sociological",
    ];
    let contexts = ["Western", "Non-Western", "Global", "Subcultural"];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",

            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Body Studies" }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Theoretical Framework" }
                select {
                    value: "{theoretical_framework}",
                    onchange: move |e| theoretical_framework.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in frameworks { option { value: "{x}", "{x}" } }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Body Aspect" }
                select {
                    value: "{body_aspect}",
                    onchange: move |e| body_aspect.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in aspects { option { value: "{x}", "{x}" } }
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
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Cultural Context" }
                select {
                    value: "{cultural_context}",
                    onchange: move |e| cultural_context.set(e.value()),
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                    for x in contexts { option { value: "{x}", "{x}" } }
                }
            }

            div {
                style: "flex: 1; display: flex; flex-direction: column;",
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Notes" }
                textarea {
                    value: "{notes}",
                    oninput: move |e| notes.set(e.value()),
                    style: "flex: 1; width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: none; box-sizing: border-box; min-height: 80px;"
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 12px 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent);",
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{theoretical_framework} | {body_aspect} | {methodology} | {cultural_context}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → knowledge engine | sieve | anchor" }
            }
        }
    }
}
