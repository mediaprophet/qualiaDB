use dioxus::prelude::*;

#[component]
pub fn SemioticsQapp() -> Element {
    let mut semiotic_tradition = use_signal(|| "Peircean".to_string());
    let mut sign_type = use_signal(|| "Symbol".to_string());
    let mut code_type = use_signal(|| "Linguistic".to_string());
    let mut sign_vehicle = use_signal(|| String::new());
    let mut object_or_referent = use_signal(|| String::new());
    let mut interpretant_notes = use_signal(|| String::new());
    let mut cultural_context = use_signal(|| String::new());

    let traditions = [
        "Saussurean",
        "Peircean",
        "Barthesian",
        "Greimas Structural",
        "Lotman Cultural",
        "Eco Unlimited Semiosis",
        "Biosemiotics",
        "Cognitive Semiotics",
    ];
    let sign_types = [
        "Symbol",
        "Icon",
        "Index (Peirce)",
        "Signifier-Signified (Saussure)",
    ];
    let code_types = [
        "Linguistic",
        "Visual",
        "Musical",
        "Bodily",
        "Spatial",
        "Digital",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Semiotics" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Semiotic Tradition" }
                    select {
                        value: "{semiotic_tradition}",
                        onchange: move |e| semiotic_tradition.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in traditions { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Sign Type" }
                    select {
                        value: "{sign_type}",
                        onchange: move |e| sign_type.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in sign_types { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Code Type" }
                    select {
                        value: "{code_type}",
                        onchange: move |e| code_type.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in code_types { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Sign Vehicle (Representamen)" }
                    input {
                        r#type: "text",
                        value: "{sign_vehicle}",
                        oninput: move |e| sign_vehicle.set(e.value()),
                        placeholder: "e.g. red traffic light, dollar sign, dove",
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Object / Referent" }
                    input {
                        r#type: "text",
                        value: "{object_or_referent}",
                        oninput: move |e| object_or_referent.set(e.value()),
                        placeholder: "e.g. stop, money, peace",
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Cultural Context" }
                    input {
                        r#type: "text",
                        value: "{cultural_context}",
                        oninput: move |e| cultural_context.set(e.value()),
                        placeholder: "e.g. Western urban, Japanese corporate, Islamic",
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
            }

            div {
                style: "flex: 1; display: flex; flex-direction: column;",
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Interpretant Notes" }
                textarea {
                    value: "{interpretant_notes}",
                    oninput: move |e| interpretant_notes.set(e.value()),
                    placeholder: "Describe the interpretant — what the sign produces in the mind of the interpreter...",
                    style: "flex: 1; width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: none; box-sizing: border-box; min-height: 80px;"
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 12px 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent);",
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{semiotic_tradition} | {sign_type} | {code_type} | {sign_vehicle} → {object_or_referent}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → semiotic analysis engine | sign relation sieve | cultural code anchor" }
            }
        }
    }
}
